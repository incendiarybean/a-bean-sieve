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

use super::traffic_filter::{TrafficFilter, TrafficFilterType};

#[derive(Debug, PartialEq, Clone)]
pub enum ProxyEvent {
    Running,
    Stopped,
    Error(String),
    Terminating,
    Terminated,
    RequestEvent((String, String, bool)),

    // Traffic Filter related Events
    ToggleFilterActive,
    SwitchFilterList,
    SetFilterList(Vec<String>),
    UpdateFilterList(String),
    UpdateFilterListRecord(usize, String),
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ProxyExclusionRow {
    pub updating: bool,
    pub index: usize,
    pub value: String,
}

impl Default for ProxyExclusionRow {
    fn default() -> Self {
        Self {
            updating: bool::default(),
            index: usize::default(),
            value: String::default(),
        }
    }
}

#[derive(serde::Serialize)]
pub struct ProxyExclusionList {
    pub request: String,
}

pub enum ProxyExclusionUpdateKind {
    Edit,
    Add,
    Remove,
}

#[derive(serde::Serialize, Clone)]
pub struct ProxyRequestLog {
    pub method: String,
    pub request: String,
    pub blocked: bool,
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

    // Traffic Filters
    pub traffic_filter: Arc<Mutex<TrafficFilter>>,

    // Different value selectors for exclusion management
    pub selected_value: String,
    pub selected_exclusion_row: ProxyExclusionRow,

    // Skip these as we don't want to restore these values
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
            Arc::clone(&status),
            Arc::clone(&requests),
            Arc::clone(&run_time),
            Arc::clone(&traffic_filter),
        );

