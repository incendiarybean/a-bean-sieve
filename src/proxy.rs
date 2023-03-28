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
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::net::TcpStream;
type HttpClient = Client<hyper::client::HttpConnector>;

fn handle_termination(
    shutdown_sig: tokio::sync::oneshot::Sender<()>,
    proxy_event_sender: std::sync::mpsc::Sender<ProxyEvent>,
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
    proxy_event_sender.send(ProxyEvent::Terminated).unwrap();
}

#[tokio::main]
pub async fn proxy_service(
    addr: SocketAddr,
    proxy_event_sender: std::sync::mpsc::Sender<ProxyEvent>,
    status: Arc<Mutex<ProxyEvent>>,
    allow_blocking: Arc<Mutex<bool>>,
) {
    let addr = addr;

    // Create a oneshot channel for sending a single burst of a termination signal
    let (shutdown_sig, shutdown_rec) = tokio::sync::oneshot::channel::<()>();

    let client = Client::builder()
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build_http();

    let request_event_sender = proxy_event_sender.clone();
    let make_service = make_service_fn(move |_| {
        let client = client.clone();
        let request_event_sender = request_event_sender.clone();

        // Check if address blocking is currently in use
        let is_blocking = match allow_blocking.lock() {
            Ok(is_blocking) => *is_blocking,
            Err(poisoned) => *poisoned.into_inner(),
        };
        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                return Proxy::request(
                    client.clone(),
                    request,
                    request_event_sender.clone(),
                    is_blocking,
                );
            }))
        }
    });

    // I try to bind here to check if the Port is available to bind to
    let server = Server::try_bind(&addr);
    match server {
        Ok(builder) => {
            proxy_event_sender.send(ProxyEvent::Running).unwrap();

            // Create handler for monitoring ProxyEvent - Termination Status
            let proxy_event_sender_clone = proxy_event_sender.clone();
            thread::spawn(move || {
                handle_termination(shutdown_sig, proxy_event_sender_clone, status);
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
                proxy_event_sender.send(ProxyEvent::Error).unwrap();
            }
        }
        Err(_) => proxy_event_sender.send(ProxyEvent::Error).unwrap(),
    }
}

pub enum ProxyEvent {
    Running,
    Stopped,
    Error,
    Terminating,
    Terminated,
    RequestEvent((String, bool)),
    Blocking(bool),
}

pub struct Proxy {
    pub port: String,
    pub port_error: String,
    pub start_enabled: bool,
    pub event: std::sync::mpsc::Sender<ProxyEvent>,
    pub status: Arc<Mutex<ProxyEvent>>,
    pub logs: bool,

    pub requests: Arc<Mutex<Vec<(String, bool)>>>,
    pub allow_blocking: Arc<Mutex<bool>>,
    pub blocking_by_allow: bool,

    pub allow_list: Vec<String>,
    pub block_list: Vec<String>,
}

impl Default for Proxy {
    fn default() -> Self {
        let (event_sender, event_receiver) = std::sync::mpsc::channel::<ProxyEvent>();
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let status_clone = Arc::clone(&status);

        let requests = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));
        let requests_clone = Arc::clone(&requests);

        let allow_blocking = Arc::new(Mutex::new(false));
        let allow_blocking_clone = Arc::clone(&allow_blocking);

        thread::spawn(move || loop {
            match event_receiver.recv() {
                Ok(event) => match event {
                    ProxyEvent::Terminated | ProxyEvent::Stopped => {
                        let mut status = status_clone.lock().unwrap();
                        *status = ProxyEvent::Stopped;
                    }
                    ProxyEvent::RequestEvent((uri, blocked)) => {
                        println!(
                            "{} {} {}",
                            "REQUEST:".green(),
                            uri,
                            if blocked {
                                "-> BLOCKED".red()
                            } else {
                                "-> ALLOWED".green()
                            }
                        );

                        let mut status = requests_clone.lock().unwrap();
                        status.push((uri, blocked));
                    }
                    ProxyEvent::Blocking(is_blocking) => {
                        let mut blocking = allow_blocking_clone.lock().unwrap();
                        *blocking = is_blocking;
                    }
                    _ => {
                        let mut status = status_clone.lock().unwrap();
                        *status = event;
                    }
                },
                Err(_) => (),
            }
        });

        let path = Path::new("./");
        let allow_list =
            match read_from_csv::<String>(&format!("{}/allow_list.csv", path.display())) {
                Ok(list) => list,
                Err(e) => {
                    println!("ERROR, {}", e);
                    Vec::<String>::new()
                }
            };

        let block_list =
            match read_from_csv::<String>(&format!("{}/block_list.csv", path.display())) {
                Ok(list) => list,
                Err(_) => Vec::<String>::new(),
            };

        Self {
            port: String::from("8000"),
            port_error: String::default(),
            start_enabled: false,
            event: event_sender.clone(),
            status,
            logs: false,
            requests,
            allow_blocking,
            blocking_by_allow: false,
            allow_list,
            block_list,
        }
    }
}

impl Proxy {
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

    pub fn get_requests(&mut self) -> Vec<(String, bool)> {
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
    ) -> Result<Response<Body>, hyper::Error> {
        // I'll need to do an Arc<Mutex<Bool>> for watching whether the blocking is enabled
        // Check if address is within blocked list, send FORBIDDEN response on bad request
        let blocked_address = Self::is_blocked_addr(request.uri().to_string());

        let logger = (
            request.uri().to_string(),
            if blocked_address && is_blocking {
                true
            } else {
                false
            },
        );

        event.send(ProxyEvent::RequestEvent(logger)).unwrap();

        if blocked_address && is_blocking {
            let mut resp = Response::new(Body::from("Oopsie Whoopsie!"));
            *resp.status_mut() = http::StatusCode::FORBIDDEN;
            return Ok(resp);
        }

        // Forward the rest of accepted requests
        if Method::CONNECT == request.method() {
            if let Some(addr) = Self::host_addr(request.uri()) {
                tokio::task::spawn(async move {
                    match hyper::upgrade::on(request).await {
                        Ok(upgraded) => {
                            if let Err(e) = Self::tunnel(upgraded, addr).await {
                                println!("server io error: {}", e.to_string().red());
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

    pub fn is_blocked_addr(uri: String) -> bool {
        // This needs to be replaced by a vec<string> passed in, this will depend on whether you're using an allow or deny list
        let allowed_uri_list = match read_from_csv::<String>("./src/whitelist.csv") {
            Ok(uri_list) => uri_list,
            Err(_) => Vec::new(),
        };

        let is_blocked = {
            let mut is_blocked = true;

            for item in allowed_uri_list {
                if uri.contains(&item) {
                    is_blocked = false;
                    break;
                }
            }

            is_blocked
        };

        is_blocked
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
