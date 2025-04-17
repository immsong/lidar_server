use std::net::Ipv4Addr;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct LiDARKey {
    pub key: u64,
}

impl LiDARKey {
    pub fn new(ip: Ipv4Addr, port: u16) -> Self {
        Self {
            key: Self::create_key(ip, port),
        }
    }

    pub fn get_ip(&self) -> Ipv4Addr {
        let ip_bytes = [
            ((self.key >> 40) & 0xFF) as u8,
            ((self.key >> 32) & 0xFF) as u8,
            ((self.key >> 24) & 0xFF) as u8,
            ((self.key >> 16) & 0xFF) as u8,
        ];
        Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3])
    }

    pub fn get_port(&self) -> u16 {
        (self.key & 0xFFFF) as u16
    }

    fn create_key(ip: Ipv4Addr, port: u16) -> u64 {
        let ip_bytes = ip.octets();

        ((ip_bytes[0] as u64) << 40) |  // IP 첫 번째 옥텟
        ((ip_bytes[1] as u64) << 32) |  // IP 두 번째 옥텟
        ((ip_bytes[2] as u64) << 24) |  // IP 세 번째 옥텟
        ((ip_bytes[3] as u64) << 16) |  // IP 네 번째 옥텟
        (port as u64) // Port
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LiDARChannelData {
    pub key: LiDARKey,
    pub raw_data: Vec<u8>,
}

impl LiDARChannelData {
    pub fn new(key: LiDARKey, raw_data: Vec<u8>) -> Self {
        Self { key, raw_data }
    }
}

/// LiDAR 제조사 정보를 나타내는 열거형
///
/// # Variants
/// * `KanaviMobility` - Kanavi Mobility사의 LiDAR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompanyInfo {
    KanaviMobility = 0,
    Unknown,
}

impl TryFrom<u8> for CompanyInfo {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompanyInfo::KanaviMobility),
            _ => Ok(CompanyInfo::Unknown),
        }
    }
}

/// 3차원 공간의 한 점을 나타내는 구조체
///
/// # Fields
/// * `x` - X 좌표
/// * `y` - Y 좌표
/// * `z` - Z 좌표
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 포인트 클라우드 데이터를 나타내는 구조체
///
/// # Fields
/// * `points` - 3차원 공간의 점들의 집합
///
/// # Examples
/// ```rust
/// let mut cloud = PointCloud::new();
/// cloud.add_point(Point { x: 1.0, y: 2.0, z: 3.0 });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct PointCloud {
    pub points: Vec<Point>,
}

impl PointCloud {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    pub fn add_point(&mut self, point: Point) {
        self.points.push(point);
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

pub mod request_command {
    pub const GET: &str = "get";
    pub const SET: &str = "set";
}

// Request types
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestMessage {
    pub lidar_info: serde_json::Value,
    pub command: String, // request_command
    pub r#type: String,
    pub data: Option<serde_json::Value>,
}

impl RequestMessage {
    pub fn new() -> Self {
        Self {
            lidar_info: serde_json::Value::Null,
            command: "".to_string(),
            r#type: "".to_string(),
            data: None,
        }
    }
}

pub mod response_status {
    pub const SUCCESS: &str = "success";
    pub const ERROR: &str = "error";
    pub const NONE: &str = "none";
}

// Response types
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub lidar_info: serde_json::Value,
    pub status: String, // response_status
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl ResponseMessage {
    pub fn new() -> Self {
        Self {
            lidar_info: serde_json::Value::Null,
            status: "".to_string(),
            message: "".to_string(),
            data: None,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    pub fn from_json(json: serde_json::Value) -> Self {
        serde_json::from_value(json).unwrap()
    }
}

