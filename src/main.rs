use axum::{
    extract::ws::{ Message, WebSocket, WebSocketUpgrade },
    extract::{ State, connect_info::ConnectInfo },
    response::IntoResponse, 
    routing::get,
    Router,
};
use futures::{
    sink::SinkExt, stream::StreamExt,
};
use std::sync::Arc;
use std::net::SocketAddr; 
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

    fn update(&mut self) -> () { // TODO: better error handling
        if self.running {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.last_updated).as_secs_f64();
            self.spent_seconds += dt; 
            self.last_updated = now;
        }
    }

    fn show(&self) -> String {
        format!("running: {}, spent_seconds: {}, last_updated: {:?}", 
            self.running, 
            self.spent_seconds, 
            self.last_updated)
    }
}

async fn handle_socket(mut socket: WebSocket, state: Arc<RwLock<TimerState>>) {
    while let Some(message) = socket.recv().await {
        // logging

        // handling
        let ret = match message {
            Ok(msg) => msg,
            Err(_) => return
        };
        if socket.send(ret).await.is_err() {
            return; 
        }
        let state_msg = Message::Text(state.read().await.show());
        if socket.send(state_msg).await.is_err() {
            return;
        }
    }
}

async fn ws_hook(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<TimerState>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // logging
    println!("connection from: {}", addr);
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
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
