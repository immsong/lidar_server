use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, ConnectInfo, State},
    response::Response,
    routing::get,
    Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, SinkExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tracing::*;
use uuid::Uuid;

pub struct WsServer {
    ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
    udp_to_ws_rx: broadcast::Receiver<Vec<u8>>,
    clients: Arc<Mutex<HashMap<Uuid, futures::stream::SplitSink<WebSocket, Message>>>>,
}

impl WsServer {
    pub fn new(
        ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
        udp_to_ws_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            ws_to_udp_tx,
            udp_to_ws_rx,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&self, addr: SocketAddr) {
        let state = Arc::new(AppState {
            ws_to_udp_tx: self.ws_to_udp_tx.clone(),
            clients: self.clients.clone(),
        });

        let state_clone = state.clone();
        let mut rx = self.udp_to_ws_rx.resubscribe();
        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        debug!(
                            "UDP -> WS data received: {:?}",
                            String::from_utf8(data.clone()).unwrap()
                        );

                        // response
                        if let Err(e) = state_clone.broadcast_message(data.clone()).await {
                            error!("Failed to broadcast message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive from UDP channel: {}", e);
                    }
                }
            }
        });

        let app = Router::new()
            .route("/ws", get(Self::handle_upgrade))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();

        handle.abort();
    }
    async fn handle_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
        ws.on_upgrade(|socket| async move { Self::handle_socket(socket, state).await })
    }

    async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
        let (sender, mut receiver) = socket.split();
        let client_id = Uuid::new_v4();

        // sender 저장
        {
            let mut clients = state.clients.lock().await;
            clients.insert(client_id, sender);
            info!("Client connected: {}", client_id);
        }

        let state_clone = state.clone();
        let ws_to_udp_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        info!("Text message received: {:?}", text);
                        _ = state_clone.ws_to_udp_tx.send(text.as_bytes().to_vec());

                        // response to all clients
                        _ = state_clone.broadcast_message(text.as_bytes().to_vec());
                    }
                    Message::Binary(data) => {
                        info!("Binary message received: {:?}", data);
                        _ = state_clone.ws_to_udp_tx.send(data.to_vec());

                        // response to all clients
                        _ = state_clone.broadcast_message(data.to_vec());
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });

        _ = tokio::join!(ws_to_udp_task);

        // 연결이 종료되면 sender 제거
        {
            let mut clients = state.clients.lock().await;
            clients.remove(&client_id);
            info!("Client disconnected: {}", client_id);
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
    pub clients: Arc<Mutex<HashMap<Uuid, futures::stream::SplitSink<WebSocket, Message>>>>,
}

impl AppState {
    pub async fn broadcast_message(&self, message: Vec<u8>) -> Result<(), String> {
        let mut clients = self.clients.lock().await;
        for (_, sender) in clients.iter_mut() {
            if let Err(e) = sender
                .send(Message::Binary(Bytes::from(message.clone())))
                .await
            {
                error!("Failed to send message: {}", e);
            }
        }
        Ok(())
    }
}
