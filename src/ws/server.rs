use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use bincode::decode_from_slice;
use bincode::{config::standard, encode_into_slice};
use bytes::Bytes;
use core::borrow;
use futures::{stream::StreamExt, SinkExt};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
};
use tokio::sync::{broadcast, Mutex};
use tracing::*;
use uuid::Uuid;

use crate::lidar::{
    kanavi_mobility::{request_types, KanaviMobilityWsHandler, KanaviUDPHandler, LiDARInfo},
    response_status, LiDARChannelData, LiDARKey, RequestMessage, ResponseMessage, UDPHandler,
    WsHandler,
};

/// WebSocket 서버 구조체
///
/// # Examples
/// ```
/// let ws_addr: SocketAddr = format!("0.0.0.0:{}", 5555).parse().unwrap();
/// let ws_server = WsServer::new(ws_to_udp_tx, udp_to_ws_rx);
/// ws_server.start(ws_addr).await;
/// ```
///
/// # Arguments
/// * `ws_to_udp_tx` - WebSocket에서 UDP로 메시지를 전송하는 mpsc 채널 송신자
/// * `udp_to_ws_rx` - UDP에서 WebSocket으로 메시지를 수신하는 mpsc 채널 수신자
/// * `clients` - 연결된 WebSocket 클라이언트들의 HashMap
///
/// # 주요 기능
/// * WebSocket 클라이언트 연결 관리
/// * UDP와 WebSocket 간의 메시지 중계
/// * LiDAR 데이터 파싱 및 처리
/// * 클라이언트 간 메시지 브로드캐스트
pub struct WsServer {
    ws_to_udp_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    udp_to_ws_rx: Option<tokio::sync::mpsc::Receiver<Vec<u8>>>,
    clients: Arc<Mutex<HashMap<Uuid, futures::stream::SplitSink<WebSocket, Message>>>>,
    client_lidar_map: Arc<Mutex<HashMap<Uuid, LiDARInfo>>>,
    lidar_infos: Arc<Mutex<HashSet<LiDARInfo>>>,
}

