mod api;
mod common;
mod udp;
mod ws;

use std::net::{SocketAddr, TcpListener};
use tokio::sync::broadcast;
use udp::UdpListener;
use ws::WsServer;
fn find_available_port(start_port: u16, max_attempts: u16) -> u16 {
    let mut ret = start_port;
    for port in start_port..start_port + max_attempts {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        match TcpListener::bind(addr) {
            Ok(_) => {
                // 포트를 즉시 닫아서 다른 프로세스가 사용할 수 있도록 함
                ret = port;
                break;
            }
            Err(_) => {
                // 포트가 이미 사용 중이면 다음 포트 시도
                continue;
            }
        }
    }
    return ret;
}

#[tokio::main]
async fn main() {
    // UDP <-> WS 양양방향 채널 생성
    let (udp_to_ws_tx, udp_to_ws_rx) = broadcast::channel(16);
    let (ws_to_udp_tx, ws_to_udp_rx) = broadcast::channel(16);

    // 5555 - 5565 까지 남는 포트 검색
    let start_port = 5555;
    let max_attempts = 10;
    let ws_port = find_available_port(start_port, max_attempts);
    if ws_port == start_port + max_attempts {
        println!("사용 가능한 포트를 찾지 못했습니다.");
        return;
    }

    // WS 서버
    let ws_addr: SocketAddr = format!("0.0.0.0:{}", ws_port).parse().unwrap();
    let ws_server = WsServer::new(ws_to_udp_tx, udp_to_ws_rx);
    let ws_handle = tokio::spawn(async move {
        ws_server.start(ws_addr).await;
    });

    // UDP 리스너
    // TODO: WS 안에서 처리하는 것으로 변경 (mut port)
    let udp_addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();
    let udp_listener = UdpListener::new(udp_addr, udp_to_ws_tx, ws_to_udp_rx)
        .await
        .expect("UDP 리스너 생성 실패");
    let udp_handle = tokio::spawn(async move {
        udp_listener.start().await;
    });

    println!("Start LiDAR Server");
    println!("UDP: {}, WS: {}", udp_addr, ws_addr);

    _ = tokio::join!(udp_handle, ws_handle);
}
