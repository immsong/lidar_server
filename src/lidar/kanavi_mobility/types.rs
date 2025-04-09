use crate::lidar::traits::*;
use crate::lidar::types::*;
use std::any::Any;
use std::f32::consts::PI;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// 사용자 영역을 나타내는 구조체
///
/// # Fields
/// * `point_count` - 영역 내 포인트 개수
/// * `points` - 영역을 구성하는 3차원 점들
#[derive(Debug, Serialize, Deserialize)]
pub struct UserArea {
    point_count: u8,
    points: Vec<Point>,
}

impl UserArea {
    pub fn new(point_count: u8, points: Vec<Point>) -> Self {
        Self {
            point_count,
            points,
        }
    }

    pub fn from_bytes(point_count: u8, bytes: Vec<u8>) -> Self {
        let points = Self::parse_points(&bytes);
        Self::new(point_count, points)
    }

    fn parse_points(bytes: &[u8]) -> Vec<Point> {
        let mut result = Vec::new();
        for i in (0..bytes.len()).step_by(4) {
            let x = Self::parse_coordinate(&bytes[i..i + 2]);
            let y = Self::parse_coordinate(&bytes[i + 2..i + 4]);
            result.push(Point { x, y, z: 0.0 });
        }
        result
    }

    fn parse_coordinate(bytes: &[u8]) -> f32 {
        let mut value1 = bytes[0] as i8;
        let mut value2 = bytes[1] as i8;

        if (value1 >> 7) == 0x01 {
            value1 = ((bytes[0] ^ 0xFF) as i8 + 1) * -1;
        }
        if (value2 >> 7) == 0x01 {
            value2 = ((bytes[1] ^ 0xFF) as i8 + 1) * -1;
        }

        value1 as f32 + (value2 as f32 * 0.01)
    }
}

/// 기본 설정을 나타내는 구조체
///
/// # Fields
/// * `output_channel` - 출력 채널
/// * `self_check_active_state` - 자가 점검 활성 상태
/// * `pulse_active_state` - 펄스 활성 상태 (Active Low or Active High)
/// * `pulse_output_mode` - 펄스 출력 딜레이 시간, 물체 감지 후 몇 ms 후 output 출력 신호를 내보낼 것인지 설정
/// * `pulse_pin_mode` - 펄스 핀 모드
/// * `pulse_pin_channel` - 펄스 핀 채널
/// * `start_angle` - 시작 각도
/// * `finish_angle` - 종료 각도
/// * `min_distance` - 최소 거리
/// * `max_distance` - 최대 거리
/// * `object_size` - 객체 크기
/// * `area_count` - 사용자 영역 개수
/// * `areas` - 사용자 영역들
#[derive(Debug, Serialize, Deserialize)]
pub struct BasicConfig {
    output_channel: u8,
    self_check_active_state: u8,
    pulse_active_state: u8,
    pulse_output_mode: u8,
    pulse_pin_mode: u8,
    pulse_pin_channel: u8,
    start_angle: u16,
    finish_angle: u16,
    min_distance: u8,
    max_distance: u8,
    object_size: u8,
    area_count: u8,
    areas: Vec<UserArea>,
}

impl BasicConfig {
    pub fn new(
        output_channel: u8,
        self_check_active_state: u8,
        pulse_active_state: u8,
        pulse_output_mode: u8,
        pulse_pin_mode: u8,
        pulse_pin_channel: u8,
        start_angle: u16,
        finish_angle: u16,
        min_distance: u8,
        max_distance: u8,
        object_size: u8,
        area_count: u8,
        areas: Vec<UserArea>,
    ) -> Self {
        Self {
            output_channel,
            self_check_active_state,
            pulse_active_state,
            pulse_output_mode,
            pulse_pin_mode,
            pulse_pin_channel,
            start_angle,
            finish_angle,
            min_distance,
            max_distance,
            object_size,
            area_count,
            areas,
        }
    }
}

/// 버전 정보를 나타내는 구조체
///
/// # Fields
/// * `firmware_version` - 펌웨어 버전
/// * `hardware_version` - 하드웨어 버전
/// * `end_target` - 설치 목적
#[derive(Debug)]
pub struct VersionInfo {
    firmware_version: [u8; 3],
    hardware_version: [u8; 3],
    end_target: u8,
}

impl VersionInfo {
    pub fn new(firmware_version: [u8; 3], hardware_version: [u8; 3], end_target: u8) -> Self {
        Self {
            firmware_version,
            hardware_version,
            end_target,
        }
    }
}

/// 네트워크 소스 정보를 나타내는 구조체
///
/// # Fields
/// * `ip_address` - IP 주소
/// * `mac_address` - MAC 주소
/// * `subnet_mask` - 서브넷 마스크
/// * `gateway` - 게이트웨이
/// * `port` - 포트 번호
#[derive(Debug)]
pub struct NetworkSourceInfo {
    ip_address: [u8; 4],
    mac_address: [u8; 6],
    subnet_mask: [u8; 4],
    gateway: [u8; 4],
    port: u16,
}

