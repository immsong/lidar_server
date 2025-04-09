use crate::lidar::kanavi_mobility::parser::KanaviMobilityParser;
use crate::lidar::kanavi_mobility::*;
use crate::lidar::{traits::*, CompanyInfo};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, Mutex};
use tracing::*;
/// UDP 리스너 구조체
///
/// # 구조체 필드
/// * `socket` - UDP 통신을 위한 소켓
/// * `addr` - 바인딩된 소켓 주소
/// * `udp_to_ws_tx` - UDP에서 WebSocket으로 데이터를 전송하는 채널 송신자
/// * `ws_to_udp_rx` - WebSocket에서 UDP로 데이터를 수신하는 채널 수신자
/// * `parsers` - LiDAR 회사별 파서를 저장하는 HashMap
///
/// # 주요 기능
/// * UDP 소켓을 통한 데이터 수신 및 WebSocket으로의 전달
/// * WebSocket으로부터 받은 데이터를 UDP로 전송
/// * LiDAR 데이터 파싱 및 처리
/// * 양방향 데이터 스트림의 관리 및 에러 처리
pub struct UdpListener {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    udp_to_ws_tx: broadcast::Sender<Vec<u8>>,
    ws_to_udp_rx: broadcast::Receiver<Vec<u8>>,
    parsers: Arc<Mutex<HashMap<CompanyInfo, Box<dyn LiDARParser>>>>,
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
    /// * `udp_to_ws_tx` - UDP에서 WebSocket으로 데이터를 전송하는 채널 송신자
    /// * `ws_to_udp_rx` - WebSocket에서 UDP로 데이터를 수신하는 채널 수신자
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
        udp_to_ws_tx: broadcast::Sender<Vec<u8>>,
        ws_to_udp_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let socket2 = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket2.set_reuse_address(true)?;
        socket2.bind(&addr.into())?;
        let socket = UdpSocket::from_std(socket2.into())?;

        // 모든 네트워크 인터페이스 가져오기
        let interfaces = NetworkInterface::show().unwrap();

        for interface in interfaces {
            let vec_addr = interface.addr;
            for addr in vec_addr {
                if let network_interface::Addr::V4(ipv4) = addr {
                    info!("Joining multicast on interface: {}", ipv4.ip);
                    let _ = socket.join_multicast_v4(Ipv4Addr::new(224, 0, 0, 5), ipv4.ip);
                }
            }
        }
        Ok(Self {
            socket: Arc::new(socket),
            addr,
            udp_to_ws_tx,
            ws_to_udp_rx,
            parsers: Arc::new(Mutex::new(HashMap::new())),
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
    ///     * LiDAR 데이터 파싱
    ///     * WebSocket으로 전달
    ///   - 채널 통신 태스크:
    ///     * WebSocket으로부터 받은 데이터를 처리
    ///     * UDP로 전송
    /// * 에러 발생 시 로깅 처리
    /// * 양방향 통신의 지속적인 모니터링 및 관리
    pub async fn start(&self) {
        let mut buf = vec![0u8; 65535];
        let ws_to_udp_rx = self.ws_to_udp_rx.resubscribe();

        // UDP 통신
        let recv_socket = Arc::clone(&self.socket);
        let udp_to_ws_tx = self.udp_to_ws_tx.clone();
        let prasers = self.parsers.clone();
        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((size, _src_addr)) => {
                        let data = buf[..size].to_vec();
                        let mut parser_guard = prasers.lock().await;
                        let mut parse_result: Result<Box<dyn LiDARData>, ()> = Err(());
                        // 추후 필요 시 회사 별 구분값에 따라 처리 필요
                        if true || data[0] == 0xFA || _src_addr.port() == 5000 {
                            parser_guard
                                .entry(CompanyInfo::KanaviMobility)
                                .or_insert_with(|| Box::new(KanaviMobilityParser::new()));
                            parse_result = parser_guard
                                .get_mut(&CompanyInfo::KanaviMobility)
                                .unwrap()
                                .parse(&data);
                        } else {
                            // 추후 필요 시 다른 회사 파서 추가 필요
                            error!("Unknown company");
                            parse_result = Err(());
                        }

                        if let Err(e) = parse_result {
                            error!("Failed to parse LiDAR data: {:?}", e);
                            continue;
                        }

                        let data = parse_result.unwrap();
                        match data.get_company_info() {
                            CompanyInfo::KanaviMobility => {
                                if let Some(kanavi_data) = data.get_data() {
                                    if let Some(config_data) = kanavi_data.downcast_ref::<KMConfigData>() {
                                        debug!("parse_result: {:?}", config_data);
                                    }
                                }
                            }
                            _ => {
                                error!("Unknown company");
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
        let mut rx = self.ws_to_udp_rx.resubscribe();
        let tx = self.udp_to_ws_tx.clone();
        let send_socket = Arc::clone(&self.socket);
        let addr = self.addr;
        let send_handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        debug!(
                            "WS -> UDP data received: {:?}",
                            String::from_utf8(data.clone()).unwrap()
                        );
                        debug!("UDP -> WS data send: {:?}", data);
                        tx.send(data.clone()).unwrap();
                    }
                    Err(e) => {
                        error!("Failed to receive from WS channel: {}", e);
                    }
                }
            }
        });

        // 두 태스크가 완료될 때까지 대기
        let _ = tokio::join!(recv_handle, send_handle);
    }
}
