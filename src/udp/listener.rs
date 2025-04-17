use crate::lidar::{LiDARChannelData, LiDARKey};
use bincode::config::standard;
use bincode::{decode_from_slice, encode_into_slice};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::*;

/// UDP 리스너 구조체
///
/// # 구조체 필드
/// * `socket` - UDP 통신을 위한 소켓
/// * `addr` - 바인딩된 소켓 주소
/// * `udp_to_ws_tx` - UDP에서 WebSocket으로 데이터를 전송하는 mpsc 채널 송신자
/// * `ws_to_udp_rx` - WebSocket에서 UDP로 데이터를 수신하는 mpsc 채널 수신자
/// * `channel_data` - LiDAR UDP 데이터를 저장하는 HashMap
///
/// # 주요 기능
/// * UDP 소켓을 통한 데이터 수신 및 WebSocket으로의 전달
/// * WebSocket으로부터 받은 데이터를 UDP로 전송
/// * 실제 데이터 파싱 등 처리는 WebSocket 서버에서 수행
pub struct UdpListener {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    udp_to_ws_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ws_to_udp_rx: Option<tokio::sync::mpsc::Receiver<Vec<u8>>>,
    channel_data: Arc<Mutex<HashMap<LiDARKey, LiDARChannelData>>>,
}

