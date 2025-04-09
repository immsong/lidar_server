mod common;
mod lidar;
mod udp;
mod ws;

use std::net::{SocketAddr, TcpListener};
use tokio::sync::broadcast;
use tracing::*;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, fmt::writer::MakeWriterExt, EnvFilter};
use udp::UdpListener;
use ws::WsServer;

/// 사용 가능한 포트 찾기
///
/// # Examples
/// ```
/// let port: u16 = find_available_port(5555, 10)
/// ```
///
/// # Arguments
/// start_port: 시작 포트
/// max_attempts: 최대 시도 횟수, 시도 시 마다 start_port + 1 을 하여 시도
///
/// # Returns
/// 사용 가능한 포트 번호
fn find_available_port(start_port: u16, max_attempts: u16) -> u16 {
    let mut ret = start_port;
    for port in start_port..start_port + max_attempts {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        match TcpListener::bind(addr) {
            Ok(_) => {
                ret = port;
                break;
            }
            Err(_) => {
                // 열린 포트 존재
                continue;
            }
        }
    }
    return ret;
}

/// 로깅 시스템 초기화
///
/// # Examples
/// ```
/// setup_logger()
/// ```
///
/// # Arguments
/// 없음
///
/// # Returns
/// 없음
///
/// # 설정 내용
/// * 로그 파일: logs/lidar-server-YYYY-MM-DD.log
/// * 로그 레벨: Release 빌드일 때는 INFO 이상, Debug 빌드일 때는 TRACE 이상
/// * 포함 정보: 시간, 스레드 ID/이름, 파일 위치, 라인 번호
/// * 로그 출력: 터미널과 파일 모두에 출력
fn setup_logger() {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "lidar-server.log");

    // 환경에 따른 로그 레벨 설정
    let filter = if cfg!(debug_assertions) {
        // 디버그 빌드일 때는 모든 레벨 출력
        EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into())
    } else {
        // 릴리즈 빌드일 때는 INFO 이상만 출력
        EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender.and(std::io::stdout))
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .init();
}

/// LiDAR 서버 메인 함수
///
/// WebSocket 서버와 UDP 리스너를 동시에 실행하여 LiDAR 데이터를 중계
/// WebSocket은 클라이언트와의 통신을, UDP는 LiDAR 장치와의 통신을 담당
///
/// # 비동기 실행
/// `#[tokio::main]` 어트리뷰트를 사용하여 비동기 런타임에서 실행
/// WebSocket 서버와 UDP 리스너가 동시에 실행되며, 각각 독립적인 태스크로 관리
///
/// # 서버 구성
/// * WebSocket 서버: `0.0.0.0:5555` (포트 사용 중이면 자동으로 다음 포트 시도)
/// * UDP 리스너: `0.0.0.0:5000` (TODO: 클라이언트 설정에 따라 port 변경 가능)
///
/// # 통신 흐름
/// 1. LiDAR -> UDP -> WebSocket -> 클라이언트
/// 2. 클라이언트 -> WebSocket -> UDP -> LiDAR
///
/// # 채널 구성
/// * `udp_to_ws`: UDP에서 WebSocket으로의 데이터 전송 (tokio broadcast 채널)
/// * `ws_to_udp`: WebSocket에서 UDP로의 데이터 전송 (tokio broadcast 채널)
#[tokio::main]
async fn main() {
    setup_logger();
    info!("Start LiDAR Server!");

    // UDP <-> WS 양방향 채널 생성
    let (udp_to_ws_tx, udp_to_ws_rx) = tokio::sync::mpsc::channel(1);
    let (ws_to_udp_tx, ws_to_udp_rx) = tokio::sync::mpsc::channel(1);

    let start_port = 5555;
    let max_attempts = 10;
    let ws_port = find_available_port(start_port, max_attempts);
    if ws_port == start_port + max_attempts {
        error!("Failed to find available port");
        return;
    }

    let ws_addr: SocketAddr = format!("0.0.0.0:{}", ws_port).parse().unwrap();
    let mut ws_server = WsServer::new(ws_to_udp_tx, udp_to_ws_rx);
    let ws_handle = tokio::spawn(async move {
        ws_server.start(ws_addr).await;
    });

    let udp_addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();
    let mut udp_listener = match UdpListener::new(udp_addr, udp_to_ws_tx, ws_to_udp_rx).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to create UDP listener: {}", e);
            return;
        }
    };
    let udp_handle = tokio::spawn(async move {
        udp_listener.start().await;
    });

    info!("UDP: {:?}, WS: {:?}", udp_addr, ws_addr);
    _ = tokio::join!(udp_handle, ws_handle);
}