impl WsServer {
    /// 새로운 WebSocket 서버 인스턴스 생성
    ///
    /// # Examples
    /// ```
    /// let server = WsServer::new(tx, rx);
    /// ```
    ///
    /// # Arguments
    /// * `ws_to_udp_tx` - WebSocket에서 UDP로의 송신 채널
    /// * `udp_to_ws_rx` - UDP에서 WebSocket으로의 수신 채널
    ///
    /// # Returns
    /// * `Self` - 새로운 WsServer 인스턴스
    pub fn new(
        ws_to_udp_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
        udp_to_ws_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            ws_to_udp_tx,
            udp_to_ws_rx: Some(udp_to_ws_rx),
            clients: Arc::new(Mutex::new(HashMap::new())),
            client_lidar_map: Arc::new(Mutex::new(HashMap::new())),
            lidar_infos: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// WebSocket 서버 시작
    ///
    /// # Examples
    /// ```
    /// let ws_addr: SocketAddr = format!("0.0.0.0:{}", 5555).parse().unwrap();
    /// server.start(ws_addr).await;
    /// ```
    ///
    /// # Arguments
    /// * `addr` - 서버를 바인딩할 소켓 주소
    ///
    /// # Returns
    /// 없음
    ///
    /// # 동작 설명
    /// * WebSocket 엔드포인트(/ws) 설정
    /// * UDP 메시지 수신 및 처리
    /// * 클라이언트 연결 관리
    pub async fn start(&mut self, addr: SocketAddr) {
        let state = Arc::new(AppState {
            ws_to_udp_tx: self.ws_to_udp_tx.clone(),
            clients: self.clients.clone(),
            client_lidar_map: self.client_lidar_map.clone(),
            lidar_infos: self.lidar_infos.clone(),
        });

        let state_clone = state.clone();
        let mut rx = self.udp_to_ws_rx.take().unwrap();
        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(data) => {
                        let mut res = ResponseMessage::new();
                        match decode_from_slice::<LiDARChannelData, _>(&data, standard()) {
                            Ok((lidar_channel_data, _)) => {
                                let ip = lidar_channel_data.key.get_ip();
                                let port = lidar_channel_data.key.get_port();
                                match KanaviUDPHandler.parse(ip, port, &lidar_channel_data.raw_data)
                                {
                                    Ok(json) => {
                                        if json["status"].to_string() != response_status::NONE {
                                            res.status = response_status::SUCCESS.to_string();
                                            res = ResponseMessage::from_json(json);
                                        }

                                        state_clone.lidar_infos.lock().await.insert(
                                            serde_json::from_value::<LiDARInfo>(
                                                res.lidar_info.clone(),
                                            )
                                            .unwrap(),
                                        );
                                    }
                                    Err(e) => {
                                        res.status = response_status::ERROR.to_string();
                                        res.message = e.to_string();
                                        error!("Failed to parse LiDAR data: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                res.status = response_status::ERROR.to_string();
                                res.message = e.to_string();
                                error!("Failed to decode LiDAR data: {:?}", e);
                            }
                        }

                        let _ = state_clone.broadcast_message(res.to_json()).await;
                    }
                    None => {
                        error!("Failed to receive from UDP channel");
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

    /// WebSocket 엔드포인트(/ws) 업그레이드 처리
    ///
    /// # Arguments
    /// * `ws` - WebSocketUpgrade 타입의 인자
    /// * `state` - AppState 타입의 인자
    ///
    /// # Returns
    /// * `Response` - 업그레이드된 WebSocket 연결
    ///
    /// # 동작 설명
    /// * WebSocket 연결 업그레이드
    /// * 연결 처리 위임
    ///
    /// 참고: 이 함수는 Axum 라우터에 의해 자동으로 호출되며, 직접 호출하지 않습니다.
    /// ```
    /// let app = Router::new()
    ///     .route("/ws", get(Self::handle_upgrade))
    ///     .with_state(state);
    /// ```
    async fn handle_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
        ws.on_upgrade(|socket| async move { Self::handle_socket(socket, state).await })
    }

    /// WebSocket 연결을 처리하는 비동기 함수
    ///
    /// # Arguments
    /// * `socket` - 업그레이드된 WebSocket 연결
    /// * `state` - 애플리케이션 상태를 포함하는 Arc<AppState>
    ///
    /// # 동작 설명
    /// * 클라이언트 연결 시 고유 UUID 할당
    /// * WebSocket 스트림을 sender와 receiver로 분리
    /// * 클라이언트의 sender를 상태에 저장
    /// * 메시지 수신 처리:
    ///   - Text 메시지: UDP로 전달 및 모든 클라이언트에게 브로드캐스트
    ///   - Binary 메시지: UDP로 전달 및 모든 클라이언트에게 브로드캐스트
    ///   - Close 메시지: 연결 종료
    /// * 연결 종료 시 클라이언트 정리
    ///
    /// 참고: 이 함수는 handle_upgrade 함수에 의해 호출되며, WebSocket 연결의 전체 생명주기를 관리합니다.
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
                let mut txt_msg = String::new();

                match msg {
                    Message::Text(text) => {
                        txt_msg = text.to_string();
                    }
                    Message::Binary(data) => {
                        txt_msg = String::from_utf8(data.to_vec()).unwrap();
                    }
                    Message::Close(_) => break,
                    _ => {}
                }

                match serde_json::from_str::<serde_json::Value>(&txt_msg) {
                    Ok(json) => {
                        let ip = Ipv4Addr::new(0, 0, 0, 0);
                        let port = 5555;
                        let mut ws_handler =
                            KanaviMobilityWsHandler::new(state_clone.clone(), client_id);
                        if let Ok(ret) = ws_handler.parse(ip, port, json).await {
                            if let Ok(res) = serde_json::from_value::<ResponseMessage>(ret.0.clone()) {
                                if res.status.to_string() != response_status::NONE {
                                    _ = state_clone.send_message(client_id, ret.0).await;
                                }

                                if let Ok(lidar_info) =
                                    serde_json::from_value::<LiDARInfo>(res.lidar_info.clone())
                                {
                                    if ret.1.len() > 0 {
                                        // make channel data
                                        let channel_data = LiDARChannelData::new(
                                            LiDARKey::new(
                                                lidar_info.ip.parse::<Ipv4Addr>().unwrap(),
                                                lidar_info.port,
                                            ),
                                            ret.1,
                                        );

                                        let mut encoded_data: Vec<u8> = vec![0u8; 4096];
                                        let size = encode_into_slice(
                                            &channel_data.clone(),
                                            &mut encoded_data,
                                            standard(),
                                        )
                                        .unwrap();
                                        let encoded_data = &encoded_data[..size];
                                        _ = state_clone
                                            .ws_to_udp_tx
                                            .send(encoded_data.to_vec())
                                            .await;
                                    }
                                } else {
                                    error!("Failed to send message: {:?}", res.to_json());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse JSON: {}", e);
                        // 에러 처리
                    }
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

/// 애플리케이션 상태 구조체
///
/// # Examples
/// ```
/// let state = AppState {
///     ws_to_udp_tx: tx,
///     clients: Arc::new(Mutex::new(HashMap::new()))
/// };
/// ```
///
/// # Arguments
/// * `ws_to_udp_tx` - WebSocket에서 UDP로의 mpsc 송신 채널
/// * `clients` - 연결된 클라이언트들의 HashMap
///
/// # 주요 기능
/// * 클라이언트 상태 관리
/// * 메시지 브로드캐스트
#[derive(Clone)]
pub struct AppState {
    pub ws_to_udp_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub clients: Arc<Mutex<HashMap<Uuid, futures::stream::SplitSink<WebSocket, Message>>>>,
    pub client_lidar_map: Arc<Mutex<HashMap<Uuid, LiDARInfo>>>,
    pub lidar_infos: Arc<Mutex<HashSet<LiDARInfo>>>,
}

impl AppState {
    /// 모든 연결된 클라이언트에게 메시지 브로드캐스트
    ///
    /// # Examples
    /// ```
    /// ```
    /// state.broadcast_message(message).await?;
    ///
    /// # Arguments
    /// * `message` - 브로드캐스트할 바이너리 메시지
    ///
    /// # Returns
    /// * `Result<(), String>` - 성공 시 Ok(()), 실패 시 에러 메시지
    ///
    /// # 동작 설명
    /// * 모든 클라이언트에게 동일한 메시지 전송
    /// * 전송 실패 시 에러 로깅
    pub async fn broadcast_message(&self, message: serde_json::Value) -> Result<(), String> {
        let mut clients = self.clients.lock().await;
        let client_lidar_map = self.client_lidar_map.lock().await;

        for (client, sender) in clients.iter_mut() {
            if let Some(lidar_info_from_map) = client_lidar_map.get(client) {
                if lidar_info_from_map.to_json().to_string() == message["lidar_info"].to_string() {
                    if let Err(e) = sender.send(Message::Text(message.to_string().into())).await {
                        error!("Failed to send message: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn send_message(&self, uuid: Uuid, message: serde_json::Value) -> Result<(), String> {
        let mut clients = self.clients.lock().await;

        if let Some(sender) = clients.get_mut(&uuid) {
            if let Err(e) = sender.send(Message::Text(message.to_string().into())).await {
                error!("Failed to send message: {}", e);
            }
        }
        Ok(())
    }
}
