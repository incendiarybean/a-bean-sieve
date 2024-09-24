use colored::Colorize;
use http_body_util::Full;
use hyper::{
    body::Bytes, http, server::conn::http1, service::service_fn, upgrade::Upgraded, Method,
    Request, Response,
};
use hyper_util::{rt::TokioIo, server::graceful};
use std::{
    convert::Infallible,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

#[derive(Debug)]
pub enum ProxyEvent {
    Running,
    Stopped,
    Error,
    Terminating,
    Terminated,
    RequestEvent((String, String, bool)),
    Blocking(Vec<String>),
    SwitchList(Vec<String>),
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct ProxyExclusionRow {
    pub updating: bool,
    pub row_index: usize,
    pub row_value: String,
}

impl Default for ProxyExclusionRow {
    fn default() -> Self {
        Self {
            updating: false,
            row_index: 0,
            row_value: String::new(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct Proxy {
    pub port: String,
    pub port_error: String,
    pub logs: bool,
    pub allow_list: Vec<String>,
    pub block_list: Vec<String>,
    pub start_enabled: bool,
    pub allow_blocking: Arc<Mutex<bool>>,
    pub allow_requests_by_default: Arc<Mutex<bool>>,
    pub current_list: Arc<Mutex<Vec<String>>>,

    // Different value selectors for exclusion management
    pub dragging_value: String,
    pub selected_value: String,
    pub selected_exclusion_row: ProxyExclusionRow,

    // Skip these as Default values are fine
    #[serde(skip)]
    pub event: std::sync::mpsc::Sender<ProxyEvent>,
    #[serde(skip)]
    pub status: Arc<Mutex<ProxyEvent>>,
    #[serde(skip)]
    pub requests: Arc<Mutex<Vec<(String, String, bool)>>>,
}

impl Default for Proxy {
    fn default() -> Self {
        let (event_sender, event_receiver) = std::sync::mpsc::channel::<ProxyEvent>();

        // Need a value, and a shareable value to update the original reference
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let status_clone = Arc::clone(&status);

        let requests = Arc::new(Mutex::new(Vec::<(String, String, bool)>::new()));
        let requests_clone = Arc::clone(&requests);

        let allow_blocking = Arc::new(Mutex::new(false));
        let allow_blocking_clone = Arc::clone(&allow_blocking);

        let allow_requests_by_default = Arc::new(Mutex::new(false));
        let allow_requests_by_default_clone = Arc::clone(&allow_requests_by_default);

        let current_list = Arc::new(Mutex::new(Vec::<String>::new()));
        let current_list_clone = Arc::clone(&current_list);

        thread::spawn(move || loop {
            // Sleep loop to loosen CPU stress
            thread::sleep(Duration::from_millis(100));

            // Check incoming Proxy events
            match event_receiver.recv() {
                Ok(event) => match event {
                    ProxyEvent::Terminated | ProxyEvent::Stopped => {
                        let mut status = status_clone.lock().unwrap();
                        *status = ProxyEvent::Stopped;
                    }
                    ProxyEvent::RequestEvent((method, uri, blocked)) => {
                        // We need to have a --no-gui option to enable this
                        // println!(
                        //     "{} {} {}",
                        //     "REQUEST:".green(),
                        //     uri,
                        //     if blocked {
                        //         "-> BLOCKED".red()
                        //     } else {
                        //         "-> ALLOWED".green()
                        //     }
                        // );

                        let mut status = requests_clone.lock().unwrap();
                        status.push((method, uri, blocked));
                    }
                    ProxyEvent::Blocking(default_list) => {
                        let mut blocking = allow_blocking_clone.lock().unwrap();
                        *blocking = !*blocking;

                        let mut current_list = current_list_clone.lock().unwrap();
                        *current_list = default_list;
                    }
                    ProxyEvent::SwitchList(exclusion_list) => {
                        // When updating the list, toggle the traffic control and update the list
                        let mut allow_requests_by_default =
                            allow_requests_by_default_clone.lock().unwrap();
                        *allow_requests_by_default = !*allow_requests_by_default;

                        let mut current_list = current_list_clone.lock().unwrap();
                        *current_list = exclusion_list;
                    }
                    _ => {
                        // If there is no custom event handler, simply set the value of status to this event type
                        let mut status = status_clone.lock().unwrap();
                        *status = event;
                    }
                },
                Err(_) => {
                    let mut status = status_clone.lock().unwrap();
                    *status = ProxyEvent::Error
                }
            }
        });

        Self {
            port: String::new(),
            port_error: String::default(),
            start_enabled: true,
            event: event_sender.clone(),
            dragging_value: String::new(),
            selected_value: String::new(),
            selected_exclusion_row: ProxyExclusionRow::default(),
            status,
            logs: false,
            requests,
            allow_blocking,
            allow_requests_by_default,
            allow_list: Vec::<String>::new(),
            block_list: Vec::<String>::new(),
            current_list,
        }
    }
}

impl Proxy {
    /// Restores previous state and returns the Proxy
    ///
    /// # Arguments
    /// * `previous_port` - A String that contains the previous port
    /// * `previous_port_error` - A String that contains the previous port error (if there was one)
    /// * `show_logs` - A bool that contains whether the logs were showing or not
    /// * `previous_allow_list` - A list of strings containing the previous allow_list
    /// * `previous_block_list` - A list of strings containing the previous block_list
    /// * `previously_blocking` - A bool that contains whether blocking was previously enabled
    /// * `previously_allowing_requests_by_default` - A bool containing whether the deny or allow list was previously active
    pub fn restore_previous(
        mut self,
        previous_port: String,
        previous_port_error: String,
        show_logs: bool,
        previous_allow_list: Vec<String>,
        previous_block_list: Vec<String>,
        previously_blocking: bool,
        previously_allowing_requests_by_default: bool,
    ) -> Self {
        self.port = previous_port;
        self.port_error = previous_port_error;
        self.logs = show_logs;
        self.allow_list = previous_allow_list;
        self.block_list = previous_block_list;

        if previously_blocking {
            self.enable_exclusion();
        }

        if previously_allowing_requests_by_default {
            self.switch_exclusion();
        }

        self
    }

    /// Handles termination of the service
    ///
    /// # Arguments
    /// * `event` - The event sender to write current state
    /// * `status` - The current ProxyEvent status
    async fn handle_termination(
        event: std::sync::mpsc::Sender<ProxyEvent>,
        status: Arc<Mutex<ProxyEvent>>,
    ) -> oneshot::Receiver<()> {
        let (shutdown_sig, shutdown_rec) = tokio::sync::oneshot::channel::<()>();

        loop {
            // We don't care about waiting a second, as long as it keeps CPU usage down
            thread::sleep(Duration::from_millis(1000));

            // Get Proxy's state
            let status = match status.lock() {
                Ok(status) => status,
                Err(poisoned) => poisoned.into_inner(),
            };

            match *status {
                ProxyEvent::Terminating => {
                    match shutdown_sig.send(()) {
                        Ok(_) => println!("{}", "Terminating Service.".red()),
                        Err(error) => println!("{:?}", error),
                    };
                    break;
                }
                _ => (),
            };
        }

        // Send event to show it's Terminated/Stopped
        event.send(ProxyEvent::Terminated).unwrap();
        println!("{}", "Terminated Service.".red());

        shutdown_rec
    }

    #[tokio::main]
    pub async fn proxy_service(self) -> io::Result<()> {
        let addr = SocketAddr::from((
            [127, 0, 0, 1],
            self.port.trim().parse::<u16>().unwrap().clone(),
        ));

        let event_clone = self.event.clone();

        let graceful = graceful::GracefulShutdown::new();

        let mut signal = std::pin::pin!(Self::handle_termination(event_clone, self.status.clone()));

        let http = http1::Builder::new();

        let listener = TcpListener::bind(addr).await;

        match listener {
            Ok(listener) => {
                self.event.send(ProxyEvent::Running).unwrap();
                let event_sender = self.event.clone();

                loop {
                    tokio::select! {
                        Ok((stream, _addr)) = listener.accept() => {
                            let io = TokioIo::new(stream);

                            let internal_event_sender = event_sender.clone();
                            let error_event_sender = internal_event_sender.clone();

                            let is_blocking = match self.allow_blocking.lock() {
                                Ok(is_blocking) => *is_blocking,
                                Err(poisoned) => *poisoned.into_inner(),
                            };

                            let allow_by_default = match self.allow_requests_by_default.lock() {
                                Ok(allow_by_default) => *allow_by_default,
                                Err(poisoned) => *poisoned.into_inner(),
                            };

                            let configured_list = match self.current_list.lock() {
                                Ok(current_list) => current_list,
                                Err(poisoned) => poisoned.into_inner(),
                            };

                            let configured_list = configured_list.clone();

                            let connection = http.serve_connection(io, service_fn(move |request| {
                                Self::request(request, internal_event_sender.clone(), is_blocking, allow_by_default, configured_list.clone())
                            }));

                            // watch this connection
                            let connection_monitor = graceful.watch(connection);
                            tokio::spawn(async move {
                                if let Err(error) = connection_monitor.await {
                                    error_event_sender.send(ProxyEvent::Error).unwrap();
                                }
                            });
                        },

                        _ = &mut signal => break
                    }
                }
            }
            Err(_) => self.event.send(ProxyEvent::Error).unwrap(),
        }

        Ok(())
    }

    pub async fn request(
        request: Request<impl hyper::body::Body>,
        event: std::sync::mpsc::Sender<ProxyEvent>,
        is_blocking: bool,
        allow_by_default: bool,
        blocking_list: Vec<String>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        // I'll need to do an Arc<Mutex<Bool>> for watching whether the blocking is enabled
        // Check if address is within blocked list, send FORBIDDEN response on bad request
        let is_excluded_address =
            Self::is_excluded_address(blocking_list, request.uri().to_string());

        let logger = (
            request.method().to_string(),
            request.uri().to_string(),
            is_blocking
                && ((is_excluded_address && allow_by_default)
                    || (!is_excluded_address & !allow_by_default)),
        );

        event.send(ProxyEvent::RequestEvent(logger)).unwrap();

        if is_blocking {
            // If we're allowing traffic but it's in the exception list - block it
            // If we're blocking traffic but it's in the exception list - block it
            if (is_excluded_address && allow_by_default)
                || (!is_excluded_address & !allow_by_default)
            {
                let mut resp = Response::new(Full::from(Bytes::from("Oopsie Whoopsie!")));
                *resp.status_mut() = http::StatusCode::FORBIDDEN;
                return Ok(resp);
            }
        }

        // Forward the rest of accepted requests
        if Method::CONNECT == request.method() {
            if let Some(addr) = Self::host_addr(request.uri()) {
                match hyper::upgrade::on(request).await {
                    Ok(upgraded) => {
                        if let Err(_) = Self::tunnel(upgraded, addr).await {
                            // This error mostly indicates an external host closed a connection
                            // Don't need to worry about that
                        };
                    }
                    Err(e) => println!("upgrade error: {}", e.to_string().red()),
                }

                return Ok(Response::new(Full::from(Bytes::from(""))));
            } else {
                let mut resp = Response::new(Full::from(Bytes::from(
                    "CONNECT must be to a socket address",
                )));
                *resp.status_mut() = http::StatusCode::BAD_REQUEST;

                return Ok(resp);
            }
        } else {
            // return client.request(request).await;
            let mut resp = Response::new(Full::from(Bytes::from(
                "CONNECT must be to a socket address",
            )));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;

            return Ok(resp);
        }
    }

    fn host_addr(uri: &http::Uri) -> Option<String> {
        uri.authority().and_then(|auth| Some(auth.to_string()))
    }

    async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
        let mut server = TcpStream::connect(addr).await?;
        let mut updgraded_connection = TokioIo::new(upgraded);
        tokio::io::copy_bidirectional(&mut updgraded_connection, &mut server).await?;
        Ok(())
    }

    pub fn get_status(&mut self) -> String {
        let proxy_state = match self.status.lock() {
            Ok(proxy_event) => proxy_event,
            Err(poisoned) => poisoned.into_inner(),
        };

        let current_proxy_status = match *proxy_state {
            ProxyEvent::Running => "RUNNING",
            ProxyEvent::Stopped => "STOPPED",
            ProxyEvent::Error => "ERROR",
            ProxyEvent::Terminating => "TERMINATING",
            ProxyEvent::Terminated => "TERMINATED",
            _ => "NOT COVERED",
        };

        current_proxy_status.to_string()
    }

    /// Returns the Proxy's current exclusion list
    pub fn get_current_list(&mut self) -> Vec<String> {
        let current_list = match self.current_list.lock() {
            Ok(current_list) => current_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        current_list.clone()
    }

    /// Returns the Proxy's current blocking status
    pub fn get_blocking_status(&mut self) -> (bool, bool) {
        let blocking_status = match self.allow_blocking.lock() {
            Ok(blocking_status) => *blocking_status,
            Err(poisoned) => *poisoned.into_inner(),
        };

        let allowing_all_traffic = match self.allow_requests_by_default.lock() {
            Ok(allowing_all_traffic) => *allowing_all_traffic,
            Err(poisoned) => *poisoned.into_inner(),
        };

        (blocking_status, allowing_all_traffic)
    }

    /// Returns the Proxy's recent requests
    pub fn get_requests(&mut self) -> Vec<(String, String, bool)> {
        let requests_list = match self.requests.lock() {
            Ok(requests_list) => requests_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests_list.to_vec()
    }

    /// Sets whether the Proxy is using an exclusion list
    pub fn enable_exclusion(&mut self) {
        let (_blocking_status, allowing_all_traffic) = self.get_blocking_status();

        self.event
            .send(ProxyEvent::Blocking(if allowing_all_traffic {
                self.block_list.clone()
            } else {
                self.allow_list.clone()
            }))
            .unwrap();
    }

    /// Sets which exclusion list the Proxy is using
    pub fn switch_exclusion(&mut self) {
        let allowing_all_traffic = match self.allow_requests_by_default.lock() {
            Ok(allowing_all_traffic) => allowing_all_traffic,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.event
            .send(ProxyEvent::SwitchList(if *allowing_all_traffic {
                self.allow_list.clone()
            } else {
                self.block_list.clone()
            }))
            .unwrap();
    }

    /// Update the Proxy's exclusion list
    pub fn add_exclusion(&mut self) {
        let mut list = self.get_current_list();
        let is_already_excluded =
            Self::is_excluded_address(list.clone(), self.dragging_value.clone());

        if !is_already_excluded {
            list.push(self.dragging_value.clone());
        } else {
            list.retain(|x| x.clone() != self.dragging_value.clone());
        }

        let (_is_blocking, allowing_all_traffic) = self.get_blocking_status();

        if allowing_all_traffic {
            self.block_list = list.clone();
        } else {
            self.allow_list = list.clone();
        }

        let mut current_list_mut = self.current_list.lock().unwrap();
        *current_list_mut = list;
    }

    /// Update a single value in the Proxy's exclusion list
    pub fn update_exclusion_list_value(&mut self, uri: String) {
        // TODO: This could be combined with the add_exclusion (rename to update_exclusion)
        let mut list = self.get_current_list();

        // Find index as cannot index by String
        let uri_index = list.iter().position(|item| item == &uri).unwrap();

        // Overwrite value in current_list
        list[uri_index] = self.selected_exclusion_row.row_value.clone();

        // Update allow/deny lists
        let allowing_all_traffic = match self.allow_requests_by_default.lock() {
            Ok(allowing_all_traffic) => *allowing_all_traffic,
            Err(poisoned) => *poisoned.into_inner(),
        };
        if allowing_all_traffic {
            self.block_list = list.clone();
        } else {
            self.allow_list = list.clone();
        }

        // Update current list values
        let mut current_list_mut = self.current_list.lock().unwrap();
        *current_list_mut = list;

        // Reset edit values
        self.selected_exclusion_row = ProxyExclusionRow::default();
    }

    /// Handles termination of the service
    ///
    /// # Arguments
    /// * `exclusion_list` - a Vec of String to compare the Uri to
    /// * `uri` - A String to check if it's in the list
    pub fn is_excluded_address(exclusion_list: Vec<String>, uri: String) -> bool {
        if exclusion_list
            .iter()
            .any(|item| uri.contains(item) || item.contains(&uri))
        {
            true
        } else {
            false
        }
    }
}