impl NetworkSourceInfo {
    pub fn new(
        ip_address: [u8; 4],
        mac_address: [u8; 6],
        subnet_mask: [u8; 4],
        gateway: [u8; 4],
        port: u16,
    ) -> Self {
        Self {
            ip_address,
            mac_address,
            subnet_mask,
            gateway,
            port,
        }
    }
}

/// 티칭 영역을 나타내는 구조체
///
/// # Fields
/// * `is_set` - 티칭 영역 설정 여부
/// * `points` - 티칭 영역을 구성하는 3차원 점들
#[derive(Debug)]
pub struct TeachingArea {
    is_set: u8,
    points: Vec<Vec<Point>>,
}

impl TeachingArea {
    pub fn new(is_set: u8, points: Vec<Vec<Point>>) -> Self {
        Self { is_set, points }
    }

    pub fn parse(product_line: u8, is_set: u8, raw_points: Vec<u8>) -> Self {
        let points = if is_set == 1 {
            Self::parse_points(product_line, raw_points)
        } else {
            Vec::new()
        };

        Self::new(is_set, points)
    }

    pub fn parse_points(product_line: u8, points: Vec<u8>) -> Vec<Vec<Point>> {
        let mut result_points = Vec::new();
        let mut fov_list: Vec<f32> = vec![-1.07, 0.0, 1.07, 2.14];
        let h_fov_resol = 0.25;
        let mut h_fov = 100.0;
        match product_line {
            2 | 3 => {
                fov_list = vec![0.0, 3.0];
                h_fov = 120.0;
            }
            7 => {
                fov_list = vec![0.0];
                h_fov = 270.0;
            }
            _ => {}
        }

        let mut distance: Vec<f32> = Vec::new();
        for i in (0..points.len() as usize).step_by(2) {
            distance.push(points[i] as f32 + points[i + 1] as f32 * 0.01);
        }

        for v_angle in fov_list.clone() {
            let mut fov_points = Vec::new();
            for h_angle_idx in 0..(h_fov / h_fov_resol) as usize {
                let mut point = Point {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };

                let idx = fov_list.iter().position(|&x| x == v_angle).unwrap() as f32
                    * (h_fov / h_fov_resol)
                    + h_angle_idx as f32;

                let dist = distance[idx as usize];
                let h = (PI * v_angle / 180.0).cos() * dist;
                point.z = (PI * v_angle / 180.0).tan() * h;

                let h_angle = (h_angle_idx as f32 * h_fov_resol) + ((180.0 - h_fov) / 2.0);
                point.x = (PI * h_angle / 180.0).cos() * h;
                point.y = (PI * h_angle / 180.0).tan() * point.x;

                fov_points.push(point);
            }

            result_points.push(fov_points);
        }

        result_points
    }
}

/// 네트워크 목적지 IP를 나타내는 구조체
///
/// # Fields
/// * `ip_address` - IP 주소
#[derive(Debug)]
pub struct NetworkDestinationIP {
    ip_address: [u8; 4],
}

impl NetworkDestinationIP {
    pub fn new(ip_address: [u8; 4]) -> Self {
        Self { ip_address }
    }
}

/// 모터 속도를 나타내는 구조체
///
/// # Fields
/// * `speed` - 모터 속도
#[derive(Debug)]
pub struct MotorSpeed {
    speed: u8,
}

impl MotorSpeed {
    pub fn new(speed: u8) -> Self {
        Self { speed }
    }
}

/// 경고 영역을 나타내는 구조체
///
/// # Fields
/// * `danger_area` - 위험 영역
/// * `warning_area` - 경고 영역
/// * `caution_area` - 주의 영역
#[derive(Debug)]
pub struct WarningArea {
    danger_area: [u8; 2],
    warning_area: [u8; 2],
    caution_area: [u8; 2],
}

impl WarningArea {
    pub fn new(danger_area: [u8; 2], warning_area: [u8; 2], caution_area: [u8; 2]) -> Self {
        Self {
            danger_area,
            warning_area,
            caution_area,
        }
    }
}

/// 안개 필터를 나타내는 구조체
///
/// # Fields
/// * `filter_value` - 필터 값
#[derive(Debug)]
pub struct FogFilter {
    filter_value: u8,
}

impl FogFilter {
    pub fn new(filter_value: u8) -> Self {
        Self { filter_value }
    }
}

/// 오감지 필터를 나타내는 구조체
///
/// # Fields
/// * `filter_value` - 필터 값
#[derive(Debug)]
pub struct RadiusFilter {
    filter_value: u8,
}

impl RadiusFilter {
    pub fn new(filter_value: u8) -> Self {
        Self { filter_value }
    }
}

/// 최대 오감지 필터 거리를 나타내는 구조체
///
/// # Fields
/// * `max_distance` - 최대 거리
#[derive(Debug)]
pub struct RadiusFilterMaxDistance {
    max_distance: u8,
}

impl RadiusFilterMaxDistance {
    pub fn new(max_distance: u8) -> Self {
        Self { max_distance }
    }
}

