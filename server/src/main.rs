use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    response::IntoResponse,
    routing::{get},
    Router,
};
use tokio::net::TcpListener;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_tunnel_server=debug,tower_http=debug,reqwest=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Simplified Rust Tunnel Server...");

    let app = Router::new()
        .route("/*path", get(get_handler))
        .route("/ws", get(ws_handler));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn get_handler(
    Path(path_segment): Path<String>,
) -> impl IntoResponse {
    info!("Received request on path: /dynamic/{}", path_segment);
    format!("You requested the path: '{}'", path_segment)
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    info!("Attempting to upgrade connection to WebSocket on /ws");
    ws.on_upgrade(handle_single_websocket)
}

async fn handle_single_websocket(mut socket: WebSocket) {
    info!("WebSocket connected on /ws");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                info!("Received text from WebSocket: {}", text);
                if let Err(e) = socket.send(Message::Text(format!("Server echoed: {}", text))).await {
                    error!("Failed to send text message to WebSocket: {:?}", e);
                    break;
                }
            }
            Message::Binary(bin) => {
                info!("Received binary from WebSocket: {:?}", bin);
                if let Err(e) = socket.send(Message::Binary(bin)).await {
                    error!("Failed to send binary message to WebSocket: {:?}", e);
                    break;
                }
            }
            Message::Ping(pong) => {
                info!("Received Ping from WebSocket. Sending Pong.");
                if let Err(e) = socket.send(Message::Pong(pong)).await {
                    error!("Failed to send Pong to WebSocket: {:?}", e);
                    break;
                }
            }
            Message::Pong(_) => {
                info!("Received Pong from WebSocket.");
            }
            Message::Close(close_frame) => {
                info!("Received Close from WebSocket: {:?}", close_frame);
                break;
            }
        }
    }
    info!("WebSocket disconnected.");
}
