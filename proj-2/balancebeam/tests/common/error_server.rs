use crate::common::server::Server;
use async_trait::async_trait;
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::Response;
use rand::Rng;
use std::sync::{atomic, Arc};
use tokio::sync::oneshot;

/// Represents the state of the server including the count of received requests.
#[derive(Debug)]
struct ServerState {
    /// Atomic counter for the number of requests received.
    pub requests_received: atomic::AtomicUsize,
}

/// Returns an HTTP response with a 500 Internal Server Error status.
async fn return_error() -> Result<Response<Empty<Bytes>>, hyper::Error> {
    Ok(Response::builder()
        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
        .body(Empty::new())
        .unwrap())
}

/// A server that always returns an internal server error for any request.
pub struct ErrorServer {
    shutdown_signal_sender: oneshot::Sender<()>,
    server_task: tokio::task::JoinHandle<()>,
    /// The address on which the server is listening.
    pub address: String,
    state: Arc<ServerState>,
}

impl ErrorServer {
    /// Creates a new `ErrorServer` instance binding to a random available port.
    pub async fn new() -> ErrorServer {
        let mut rng = rand::rng();
        ErrorServer::new_at_address(format!("127.0.0.1:{}", rng.random_range(1024..65535))).await
    }

    /// Creates a new `ErrorServer` instance binding to the specified address.
    pub async fn new_at_address(bind_addr_string: String) -> ErrorServer {
        let bind_addr_string_clone = bind_addr_string.clone();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let server_state = Arc::new(ServerState {
            requests_received: atomic::AtomicUsize::new(0),
        });
        // Clone the state to share it between connections.
        let server_task_state = server_state.clone();
        let server_task = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&bind_addr_string_clone)
                .await
                .unwrap();
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    result = listener.accept() => {
                        let (stream, _) = result.unwrap();
                        let io = hyper_util::rt::TokioIo::new(stream);
                        // Clone the shared state for this connection.
                        let state = server_task_state.clone();
                        let service = service_fn(move |_req| {
                            // Increment the request counter atomically.
                            state.requests_received.fetch_add(1, atomic::Ordering::SeqCst);
                            return_error()
                        });
                        tokio::spawn(async move {
                            if let Err(e) = hyper::server::conn::http1::Builder::new()
                                .serve_connection(io, service)
                                .await
                            {
                                log::error!("Error serving connection: {}", e);
                            }
                        });
                    }
                }
            }
        });
        ErrorServer {
            shutdown_signal_sender: shutdown_tx,
            server_task,
            state: server_state,
            address: bind_addr_string,
        }
    }
}

#[async_trait]
impl Server for ErrorServer {
    /// Stops the server and returns the number of requests received.
    async fn stop(self: Box<Self>) -> usize {
        let _ = self.shutdown_signal_sender.send(());
        self.server_task
            .await
            .expect("ErrorServer server task panicked");
        self.state.requests_received.load(atomic::Ordering::SeqCst)
    }

    /// Returns the address the server is bound to.
    fn address(&self) -> String {
        self.address.clone()
    }
}