/// 창 오염 감지 모드를 나타내는 구조체
///
/// # Fields
/// * `mode` - 모드
#[derive(Debug)]
pub struct WindowContaminationDetectionMode {
    mode: u8,
}

impl WindowContaminationDetectionMode {
    pub fn new(mode: u8) -> Self {
        Self { mode }
    }
}

/// 티칭 모드를 나타내는 구조체
///
/// # Fields
/// * `range` - 범위
/// * `margin` - 마진
#[derive(Debug)]
pub struct TeachingMode {
    range: u8,
    margin: u8,
}

impl TeachingMode {
    pub fn new(range: u8, margin: u8) -> Self {
        Self { range, margin }
    }
}

/// 최소 오감지 필터 거리를 나타내는 구조체
///
/// # Fields
/// * `min_distance` - 최소 거리
#[derive(Debug)]
pub struct RadiusFilterMinDistance {
    min_distance: u8,
}

impl RadiusFilterMinDistance {
    pub fn new(min_distance: u8) -> Self {
        Self { min_distance }
    }
}

/// Kanavi Mobility LiDAR 설정 데이터 열거형
///
/// # Variants
/// * `BasicConfig` - 기본 설정
/// * `VersionInfo` - 버전 정보
/// * `NetworkSourceInfo` - 네트워크 소스 정보
/// * `TeachingArea` - 티칭 영역
/// * `NetworkDestinationIP` - 네트워크 목적지 IP
/// * `MotorSpeed` - 모터 속도
/// * `WarningArea` - 경고 영역
/// * `FogFilter` - 안개 필터
/// * `RadiusFilter` - 오감지 필터
/// * `RadiusFilterMaxDistance` - 최대 오감지 필터 거리
/// * `WindowContaminationDetectionMode` - 창 오염 감지 모드
/// * `TeachingMode` - 티칭 모드
/// * `RadiusFilterMinDistance` - 최소 오감지 필터 거리
/// * `Ack` - 정상 응답
/// * `Nak` - 비정상 응답
#[derive(Debug)]
pub enum KMConfigData {
    BasicConfig(BasicConfig),
    VersionInfo(VersionInfo),
    NetworkSourceInfo(NetworkSourceInfo),
    TeachingArea(TeachingArea),
    NetworkDestinationIP(NetworkDestinationIP),
    MotorSpeed(MotorSpeed),
    WarningArea(WarningArea),
    FogFilter(FogFilter),
    RadiusFilter(RadiusFilter),
    RadiusFilterMaxDistance(RadiusFilterMaxDistance),
    WindowContaminationDetectionMode(WindowContaminationDetectionMode),
    TeachingMode(TeachingMode),
    RadiusFilterMinDistance(RadiusFilterMinDistance),
    Ack(u8),
    Nak(u8),
}

/// Kanavi Mobility LiDAR 데이터 구조체
///
/// # Fields
/// * `raw_data` - 원본 바이트 데이터
/// * `points` - 포인트 클라우드 데이터
/// * `ip` - LiDAR의 IP 주소
/// * `product_line` - 제품 라인
/// * `lidar_id` - LiDAR ID
/// * `mode` - 모드
/// * `param` - 파라미터
/// * `data` - 설정 데이터
#[derive(Debug)]
pub struct KanaviMobilityData {
    // 공통 데이터
    raw_data: Vec<u8>,
    points: Vec<PointCloud>,

    // Kanavi Mobility 데이터
    ip: Ipv4Addr,
    product_line: u8,
    lidar_id: u8,
    mode: u8,
    param: u8,
    data: Option<KMConfigData>,
}

impl KanaviMobilityData {
    pub fn new(
        raw_data: Vec<u8>,
        product_line: u8,
        lidar_id: u8,
        mode: u8,
        param: u8,
        ip: Ipv4Addr,
    ) -> Self {
        Self {
            raw_data,
            points: Vec::new(),
            ip,
            product_line,
            lidar_id,
            mode,
            param,
            data: None,
        }
    }

    pub fn set_points(&mut self, ch: u8, points: PointCloud) {
        while self.points.len() <= ch as usize {
            self.points.push(PointCloud { points: Vec::new() });
        }

        self.points[ch as usize] = points;
    }

    pub fn set_data(&mut self, data: KMConfigData) {
        self.data = Some(data);
    }
}

impl LiDARData for KanaviMobilityData {
    fn get_raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    fn get_company_info(&self) -> CompanyInfo {
        CompanyInfo::KanaviMobility
    }

    fn get_points(&self) -> &[PointCloud] {
        &self.points
    }

    fn get_data(&self) -> Option<&dyn Any> {
        self.data.as_ref().map(|data| data as &dyn Any)
    }

    fn get_key(&self) -> u64 {
        let octets = self.ip.octets();
        // IP의 4바이트를 u32로 변환하고, id를 상위 8비트에 배치
        ((self.lidar_id as u64) << 32)
            | ((octets[0] as u64) << 24)
            | ((octets[1] as u64) << 16)
            | ((octets[2] as u64) << 8)
            | (octets[3] as u64)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
