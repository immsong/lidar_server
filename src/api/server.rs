use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct ApiServer {
    router: Router,
    udp_to_api_rx: broadcast::Receiver<Vec<u8>>,
    api_to_udp_tx: broadcast::Sender<Vec<u8>>,
}

impl ApiServer {
    pub fn new(
        udp_to_api_rx: broadcast::Receiver<Vec<u8>>,
        api_to_udp_tx: broadcast::Sender<Vec<u8>>,
    ) -> Self {
        let state = Arc::new(AppState {
            api_to_udp_tx: api_to_udp_tx.clone(),
        });

        let router = Router::new()
            .route("/data", get(get_data))
            .route("/command", post(send_command))
            .with_state(state);

        Self {
            router,
            udp_to_api_rx,
            api_to_udp_tx,
        }
    }

    pub async fn start(&self, addr: std::net::SocketAddr) {
        println!("API 서버 시작: {}", addr);

        // UDP 통신 태스크 시작
        let mut rx = self.udp_to_api_rx.resubscribe();
        let udp_handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        println!("UDP 데이터 수신: {:?}", String::from_utf8(data).unwrap());
                        // 여기서 데이터 처리
                    }
                    Err(e) => {
                        eprintln!("UDP 데이터 수신 실패: {}", e);
                    }
                }
            }
        });

        // HTTP 서버 시작
        let router = self.router.clone();
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        // ================================
        // 채널 통신 테스트
        // ================================
        // let tx = self.api_to_udp_tx.clone();
        // _ = tokio::spawn(async move {
        //     tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        //     loop {
        //         _ = tx.send(b"im api server".to_vec());
        //         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        //     }
        // });
        // ================================

        // 두 태스크가 완료될 때까지 대기
        let _ = tokio::join!(udp_handle, server_handle);
    }
}

#[derive(Clone)]
struct AppState {
    api_to_udp_tx: broadcast::Sender<Vec<u8>>,
}

async fn get_data(State(state): State<Arc<AppState>>) -> Json<Value> {
    println!("get_data api request");
    Json(serde_json::json!({
        "status": "success",
    }))
}

async fn send_command(State(state): State<Arc<AppState>>) -> Json<Value> {
    println!("send_command api request");
    Json(serde_json::json!({
        "status": "success",
    }))
}
