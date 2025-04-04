use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::*;

pub struct UdpListener {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    udp_to_ws_tx: broadcast::Sender<Vec<u8>>,
    ws_to_udp_rx: broadcast::Receiver<Vec<u8>>,
}

impl UdpListener {
    pub async fn new(
        addr: SocketAddr,
        udp_to_ws_tx: broadcast::Sender<Vec<u8>>,
        ws_to_udp_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
            addr,
            udp_to_ws_tx,
            ws_to_udp_rx,
        })
    }

    pub async fn start(&self) {
        let mut buf = vec![0u8; 65535];
        let ws_to_udp_rx = self.ws_to_udp_rx.resubscribe();

        // UDP 통신
        let recv_socket = Arc::clone(&self.socket);
        let udp_to_ws_tx = self.udp_to_ws_tx.clone();
        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((size, _src_addr)) => {
                        let data = buf[..size].to_vec();
                        if let Err(e) = udp_to_ws_tx.send(data) {
                            eprintln!("Failed to broadcast data: {}", e);
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
                        debug!("WS -> UDP data received: {:?}", String::from_utf8(data.clone()).unwrap());
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
