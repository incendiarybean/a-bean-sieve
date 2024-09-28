use colored::Colorize;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    body::Bytes, http, server::conn::http1, service::service_fn, upgrade::Upgraded, Method,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use std::{
    fmt::Debug,
    io,
    net::SocketAddr,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::net::{TcpListener, TcpStream};

use super::traffic_filter::TrafficFilter;

#[derive(Debug, PartialEq, Clone)]
pub enum ProxyEvent {
    Running,
    Stopped,
    Error(String),
    Terminating,
    Terminated,
    RequestEvent((String, String, bool)),
    Blocking(Vec<String>),
    SwitchList(Vec<String>),
}

impl std::string::ToString for ProxyEvent {
    fn to_string(&self) -> String {
        let current_proxy_status = match self {
            ProxyEvent::Running => String::from("RUNNING"),
            ProxyEvent::Stopped => String::from("STOPPED"),
            ProxyEvent::Error(_) => String::from("ERROR"),
            ProxyEvent::Terminating => String::from("TERMINATING"),
            ProxyEvent::Terminated => String::from("TERMINATED"),
            _ => String::from("UNKNOWN"),
        };

        current_proxy_status
    }
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

#[derive(serde::Serialize, Clone)]
pub struct ProxyRequestLog {
    pub method: String,
    pub request: String,
    pub blocked: bool,
}

#[derive(serde::Serialize)]
pub struct ProxyExclusionList {
    pub request: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct Proxy {
    // Startup related items
    pub port: String,
    pub port_error: String,
    pub start_enabled: bool,

    // Logs
    pub logs: bool,

    pub traffic_filter: Arc<Mutex<TrafficFilter>>,

    // pub traffic_filtering: Arc<Mutex<bool>>,
    // pub traffic_filtering_type: Arc<Mutex<TrafficFilterType>>
    // pub allow_list: Vec<String>,
    // pub block_list: Vec<String>,
    // pub allow_blocking: Arc<Mutex<bool>>,
    // pub allow_requests_by_default: Arc<Mutex<bool>>,
    // pub current_list: Arc<Mutex<Vec<String>>>,

    // Different value selectors for exclusion management
    // pub dragging_value: String,
    pub selected_value: String,
    pub selected_exclusion_row: ProxyExclusionRow,

    // Skip these as Default values are fine
    #[serde(skip)]
    pub run_time: Arc<Mutex<Option<std::time::Instant>>>,
    #[serde(skip)]
    pub event: std::sync::mpsc::Sender<ProxyEvent>,
    #[serde(skip)]
    pub status: Arc<Mutex<ProxyEvent>>,
    #[serde(skip)]
    pub requests: Arc<Mutex<Vec<ProxyRequestLog>>>,
}

impl Default for Proxy {
    fn default() -> Self {
        let (event_sender, event_receiver) = std::sync::mpsc::channel::<ProxyEvent>();
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let requests = Arc::new(Mutex::new(Vec::<ProxyRequestLog>::new()));
        let run_time = Arc::new(Mutex::new(None));
        let traffic_filter = Arc::new(Mutex::new(TrafficFilter::default()));

        // Run the event handler
        Self::event_handler(
            event_receiver,
            status.clone(),
            requests.clone(),
            run_time.clone(),
            traffic_filter.clone(),
        );

        Self {
            port: String::new(),
            port_error: String::default(),
            start_enabled: true,
            event: event_sender.clone(),
            selected_value: String::new(),
            selected_exclusion_row: ProxyExclusionRow::default(),
            status,
            logs: false,
            requests,
            run_time,
            traffic_filter,
        }
    }
}

impl Proxy {
    /// Creates a new Proxy from given values
    ///
    /// # Arguments
    /// * `port` - A String that contains the port
    /// * `show_logs` - A bool that contains whether the logs are showing or not
    /// * `traffic_filter` - A TrafficFilter containing the applied filters
    pub fn new(port: String, show_logs: bool, traffic_filter: Arc<Mutex<TrafficFilter>>) -> Self {
        let (event_sender, event_receiver) = std::sync::mpsc::channel::<ProxyEvent>();

        // Need a value, and a shareable value to update the original reference
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));

        let requests = Arc::new(Mutex::new(Vec::<ProxyRequestLog>::new()));

        let run_time = Arc::new(Mutex::new(None));

        Self::event_handler(
            event_receiver,
            status.clone(),
            requests.clone(),
            run_time.clone(),
            traffic_filter.clone(),
        );

        Self {
            port,
            port_error: String::default(),
            start_enabled: true,
            event: event_sender.clone(),
            selected_value: String::new(),
            selected_exclusion_row: ProxyExclusionRow::default(),
            status,
            logs: show_logs,
            requests,
            run_time,
            traffic_filter,
        }
    }

    fn event_handler(
        event_receiver: Receiver<ProxyEvent>,
        status: Arc<Mutex<ProxyEvent>>,
        requests: Arc<Mutex<Vec<ProxyRequestLog>>>,
        run_time: Arc<Mutex<Option<std::time::Instant>>>,
        traffic_filter: Arc<Mutex<TrafficFilter>>,
    ) {
        let requests_clone: Arc<Mutex<Vec<ProxyRequestLog>>> = Arc::clone(&requests);
        let status_clone = Arc::clone(&status);
        let run_time_clone = Arc::clone(&run_time);
        let traffic_filter_clone = Arc::clone(&traffic_filter);

        thread::spawn(move || {
            loop {
                // Sleep loop to loosen CPU stress
                thread::sleep(Duration::from_millis(100));

                // Check incoming Proxy events
                match event_receiver.recv() {
                    Ok(event) => match event {
                        ProxyEvent::Terminated | ProxyEvent::Stopped => {
                            let mut status = status_clone.lock().unwrap();
                            *status = ProxyEvent::Stopped;

                            let mut run_time = run_time_clone.lock().unwrap();
                            *run_time = None;
                        }
                        ProxyEvent::RequestEvent((method, request, blocked)) => {
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

                            let mut requests_list = requests_clone.lock().unwrap();
                            requests_list.push(ProxyRequestLog {
                                method,
                                request,
                                blocked,
                            });
                        }
                        ProxyEvent::Blocking(list) => {
                            let mut traffic_filter = traffic_filter_clone.lock().unwrap();
                            let enabled = traffic_filter.get_enabled();

                            traffic_filter.set_enabled(!enabled);
                            traffic_filter.set_filter_list(list);

                            *traffic_filter = traffic_filter.clone();
                        }
                        ProxyEvent::SwitchList(exclusion_list) => {
                            // When updating the list, toggle the traffic control and update the list
                            // let mut allow_requests_by_default =
                            //     allow_requests_by_default_clone.lock().unwrap();
                            // *allow_requests_by_default = !*allow_requests_by_default;

                            // let mut current_list = current_list_clone.lock().unwrap();
                            // *current_list = exclusion_list;
                        }
                        ProxyEvent::Error(message) => {
                            println!("{}", message.red());

                            let mut status = status_clone.lock().unwrap();
                            *status = ProxyEvent::Error(message);
                        }
                        ProxyEvent::Running => {
                            println!("{}", "Running service...".green());

                            let mut run_time = run_time_clone.lock().unwrap();
                            *run_time = Some(std::time::Instant::now());

                            let mut status = status_clone.lock().unwrap();
                            *status = event;
                        }
                        _ => {
                            // If there is no custom event handler, simply set the value of status to this event type
                            let mut status = status_clone.lock().unwrap();
                            *status = event;
                        }
                    },
                    Err(message) => {
                        let mut status = status_clone.lock().unwrap();
                        *status = ProxyEvent::Error(message.to_string())
                    }
                }
            }
        });
    }

    #[tokio::main]
    pub async fn proxy_service(self) -> io::Result<()> {
        let addr = SocketAddr::from((
            [127, 0, 0, 1],
            self.port.trim().parse::<u16>().unwrap().clone(),
        ));

        let event_clone = self.event.clone();

        let mut signal = std::pin::pin!(Self::handle_termination(event_clone, self.status.clone()));

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

                            let traffic_filter = self.traffic_filter.lock().unwrap().clone();

                            // let is_blocking = match self.allow_blocking.lock() {
                            //     Ok(is_blocking) => *is_blocking,
                            //     Err(poisoned) => *poisoned.into_inner(),
                            // };

                            // let allow_by_default = match self.allow_requests_by_default.lock() {
                            //     Ok(allow_by_default) => *allow_by_default,
                            //     Err(poisoned) => *poisoned.into_inner(),
                            // };

                            // let configured_list = match self.current_list.lock() {
                            //     Ok(current_list) => current_list,
                            //     Err(poisoned) => poisoned.into_inner(),
                            // };

                            // let configured_list = traffic_filter.get_filter_list().clone();

                            let connection = http1::Builder::new()
                            .preserve_header_case(true)
                            .title_case_headers(true)
                            .serve_connection(io, service_fn( move |request|
                                Self::request(
                                    request,
                                    internal_event_sender.clone(),
                                    traffic_filter.clone()
                                )))
                            .with_upgrades();

                            tokio::task::spawn(async move {
                                let _ = connection.await;
                            });
                        },

                        _ = &mut signal => break
                    }
                }
            }
            Err(message) => self
                .event
                .send(ProxyEvent::Error(message.to_string()))
                .unwrap(),
        }

        Ok(())
    }

    /// Handles termination of the service
    ///
    /// # Arguments
    /// * `event` - The event sender to write current state
    /// * `status` - The current ProxyEvent status
    async fn handle_termination(
        event: std::sync::mpsc::Sender<ProxyEvent>,
        status: Arc<Mutex<ProxyEvent>>,
    ) {
        let (shutdown_sig, shutdown_rec) = tokio::sync::oneshot::channel::<()>();

        std::thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(1000));

            // Get Proxy's state
            let status = match status.lock() {
                Ok(status) => status,
                Err(poisoned) => poisoned.into_inner(),
            };

            match *status {
                ProxyEvent::Terminating => {
                    let _ = shutdown_sig.send(());
                    break;
                }
                _ => (),
            };
        });

        match shutdown_rec.await {
            Ok(_) => {
                // Send event to show it's Terminated/Stopped
                event.send(ProxyEvent::Terminated).unwrap();
                println!("{}", "Terminated Service.".red());
            }
            Err(_) => {}
        }
    }

    /// Handles the proxy request
    async fn request(
        request: Request<hyper::body::Incoming>,
        event: std::sync::mpsc::Sender<ProxyEvent>,
        _traffic_filter: TrafficFilter,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        // let traffic_filter_enabled = traffic_filter.get_enabled();
        // let traffic_filter_type = traffic_filter.get_filter();
        // let exclusion_list = traffic_filter.get_filter_list();

        // let request_uri = request.uri().to_string();
        // let is_excluded_address = exclusion_list
        //     .iter()
        //     .any(|item| request_uri.contains(item) || item.contains(&request_uri));

        let logger = (
            request.method().to_string(),
            request.uri().to_string(),
            false,
        );

        event.send(ProxyEvent::RequestEvent(logger)).unwrap();

        // if is_blocking {
        //     if (is_excluded_address && allow_by_default)
        //         || (!is_excluded_address & !allow_by_default)
        //     {
        //         let mut resp = Response::new(Self::full("Oopsie Whoopsie!"));
        //         *resp.status_mut() = http::StatusCode::FORBIDDEN;
        //         return Ok(resp);
        //     }
        // }

        if Method::CONNECT == request.method() {
            if let Some(addr) = Self::host_addr(request.uri()) {
                tokio::task::spawn(async move {
                    match hyper::upgrade::on(request).await {
                        Ok(upgraded) => {
                            let _ = Self::tunnel(upgraded, addr).await;
                        }
                        Err(_) => {}
                    }
                });

                return Ok(Response::new(Self::empty()));
            } else {
                let mut resp = Response::new(Self::full("CONNECT must be to a socket address"));
                *resp.status_mut() = http::StatusCode::BAD_REQUEST;

                return Ok(resp);
            }
        } else {
            let host = request
                .uri()
                .host()
                .expect(format!("Provided URI contains no Host: {}", request.uri()).as_str());
            let port = request.uri().port_u16().unwrap_or(80);

            let stream = TcpStream::connect((host, port)).await.unwrap();
            let io = TokioIo::new(stream);

            let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .handshake(io)
                .await?;

            tokio::task::spawn(async move {
                let _ = conn.await;
            });

            let response = sender.send_request(request).await?;
            Ok(response.map(|b| b.boxed()))
        }
    }

    async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
        let mut server = TcpStream::connect(addr).await?;
        let mut upgraded_connection = TokioIo::new(upgraded);

        tokio::io::copy_bidirectional(&mut upgraded_connection, &mut server).await?;

        Ok(())
    }

    fn host_addr(uri: &http::Uri) -> Option<String> {
        uri.authority().and_then(|auth| Some(auth.to_string()))
    }

    fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }

    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
        Full::new(chunk.into())
            .map_err(|never| match never {})
            .boxed()
    }

    /// Returns the Proxy's run-time
    pub fn get_run_time(&mut self) -> String {
        let run_time = match self.run_time.lock() {
            Ok(run_time) => run_time,
            Err(poisoned) => poisoned.into_inner(),
        };

        match *run_time {
            Some(duration) => duration.elapsed().as_secs().to_string(),
            None => 0.to_string(),
        }
    }

    /// Returns the Proxy's current status
    pub fn get_status(&mut self) -> ProxyEvent {
        let proxy_state = match self.status.lock() {
            Ok(proxy_event) => proxy_event,
            Err(poisoned) => poisoned.into_inner(),
        };

        proxy_state.clone()
    }

    /// Returns the Proxy's current exclusion list
    // pub fn get_current_list(&mut self) -> Vec<String> {
    //     let current_list = match self.current_list.lock() {
    //         Ok(current_list) => current_list,
    //         Err(poisoned) => poisoned.into_inner(),
    //     };

    //     current_list.clone()
    // }

    /// Returns the Proxy's current blocking status
    // pub fn get_blocking_status(&mut self) -> (bool, bool) {
    //     let blocking_status = match self.allow_blocking.lock() {
    //         Ok(blocking_status) => *blocking_status,
    //         Err(poisoned) => *poisoned.into_inner(),
    //     };

    //     let allowing_all_traffic = match self.allow_requests_by_default.lock() {
    //         Ok(allowing_all_traffic) => *allowing_all_traffic,
    //         Err(poisoned) => *poisoned.into_inner(),
    //     };

    //     (blocking_status, allowing_all_traffic)
    // }

    pub fn get_traffic_filter(&self) -> TrafficFilter {
        let traffic_filter = match self.traffic_filter.lock() {
            Ok(traffic_filter) => traffic_filter.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        traffic_filter
    }

    /// Returns the Proxy's recent requests
    pub fn get_requests(&self) -> Vec<ProxyRequestLog> {
        let requests_list = match self.requests.lock() {
            Ok(requests_list) => requests_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests_list.to_vec()
    }

    /// Sets whether the Proxy is using an exclusion list
    pub fn enable_exclusion(&self) {
        self.event
            .send(ProxyEvent::Blocking(
                self.get_traffic_filter().get_filter_list(),
            ))
            .unwrap();
    }

    /// Sets which exclusion list the Proxy is using
    pub fn switch_exclusion(&mut self) {
        // let allowing_all_traffic = match self.allow_requests_by_default.lock() {
        //     Ok(allowing_all_traffic) => allowing_all_traffic,
        //     Err(poisoned) => poisoned.into_inner(),
        // };
        // self.event
        //     .send(ProxyEvent::SwitchList(if *allowing_all_traffic {
        //         self.allow_list.clone()
        //     } else {
        //         self.block_list.clone()
        //     }))
        //     .unwrap();
    }

    /// Update the Proxy's exclusion list
    pub fn add_exclusion(&mut self) {
        // let mut list = self.get_current_list();
        // let selected_value = self.dragging_value.clone();

        // let is_already_excluded = list
        //     .iter()
        //     .any(|item| selected_value.contains(item) || item.contains(&selected_value));

        // if !is_already_excluded {
        //     list.push(self.dragging_value.clone());
        // } else {
        //     list.retain(|x| x.clone() != self.dragging_value.clone());
        // }

        // let (_is_blocking, allowing_all_traffic) = self.get_blocking_status();

        // if allowing_all_traffic {
        //     self.block_list = list.clone();
        // } else {
        //     self.allow_list = list.clone();
        // }

        // let mut current_list_mut = self.current_list.lock().unwrap();
        // *current_list_mut = list;
    }

    /// Update a single value in the Proxy's exclusion list
    pub fn update_exclusion_list_value(&mut self, uri: String) {
        // // TODO: This could be combined with the add_exclusion (rename to update_exclusion)
        // let mut list = self.get_current_list();

        // // Find index as cannot index by String
        // let uri_index = list.iter().position(|item| item == &uri).unwrap();

        // // Overwrite value in current_list
        // list[uri_index] = self.selected_exclusion_row.row_value.clone();

        // // Update allow/deny lists
        // let allowing_all_traffic = match self.allow_requests_by_default.lock() {
        //     Ok(allowing_all_traffic) => *allowing_all_traffic,
        //     Err(poisoned) => *poisoned.into_inner(),
        // };

        // if allowing_all_traffic {
        //     self.block_list = list.clone();
        // } else {
        //     self.allow_list = list.clone();
        // }

        // // Update current list values
        // let mut current_list_mut = self.current_list.lock().unwrap();
        // *current_list_mut = list;

        // // Reset edit values
        // self.selected_exclusion_row = ProxyExclusionRow::default();
    }
}
