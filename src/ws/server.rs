use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use futures::{stream::StreamExt, SinkExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct WsServer {
    ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
    udp_to_ws_rx: broadcast::Receiver<Vec<u8>>,
}

impl WsServer {
    pub fn new(
        ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
        udp_to_ws_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            ws_to_udp_tx,
            udp_to_ws_rx,
        }
    }

    pub async fn start(&self, addr: SocketAddr) {
        let state = Arc::new(AppState {
            ws_to_udp_tx: self.ws_to_udp_tx.clone(),
        });
        let app = Router::new()
            .route("/ws", get(Self::handle_upgrade))
            .with_state(state);

        println!("WebSocket 서버 시작: {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();

        // UDP -> WebSocket 전송을 위한 채널 구독
        let mut udp_rx = self.udp_to_ws_rx.resubscribe();

        // UDP 수신 태스크
        let udp_to_ws_task = tokio::spawn(async move {
            while let Ok(data) = udp_rx.recv().await {
                // if let Err(e) = sender.send(Message::Binary(data)).await {
                //     eprintln!("WebSocket 전송 실패: {}", e);
                //     break;
                // }
            }
        });
    }

    async fn handle_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
        ws.on_upgrade(|socket| async move { Self::handle_socket(socket, state).await })
    }

    async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
        // 소켓을 송신자와 수신자로 분리
        let (mut sender, mut receiver) = socket.split();

        // WebSocket 수신 태스크
        let ws_to_udp_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        println!("텍스트 메시지 수신: {}", text);
                        // 텍스트 메시지 처리
                    }
                    Message::Binary(data) => {
                        println!("바이너리 메시지 수신: {:?}", data);
                        // if let Err(e) = state.ws_to_udp_tx.send(data.to_vec()) {
                        //     eprintln!("UDP 전송 실패: {}", e);
                        // }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });

        tokio::join!(ws_to_udp_task);
        println!("WebSocket 연결 종료");
    }
}

#[derive(Clone)]
pub struct AppState {
    pub ws_to_udp_tx: broadcast::Sender<Vec<u8>>,
}
