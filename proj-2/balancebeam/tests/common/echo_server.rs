use crate::common::server::Server;
use async_trait::async_trait;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{Request, Response};
use rand::Rng;
use std::sync::{atomic, Arc};
use tokio::sync::oneshot;
// Required for collect and to_bytes

#[derive(Debug)]
struct ServerState {
    pub requests_received: atomic::AtomicUsize,
}

async fn echo(
    server_state: Arc<ServerState>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    server_state
        .requests_received
        .fetch_add(1, atomic::Ordering::SeqCst);
    let mut req_text = format!("{} {} {:?}\n", req.method(), req.uri(), req.version());
    for (header_name, header_value) in req.headers() {
        req_text += &format!(
            "{}: {}\n",
            header_name.as_str(),
            header_value.to_str().unwrap_or("<binary value>")
        );
    }
    req_text += "\n";
    let mut req_as_bytes = req_text.into_bytes();
    let body_bytes = req.into_body().collect().await?.to_bytes(); // Correctly collect body into Bytes
    req_as_bytes.extend(body_bytes);
    Ok(Response::new(Full::new(Bytes::from(req_as_bytes))))
}

pub struct EchoServer {
    shutdown_signal_sender: oneshot::Sender<()>,
    server_task: tokio::task::JoinHandle<()>,
    pub address: String,
    state: Arc<ServerState>,
}

impl EchoServer {
    pub async fn new() -> EchoServer {
        let mut rng = rand::rng();
        EchoServer::new_at_address(format!("127.0.0.1:{}", rng.random_range(1024..65535))).await
    }

    pub async fn new_at_address(bind_addr_string: String) -> EchoServer {
        let bind_addr_string_clone = bind_addr_string.clone(); // Clone for the closure
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let server_state = Arc::new(ServerState {
            requests_received: atomic::AtomicUsize::new(0),
        });
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
                        let service = service_fn({
                            let server_task_state = server_task_state.clone();
                            move |req| {
                                let server_task_state = server_task_state.clone();
                                echo(server_task_state, req)
                            }
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
        EchoServer {
            shutdown_signal_sender: shutdown_tx,
            server_task,
            state: server_state,
            address: bind_addr_string, // Use the original
        }
    }
}

#[async_trait]
impl Server for EchoServer {
    async fn stop(self: Box<Self>) -> usize {
        let _ = self.shutdown_signal_sender.send(());
        self.server_task
            .await
            .expect("EchoServer server task panicked");
        self.state.requests_received.load(atomic::Ordering::SeqCst)
    }

    fn address(&self) -> String {
        self.address.clone()
    }
}
