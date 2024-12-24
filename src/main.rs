use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{connect_info::ConnectInfo, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
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
            last_updated: std::time::Instant::now(),
        }
    }

    fn update(&mut self) -> bool {
        // TODO: change bool to enum
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
        } else {
            false
        }
    }

    fn show(&self) -> String {
        format!(
            "time_left: {}:{}",
            (self.time_left / 60.0).floor(), (self.time_left % 60.0).floor()
        )
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd")]
enum TimerCommands {
    Start,
    Stop,
    Reset,
    SetTime { min: u32, sec: u32 },
}

#[derive(Serialize)]
struct TimerResponse {
    running: bool,
    display: String,
    finished: bool,
}

async fn handle_socket(mut socket: WebSocket, state: Arc<RwLock<TimerState>>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(_) => break,
                };
                if let Message::Text(txt) = msg {
                    println!("Received: {}\n", txt);
                    if let Ok(cmd) = serde_json::from_str::<TimerCommands>(&txt) {
                        let mut state = state.write().await;
                        match cmd {
                            TimerCommands::Start => {
                                println!("start");
                                state.running = true;
                                state.last_updated = std::time::Instant::now();
                            }
                            TimerCommands::Stop => {
                                println!("stop");
                                state.running = false;
                            }
                            TimerCommands::Reset => {
                                println!("reset");
                                state.running = false;
                                state.time_left = 0.0;
                                state.last_updated = std::time::Instant::now();
                            }
                            TimerCommands::SetTime { min, sec } => {
                                println!("set time");
                                state.time_left = (min * 60 + sec) as f64;
                                state.running = false;
                                state.last_updated = std::time::Instant::now();
                            }
                        }
                    } else {
                        println!("beep beep");
                        println!("{:?}", serde_json::from_str::<TimerCommands>(&txt))
                    }
                }
            }

            _ = interval.tick() => {
                let mut state = state.write().await; 
                let finished = state.update(); 

                let update = TimerResponse {
                    running: state.running, 
                    display: state.show(),
                    finished: finished,
                };

                if socket.send(Message::Text(serde_json::to_string(&update).unwrap()))
                    .await
                    .is_err() 
                {
                    break;
                }
            }
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
