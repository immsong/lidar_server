use serde_json::Value;
use std::{error::Error, net::Ipv4Addr};

#[derive(Debug)]
pub enum LiDARError {
    InvalidData(String),
}

impl std::fmt::Display for LiDARError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiDARError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl Error for LiDARError {}

pub trait UDPHandler: Send {
    fn parse(&mut self, ip: Ipv4Addr, port: u16, data: &[u8]) -> Result<Value, Box<LiDARError>>;
}

pub trait WsHandler: Send {
    async fn parse(&mut self, ip: Ipv4Addr, port: u16, data: Value) -> Result<(Value, Vec<u8>), Box<LiDARError>>;
}
