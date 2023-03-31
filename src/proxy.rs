use crate::csv_reader::read_from_csv;
use colored::Colorize;
use hyper::{
    http,
    service::{make_service_fn, service_fn},
    upgrade::Upgraded,
    Body, Client, Method, Request, Response, Server,
};
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::net::TcpStream;
type HttpClient = Client<hyper::client::HttpConnector>;

pub enum ProxyEvent {
    Running,
    Stopped,
    Error,
    Terminating,
    Terminated,
    RequestEvent((String, String, bool)),
    Blocking(bool, Vec<String>),
    UpdateList(Vec<String>),
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Clone)]
pub struct Proxy {
    pub port: String,
    pub port_error: String,
    pub logs: bool,
    #[serde(skip)]
    pub allow_list: Vec<String>,
    #[serde(skip)]
    pub block_list: Vec<String>,

    // #[serde(skip)] // We don't want to allow starting proxy by default
    pub start_enabled: bool,

    // Mutex values seem to be locked when they get entered, will check on this
    #[serde(skip)]
    pub allow_requests_by_default: Arc<Mutex<bool>>,
    #[serde(skip)]
    pub event: std::sync::mpsc::Sender<ProxyEvent>,
    #[serde(skip)]
    pub status: Arc<Mutex<ProxyEvent>>,
    #[serde(skip)]
    pub requests: Arc<Mutex<Vec<(String, String, bool)>>>,
    #[serde(skip)]
    pub allow_blocking: Arc<Mutex<bool>>,
    #[serde(skip)]
    pub current_list: Arc<Mutex<Vec<String>>>,
}

impl Default for Proxy {
    fn default() -> Self {
        // I'm going to want to figure out dynamic file access or saved state
        let allow_list_file: &[u8] = include_bytes!("./allow_list.csv");
        let allow_list = match read_from_csv::<String>(allow_list_file) {
            Ok(list) => list,
            Err(_) => Vec::<String>::new(),
        };

        let block_list_file: &[u8] = include_bytes!("./block_list.csv");
        let block_list = match read_from_csv::<String>(block_list_file) {
            Ok(list) => list,
            Err(_) => Vec::<String>::new(),
        };

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
            thread::sleep(Duration::from_millis(100));
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
                    ProxyEvent::Blocking(is_blocking, default_list) => {
                        let mut blocking = allow_blocking_clone.lock().unwrap();
                        *blocking = is_blocking;

                        let mut current_list = current_list_clone.lock().unwrap();
                        *current_list = default_list;
                    }
                    ProxyEvent::UpdateList(new_list) => {
                        // When updating the list, toggle the traffic control and update the list
                        let mut allow_requests_by_default =
                            allow_requests_by_default_clone.lock().unwrap();
                        *allow_requests_by_default = !*allow_requests_by_default;

                        let mut current_list = current_list_clone.lock().unwrap();
                        *current_list = new_list;
                    }
                    _ => {
                        // If there is no custom event handler, simply set the value of status to this event type
                        let mut status = status_clone.lock().unwrap();
                        *status = event;
                    }
                },
                Err(_) => (),
            }
        });

        Self {
            port: String::from("8000"),
            port_error: String::default(),
            start_enabled: false,
            event: event_sender.clone(),
            status,
            logs: false,
            requests,
            allow_blocking,
            allow_requests_by_default,
            allow_list,
            block_list,
            current_list,
        }
    }
}