impl UdpListener {
    /// 새로운 UDP 리스너 인스턴스를 생성하는 비동기 함수
    ///
    /// # Examples
    /// ```
    /// let udp_addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();
    /// let udp_listener = UdpListener::new(udp_addr, udp_to_ws_tx, ws_to_udp_rx).await?;
    /// ```
    ///
    /// # Arguments
    /// * `addr` - 바인딩할 소켓 주소
    /// * `udp_to_ws_tx` - UDP에서 WebSocket으로 데이터를 전송하는 mpsc 채널 송신자
    /// * `ws_to_udp_rx` - WebSocket에서 UDP로 데이터를 수신하는 mpsc 채널 수신자
    ///
    /// # Returns
    /// * `Result<Self, std::io::Error>` - 성공 시 UdpListener 인스턴스, 실패 시 IO 에러
    ///
    /// # 동작 설명
    /// * 지정된 주소에 UDP 소켓을 바인딩
    /// * 멀티캐스트 그룹 가입
    /// * 소켓과 채널들을 포함하는 UdpListener 인스턴스 생성
    pub async fn new(
        addr: SocketAddr,
        udp_to_ws_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
        ws_to_udp_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let udp_socket = UdpSocket::bind(addr).await?;

        // SO_REUSEADDR 및 SO_REUSEPORT 설정
        let socket2_socket = socket2::Socket::from(udp_socket.into_std()?);
        socket2_socket.set_reuse_address(true)?;
        socket2_socket.set_reuse_port(true)?;
        let socket = UdpSocket::from_std(socket2_socket.into())?;

        // 멀티캐스트 설정
        let interfaces = NetworkInterface::show().unwrap();
        for interface in interfaces {
            if let Some(network_interface::Addr::V4(ipv4)) = interface
                .addr
                .iter()
                .find(|addr| matches!(addr, network_interface::Addr::V4(_)))
            {
                info!("Joining multicast on interface: {}", ipv4.ip);
                let _ = socket.join_multicast_v4(Ipv4Addr::new(224, 0, 0, 5), ipv4.ip);
            }
        }

        Ok(Self {
            socket: Arc::new(socket),
            addr,
            udp_to_ws_tx,
            ws_to_udp_rx: Some(ws_to_udp_rx),
            channel_data: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// UDP 리스너의 메인 실행 함수
    ///
    /// # Examples
    /// ```
    /// let udp_listener = UdpListener::new(udp_addr, udp_to_ws_tx, ws_to_udp_rx).await?;
    /// udp_listener.start().await;
    /// ```
    ///
    /// # 동작 설명
    /// * 두 개의 비동기 태스크를 생성하여 실행:
    ///   - UDP 수신 태스크:
    ///     * UDP 소켓으로부터 데이터를 수신
    ///     * 원하는 데이터 크기까지 데이터를 수신
    ///     * WebSocket으로 전달
    ///   - 채널 통신 태스크:
    ///     * WebSocket으로부터 받은 데이터를 처리
    ///     * UDP로 전송
    /// * 에러 발생 시 로깅 처리
    /// * 양방향 통신의 지속적인 모니터링 및 관리
    pub async fn start(&mut self) {
        // UDP 통신
        let recv_socket = Arc::clone(&self.socket);
        let udp_to_ws_tx = self.udp_to_ws_tx.clone();
        let channel_data_arc = Arc::clone(&self.channel_data);
        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];

            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((size, _src_addr)) => {
                        let data = buf[..size].to_vec();
                        let ip = if let SocketAddr::V4(addr) = _src_addr {
                            *addr.ip()
                        } else {
                            Ipv4Addr::new(0, 0, 0, 0)
                        };
                        let key = LiDARKey::new(ip, _src_addr.port());
                        let mut channel_data_guard = channel_data_arc.lock().await;
                        channel_data_guard
                            .entry(key)
                            .and_modify(|value| {
                                value.raw_data.extend_from_slice(&data);
                            })
                            .or_insert_with(|| LiDARChannelData::new(key, data));

                        if let Some(channel_data) = channel_data_guard.get_mut(&key) {
                            if channel_data.raw_data.is_empty() {
                                error!("empty data");
                                continue;
                            }

                            // KanaviMobility
                            if channel_data.raw_data[0] == 0xFA {
                                if channel_data.raw_data.len() < 7 {
                                    error!("not enough minimum data");
                                    continue;
                                }

                                let data_len = (channel_data.raw_data[5] as u16) << 8
                                    | channel_data.raw_data[6] as u16;
                                let total_len = data_len as usize + 7 + 1;
                                if channel_data.raw_data.len() < total_len {
                                    // debug!("not enough data");
                                    continue;
                                }

                                if channel_data.raw_data.len() > total_len {
                                    error!("too much data");
                                    channel_data.raw_data.clear();
                                    continue;
                                }

                                // println!(
                                //     "ip: {:?}, port: {:?}, len: {:?}",
                                //     ip,
                                //     _src_addr.port(),
                                //     channel_data.raw_data.len()
                                // );

                                let mut encoded_data: Vec<u8> = vec![0u8; 4096];
                                let size = encode_into_slice(
                                    &channel_data.clone(),
                                    &mut encoded_data,
                                    standard(),
                                )
                                .unwrap();
                                let encoded_data = &encoded_data[..size];
                                let _ = udp_to_ws_tx.send(encoded_data.to_vec()).await;

                                channel_data.raw_data.clear();
                            // } else if channel_data.raw_data[0] == 0x?? { // Other Comapny
                            } else {
                                channel_data.raw_data.clear();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive data: {}", e);
                    }
                }
            }
        });

        // Channel 통신
        let mut rx = self.ws_to_udp_rx.take().unwrap();
        let tx = self.udp_to_ws_tx.clone();
        let send_socket = Arc::clone(&self.socket);
        let send_handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(data) => {
                        match decode_from_slice::<LiDARChannelData, _>(&data, standard()) {
                            Ok((lidar_channel_data, _)) => {
                                let ip = lidar_channel_data.key.get_ip();
                                let port = lidar_channel_data.key.get_port();
                                println!("send to: {:?}, {:?}", ip, port);
                                let _ret = send_socket
                                    .send_to(&lidar_channel_data.raw_data, SocketAddr::new(IpAddr::V4("224.0.0.5".parse().unwrap()), port))
                                    .await;

                                println!("send result: {:?}", _ret);
                            }
                            Err(e) => {
                                error!("Failed to decode LiDARChannelData: {}", e);
                            }
                        }
                    }
                    None => {
                        error!("Channel closed");
                    }
                }
            }
        });

        // 두 태스크가 완료될 때까지 대기
        let _ = tokio::join!(recv_handle, send_handle);
    }
}