        Self {
            port: String::new(),
            port_error: String::default(),
            start_enabled: true,
            event: event_sender,
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
    /// * `logs` - A bool that contains whether the logs are showing or not
    /// * `traffic_filter` - A TrafficFilter containing the applied filters
    pub fn new(port: String, logs: bool, traffic_filter: TrafficFilter) -> Self {
        let (event_sender, event_receiver) = std::sync::mpsc::channel::<ProxyEvent>();

        // Need a value, and a shareable value to update the original reference
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let requests = Arc::new(Mutex::new(Vec::<ProxyRequestLog>::new()));
        let run_time = Arc::new(Mutex::new(None));
        let traffic_filter = Arc::new(Mutex::new(traffic_filter));

        Self::event_handler(
            event_receiver,
            Arc::clone(&status),
            Arc::clone(&requests),
            Arc::clone(&run_time),
            Arc::clone(&traffic_filter),
        );

        Self {
            port,
            port_error: String::default(),
            start_enabled: true,
            event: event_sender,
            selected_value: String::new(),
            selected_exclusion_row: ProxyExclusionRow::default(),
            status,
            logs,
            requests,
            run_time,
            traffic_filter,
        }
    }

    /// The ProxyEvent handler
    ///
    /// # Arguments:
    /// * `event_receiver` - The listener for ProxyEvents
    /// * `status` - The self.status Mutex
    /// * `requests` - The self.requests Mutex
    /// * `run_time` - The self.run_time Mutex
    /// * `traffic_filter` - The self.traffic_filter Mutex
    fn event_handler(
        event_receiver: Receiver<ProxyEvent>,
        status: Arc<Mutex<ProxyEvent>>,
        requests: Arc<Mutex<Vec<ProxyRequestLog>>>,
        run_time: Arc<Mutex<Option<std::time::Instant>>>,
        traffic_filter: Arc<Mutex<TrafficFilter>>,
    ) {
        thread::spawn(move || {
            loop {
                // Sleep loop to loosen CPU stress
                thread::sleep(Duration::from_millis(100));

                // Check incoming Proxy events
                match event_receiver.recv() {
                    Ok(event) => match event {
                        // Generic Events
                        ProxyEvent::Running => {
                            println!("{}", "Running service...".green());

                            let mut run_time = run_time.lock().unwrap();
                            *run_time = Some(std::time::Instant::now());

                            let mut status = status.lock().unwrap();
                            *status = event;
                        }
                        ProxyEvent::Terminated | ProxyEvent::Stopped => {
                            let mut status = status.lock().unwrap();
                            *status = ProxyEvent::Stopped;

                            let mut run_time = run_time.lock().unwrap();
                            *run_time = None;
                        }
                        ProxyEvent::Error(message) => {
                            println!("{}", message.red());

                            let mut status = status.lock().unwrap();
                            *status = ProxyEvent::Error(message);
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

                            let mut requests_list = requests.lock().unwrap();
                            requests_list.push(ProxyRequestLog {
                                method,
                                request,
                                blocked,
                            });
                        }
                        // Traffic Filter events
                        ProxyEvent::ToggleFilterActive => {
                            let mut traffic_filter = traffic_filter.lock().unwrap();
                            let enabled = traffic_filter.get_enabled();
                            traffic_filter.set_enabled(!enabled);
                        }
                        ProxyEvent::SwitchFilterList => {
                            let mut traffic_filter = traffic_filter.lock().unwrap();
                            let switched_filter = traffic_filter.get_opposing_filter_type();
                            traffic_filter.set_filter_type(switched_filter);
                        }
                        ProxyEvent::SetFilterList(exclusion_list) => {
                            let mut traffic_filter = traffic_filter.lock().unwrap();
                            traffic_filter.set_filter_list(exclusion_list);
                        }
                        ProxyEvent::UpdateFilterList(uri) => {
                            let mut traffic_filter = traffic_filter.lock().unwrap();
                            traffic_filter.update_filter_list(uri);
                        }
                        ProxyEvent::UpdateFilterListRecord(index, value) => {
                            let mut traffic_filter = traffic_filter.lock().unwrap();
                            traffic_filter.update_filter_list_item(index, value);
                        }
                        _ => {
                            let mut status = status.lock().unwrap();
                            *status = event;
                        }
                    },
                    Err(message) => {
                        let mut status = status.lock().unwrap();
                        *status = ProxyEvent::Error(message.to_string())
                    }
                }
            }
        });
    }

    #[tokio::main]
    pub async fn proxy_service(self) -> io::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port.trim().parse::<u16>().unwrap()));

        let mut signal = std::pin::pin!(Self::handle_termination(self.event.clone(), self.status));

        let listener = TcpListener::bind(addr).await;

        let internal_event_sender = self.event.clone();

        let proxy_service = service_fn(move |request| {
            Self::request(
                request,
                internal_event_sender.clone(),
                self.traffic_filter.lock().unwrap().clone(),
            )
        });

        match listener {
            Ok(listener) => {
                self.event.send(ProxyEvent::Running).unwrap();

                loop {
                    tokio::select! {
                        Ok((stream, _addr)) = listener.accept() => {
                            let io = TokioIo::new(stream);
                            let connection = http1::Builder::new()
                                .preserve_header_case(true)
                                .title_case_headers(true)
                                .serve_connection(io, proxy_service.clone())
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
                event.send(ProxyEvent::Terminated).unwrap();
                println!("{}", "Terminated Service.".red());
            }
            Err(_) => {}
        }
    }

    /// Handle a server request
    ///
    /// # Arguments:
    /// * `request` - The request to proxy
    /// * `event` - An internal event sender, to change the Proxy state
    /// * `traffic_filter` - The current TrafficFilter configuration
    async fn request(
        request: Request<hyper::body::Incoming>,
        event: std::sync::mpsc::Sender<ProxyEvent>,
        traffic_filter: TrafficFilter,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let request_uri = request.uri().to_string();

        let is_excluded_address = traffic_filter.in_filter_list(&request_uri);
        let is_traffic_blocking = match traffic_filter.get_filter_type() {
            TrafficFilterType::Allow => false,
            TrafficFilterType::Deny => true,
        };

        if traffic_filter.get_enabled() {
            let is_blocking_but_exluded = !is_excluded_address && is_traffic_blocking;
            let is_allowing_but_excluded = is_excluded_address && !is_traffic_blocking;
            let blocked = is_allowing_but_excluded || is_blocking_but_exluded;

            // Log the event
            let logger = (request.method().to_string(), request_uri, blocked);
            event.send(ProxyEvent::RequestEvent(logger)).unwrap();

            if blocked {
                let mut resp = Response::new(Self::full("Oopsie Whoopsie!"));
                *resp.status_mut() = http::StatusCode::FORBIDDEN;
                return Ok(resp);
            }
        }

        if Method::CONNECT == request.method() {
            if let Some(addr) = Self::get_host_address(request.uri()) {
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
            match request.uri().host() {
                Some(host) => {
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
                None => {
                    let mut resp =
                        Response::new(Self::full("Host address could not be processed."));
                    *resp.status_mut() = http::StatusCode::BAD_REQUEST;
                    return Ok(resp);
                }
            }
        }
    }

    /// Tunnel a connection bidirectionally
    ///
    /// # Arguments:
    /// * `upgraded` - The upgraded connection to copy data to/from
    /// * `address` - The target address to copy data to/from
    async fn tunnel(upgraded: Upgraded, address: String) -> std::io::Result<()> {
        let mut server = TcpStream::connect(address).await?;
        let mut upgraded_connection = TokioIo::new(upgraded);

        tokio::io::copy_bidirectional(&mut upgraded_connection, &mut server).await?;

        Ok(())
    }

    /// Get the current URI's host address
    ///
    /// # Arguments
    /// * `uri` - The given URI
    fn get_host_address(uri: &http::Uri) -> Option<String> {
        uri.authority().and_then(|auth| Some(auth.to_string()))
    }

    /// Create an empty response body
    fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }

    /// Create an body from the given bytes
    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
        Full::new(chunk.into())
            .map_err(|never| match never {})
            .boxed()
    }

    /// Returns the Proxy's current status
    pub fn get_status(&mut self) -> ProxyEvent {
        let proxy_state = match self.status.lock() {
            Ok(proxy_event) => proxy_event,
            Err(poisoned) => poisoned.into_inner(),
        };

        proxy_state.clone()
    }

    /// Returns the Proxy's current TrafficFilter
    pub fn get_traffic_filter(&self) -> TrafficFilter {
        let traffic_filter = match self.traffic_filter.lock() {
            Ok(traffic_filter) => traffic_filter,
            Err(poisoned) => poisoned.into_inner(),
        };

        traffic_filter.clone()
    }

    /// Returns the Proxy's recent requests
    pub fn get_requests(&self) -> Vec<ProxyRequestLog> {
        let requests_list = match self.requests.lock() {
            Ok(requests_list) => requests_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests_list.to_vec()
    }

    /// Returns the Proxy's current running time
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

    /// Send an event to toggle the TrafficFilter on/off
    pub fn toggle_traffic_filtering(&self) {
        self.event.send(ProxyEvent::ToggleFilterActive).unwrap();
    }

    /// Send an event to toggle between TrafficFilterType::Allow / TrafficFilterType::Deny
    pub fn switch_exclusion_list(&self) {
        self.event.send(ProxyEvent::SwitchFilterList).unwrap();
    }

    /// Send an event to set the current exclusion list
    pub fn set_exclusion_list(&mut self, list: Vec<String>) {
        self.event.send(ProxyEvent::SetFilterList(list)).unwrap();
    }

    /// Send an event to add a value to the current exclusion list
    pub fn update_exclusion_list(&mut self, event_type: ProxyExclusionUpdateKind) {
        match event_type {
            ProxyExclusionUpdateKind::Edit => {
                self.event
                    .send(ProxyEvent::UpdateFilterListRecord(
                        self.selected_exclusion_row.index,
                        self.selected_exclusion_row.value.clone(),
                    ))
                    .unwrap();

                self.selected_exclusion_row = ProxyExclusionRow::default();
            }
            ProxyExclusionUpdateKind::Add | ProxyExclusionUpdateKind::Remove => self
                .event
                .send(ProxyEvent::UpdateFilterList(self.selected_value.clone()))
                .unwrap(),
        };
    }
}