impl Proxy {
    // Event listener for Proxy Termination
    fn handle_termination(
        shutdown_sig: tokio::sync::oneshot::Sender<()>,
        event: std::sync::mpsc::Sender<ProxyEvent>,
        status: Arc<Mutex<ProxyEvent>>,
    ) {
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
                    println!("{}", "Terminating Service.".red());
                    shutdown_sig.send(()).unwrap();
                    break;
                }
                _ => (),
            };
        }

        // Send event to show it's Terminated/Stopped
        event.send(ProxyEvent::Terminated).unwrap();
    }

    #[tokio::main]
    pub async fn proxy_service(self) {
        let addr = SocketAddr::from((
            [127, 0, 0, 1],
            self.port.trim().parse::<u16>().unwrap().clone(),
        ));

        // Create a oneshot channel for sending a single burst of a termination signal
        let (shutdown_sig, shutdown_rec) = tokio::sync::oneshot::channel::<()>();

        let client = Client::builder()
            .http1_title_case_headers(true)
            .http1_preserve_header_case(true)
            .build_http();

        let request_event_sender = self.event.clone();
        let make_service = make_service_fn(move |_| {
            let client = client.clone();
            let request_event_sender = request_event_sender.clone();

            // Check if address blocking is currently in use
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
            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    return Self::request(
                        client.clone(),
                        request,
                        request_event_sender.clone(),
                        is_blocking,
                        allow_by_default.clone(),
                        configured_list.clone(),
                    );
                }))
            }
        });

        // I try to bind here to check if the Port is available to bind to
        let server = Server::try_bind(&addr);
        match server {
            Ok(builder) => {
                self.event.send(ProxyEvent::Running).unwrap();

                // Create handler for monitoring ProxyEvent - Termination Status
                let event_clone = self.event.clone();
                thread::spawn(move || {
                    Self::handle_termination(shutdown_sig, event_clone, self.status.clone());
                });

                // Create server
                let server = builder
                    .http1_preserve_header_case(true)
                    .http1_title_case_headers(true)
                    .serve(make_service)
                    .with_graceful_shutdown(async {
                        shutdown_rec.await.ok();
                    });

                // Run server non-stop unless there's an error
                if let Err(_) = server.await {
                    self.event.send(ProxyEvent::Error).unwrap();
                }
            }
            Err(_) => self.event.send(ProxyEvent::Error).unwrap(),
        }
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

    pub fn get_current_list(&mut self) -> Vec<String> {
        let current_list = match self.current_list.lock() {
            Ok(current_list) => current_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        current_list.clone()
    }

    pub fn get_blocking_status(&mut self) -> (bool, bool) {
        let blocking_status = match self.allow_blocking.lock() {
            Ok(blocking_status) => blocking_status,
            Err(poisoned) => poisoned.into_inner(),
        };

        let allowing_all_traffic = match self.allow_requests_by_default.lock() {
            Ok(allowing_all_traffic) => allowing_all_traffic,
            Err(poisoned) => poisoned.into_inner(),
        };

        (*blocking_status, *allowing_all_traffic)
    }

    pub fn get_requests(&mut self) -> Vec<(String, String, bool)> {
        let requests_list = match self.requests.lock() {
            Ok(requests_list) => requests_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests_list.to_vec()
    }

    pub async fn request(
        client: HttpClient,
        request: Request<Body>,
        event: std::sync::mpsc::Sender<ProxyEvent>,
        is_blocking: bool,
        allow_by_default: bool,
        blocking_list: Vec<String>,
    ) -> Result<Response<Body>, hyper::Error> {
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
                let mut resp = Response::new(Body::from("Oopsie Whoopsie!"));
                *resp.status_mut() = http::StatusCode::FORBIDDEN;
                return Ok(resp);
            }
        }

        // Forward the rest of accepted requests
        if Method::CONNECT == request.method() {
            if let Some(addr) = Self::host_addr(request.uri()) {
                tokio::task::spawn(async move {
                    match hyper::upgrade::on(request).await {
                        Ok(upgraded) => {
                            if let Err(_) = Self::tunnel(upgraded, addr).await {
                                // This error mostly indicates an external host closed a connection
                                // Don't need to worry about that
                            };
                        }
                        Err(e) => println!("upgrade error: {}", e.to_string().red()),
                    }
                });

                Ok(Response::new(Body::empty()))
            } else {
                let mut resp = Response::new(Body::from("CONNECT must be to a socket address"));
                *resp.status_mut() = http::StatusCode::BAD_REQUEST;

                return Ok(resp);
            }
        } else {
            return client.request(request).await;
        }
    }

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

    fn host_addr(uri: &http::Uri) -> Option<String> {
        uri.authority().and_then(|auth| Some(auth.to_string()))
    }

    async fn tunnel(mut upgraded: Upgraded, addr: String) -> std::io::Result<()> {
        let mut server = TcpStream::connect(addr).await?;
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
        Ok(())
    }
}
