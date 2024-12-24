use axum::{
    extract::ws::{ Message, WebSocket, WebSocketUpgrade },
    extract::State,
    response::IntoResponse, 
    routing::get,
    Router,
};
use futures::{
    sink::SinkExt, stream::StreamExt,
};
use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Debug)]
struct TimerState {
    running: bool, 
    spent_seconds: f64, 
    last_updated: std::time::Instant,
}

impl TimerState {
    fn new() -> TimerState {
        TimerState {
            running: false, 
            spent_seconds: 0f64,
            last_updated: std::time::Instant::now() 
        }
    }
}

async fn handle_socket(mut socket: WebSocket, state: Arc<RwLock<TimerState>>) {
    while let Some(message) = socket.recv().await {
        let ret = match message {
            Ok(msg) => msg,
            Err(_) => return
        };
        if socket.send(ret).await.is_err() {
            return; 
        }
    }
}

async fn ws_hook(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<TimerState>>>
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[tokio::main]
async fn main() {
    let app_state: RwLock<TimerState> = RwLock::new(TimerState::new());
    let app = Router::new()
        .route("/ws", get(ws_hook))
        .with_state(Arc::new(app_state)) as Router;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
