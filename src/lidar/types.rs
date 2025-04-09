use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

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
}
