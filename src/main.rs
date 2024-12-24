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
use serde::{ Serialize, Deserialize };
use std::sync::Arc;
use std::net::SocketAddr; 
use tokio::sync::RwLock;

#[derive(Debug)]
struct TimerState {
    running: bool, 
    time_left: f64,
    last_updated: std::time::Instant,
}

impl TimerState {
    fn new() -> TimerState {
        TimerState {
            running: false, 
            time_left: 0f64,
            last_updated: std::time::Instant::now() 
        }
    }

    fn update(&mut self) -> bool { // TODO: change bool to enum
        if self.running {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.last_updated).as_secs_f64();
            self.time_left = (self.time_left - dt).max(0.0);
            self.last_updated = now;
            
            if self.time_left == 0.0 {
                self.running = false; 
                true
            } else {
                false
            }
        } else { false }
    }

    fn show(&self) -> String {
        format!("running: {}, spent_seconds: {}, last_updated: {:?}", 
            self.running, 
            self.time_left, 
            self.last_updated)
    }
}

#[derive(Deserialize)]
enum TimerCommands {
    Start, 
    Stop, 
    Reset, 
    SetTime { min: u32, sec: u32 },
}

struct TimerResponse {
    running: bool, 
    display: String, 
    finished: bool,
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
