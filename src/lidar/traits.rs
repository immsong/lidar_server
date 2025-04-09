use crate::lidar::types::*;
use std::{any::Any, net::Ipv4Addr};

/// LiDAR 데이터 파서 트레이트
///
/// # 주요 기능
/// * 바이트 데이터를 LiDAR 데이터 구조로 파싱
/// * 회사별 파서 구현을 위한 인터페이스 제공
///
/// # 구현 예시
/// ```rust
/// impl LiDARParser for KanaviMobilityParser {
///     fn parse(&mut self, data: &[u8]) -> Result<Box<dyn LiDARData>, ()> {
///         // 파싱 로직 구현
///     }
/// }
/// ```
pub trait LiDARParser: Send {
    /// 바이트 데이터를 파싱하여 LiDAR 데이터로 변환
    ///
    /// # Arguments
    /// * `data` - 파싱할 바이트 데이터
    ///
    /// # Returns
    /// * `Result<Box<dyn LiDARData>, ()>` - 성공 시 파싱된 데이터, 실패 시 에러
    fn parse(&mut self, ip: Ipv4Addr, data: &[u8]) -> Result<Box<dyn LiDARData>, ()>;
}

/// LiDAR 데이터 트레이트
///
/// # 주요 기능
/// * 원본 데이터 접근
/// * 회사 정보 제공
/// * 포인트 클라우드 데이터 접근
/// * 회사별 설정 데이터 접근
///
/// # 구현 예시
/// ```rust
/// impl LiDARData for KanaviMobilityData {
///     fn get_raw_data(&self) -> &[u8] {
///         &self.raw_data
///     }
///     // ... 다른 메서드 구현
/// }
/// ```
pub trait LiDARData: Send {
    /// 원본 바이트 데이터 반환
    ///
    /// # Returns
    /// * `&[u8]` - 원본 바이트 데이터 슬라이스
    fn get_raw_data(&self) -> &[u8];

    /// LiDAR 제조사 정보 반환
    ///
    /// # Returns
    /// * `CompanyInfo` - LiDAR 제조사 정보
    fn get_company_info(&self) -> CompanyInfo;

    /// 포인트 클라우드 데이터 반환
    ///
    /// # Returns
    /// * `&[PointCloud]` - 포인트 클라우드 데이터 슬라이스
    fn get_points(&self) -> &[PointCloud];

    /// 설정 데이터 반환
    ///
    /// # Returns
    /// * `Option<&dyn Any>` - 설정 데이터 (있는 경우)
    fn get_data(&self) -> Option<&dyn Any>;

    /// LiDAR 고유 키 반환
    ///
    /// # Returns
    /// * `u64` - LiDAR 키
    fn get_key(&self) -> u64;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct EmptyLiDARData;

impl LiDARData for EmptyLiDARData {
    fn get_raw_data(&self) -> &[u8] {
        &[]
    }

    fn get_points(&self) -> &[PointCloud] {
        &[]
    }

    fn get_company_info(&self) -> CompanyInfo {
        CompanyInfo::Unknown
    }
    
    fn get_data(&self) -> Option<&dyn Any> {
        None
    }
    
    fn get_key(&self) -> u64 {
        0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}