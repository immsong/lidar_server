use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

pub struct UdpListener {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    udp_to_api_tx: broadcast::Sender<Vec<u8>>,
    api_to_udp_rx: broadcast::Receiver<Vec<u8>>,
}

impl UdpListener {
    pub async fn new(
        addr: SocketAddr,
        udp_to_api_tx: broadcast::Sender<Vec<u8>>,
        api_to_udp_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
            addr,
            udp_to_api_tx,
            api_to_udp_rx,
        })
    }

    pub async fn start(&self) {
        let mut buf = vec![0u8; 65535];
        let api_to_udp_rx = self.api_to_udp_rx.resubscribe();

        // UDP 통신
        let recv_socket = Arc::clone(&self.socket);
        let udp_to_api_tx = self.udp_to_api_tx.clone();
        let recv_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((size, _src_addr)) => {
                        let data = buf[..size].to_vec();
                        if let Err(e) = udp_to_api_tx.send(data) {
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
        let mut rx = self.api_to_udp_rx.resubscribe();
        let tx = self.udp_to_api_tx.clone();
        let send_socket = Arc::clone(&self.socket);
        let addr = self.addr;
        let send_handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        // ================================
                        // 채널 통신 테스트
                        // ================================
                        // println!("API 데이터 수신: {:?}", String::from_utf8(data).unwrap());
                        // _ = tx.send(b"im udp listener".to_vec());
                        // ================================
                        
                        // if let Err(e) = send_socket.send_to(&data, addr).await {
                        //     eprintln!("Failed to send data: {}", e);
                        // }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive from API channel: {}", e);
                    }
                }
            }
        });

        // 두 태스크가 완료될 때까지 대기
        let _ = tokio::join!(recv_handle, send_handle);
    }
}
