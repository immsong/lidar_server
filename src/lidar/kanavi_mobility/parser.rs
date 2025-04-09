use std::any::Any;
use std::f32::consts::PI;

use crate::lidar::kanavi_mobility::types::*;
use crate::lidar::traits::*;
use crate::lidar::types::*;
use tracing::*;

/// Kanavi Mobility LiDAR 데이터 파서
///
/// # 주요 기능
/// * 바이트 데이터를 LiDAR 데이터 구조로 파싱
/// * 다양한 설정 데이터 처리
/// * 포인트 클라우드 데이터 생성
#[derive(Debug, Clone)]
pub struct KanaviMobilityParser {
    buffer: Vec<u8>,
}

impl KanaviMobilityParser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl LiDARParser for KanaviMobilityParser {
    /// 바이트 데이터를 파싱하여 LiDAR 데이터로 변환
    ///
    /// # Arguments
    /// * `data` - 파싱할 바이트 데이터
    ///
    /// # Returns
    /// * `Result<Box<dyn LiDARData>, ()>` - 성공 시 파싱된 데이터, 실패 시 에러
    ///
    /// # 동작 설명
    /// 1. 데이터 버퍼에 추가
    /// 2. 헤더 검증 (0xFA)
    /// 3. 데이터 길이 확인
    /// 4. 모드에 따른 데이터 처리:
    ///    - 0xCF: 설정 데이터 파싱
    ///    - 0xF0: NAK 응답 처리
    ///    - 0xDD: 포인트 클라우드 데이터 처리
    fn parse(&mut self, data: &[u8]) -> Result<Box<dyn LiDARData>, ()> {
        self.buffer.extend_from_slice(data);
        if self.buffer.len() < 8 {
            error!("not enough data");
            return Err(());
        }

        if self.buffer[0] != 0xFA {
            self.buffer.clear();
            error!("header not found");
            return Err(());
        }

        let data_len = (self.buffer[5] as u16) << 8 | self.buffer[6] as u16;
        if self.buffer.len() < data_len as usize {
            error!("not enough data");
            return Err(());
        }

        let product_line = self.buffer[1];
        let lidar_id = self.buffer[2];
        let mode = self.buffer[3];
        let param = self.buffer[4];

        let mut lidar_data =
            KanaviMobilityData::new(data.to_vec(), product_line, lidar_id, mode, param);
        let buffer = self.buffer.clone();

        match mode {
            0xCF => match self.parse_cf(product_line, param, &buffer[7..7 + data_len as usize]) {
                Ok(Some(data)) => {
                    lidar_data.set_data(*data.downcast::<KMConfigData>().unwrap());
                }
                Ok(None) => {
                    error!("unknown parse error");
                    self.buffer.clear();
                    return Err(());
                }
                Err(e) => {
                    self.buffer.clear();
                    return Err(e);
                }
            },
            0xF0 => {
                lidar_data.set_data(KMConfigData::Nak(0x00));
            }
            0xDD => {
                let ch = param & 0x0F;
                let data = buffer[7..7 + data_len as usize].to_vec();

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
                for i in (0..data.len() - 1).step_by(2) {
                    distance.push(data[i] as f32 + data[i + 1] as f32 * 0.01);
                }

                let v_angle = fov_list[ch as usize];
                let mut fov_points = PointCloud::new();
                for h_angle_idx in 0..(h_fov / h_fov_resol) as usize {
                    let mut point = Point {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    };

                    let dist = distance[h_angle_idx];
                    let h = (PI * v_angle / 180.0).cos() * dist;
                    point.z = (PI * v_angle / 180.0).tan() * h;

                    let h_angle = (h_angle_idx as f32 * h_fov_resol) + ((180.0 - h_fov) / 2.0);
                    point.x = (PI * h_angle / 180.0).cos() * h;
                    point.y = (PI * h_angle / 180.0).tan() * point.x;

                    fov_points.add_point(point);
                }

                lidar_data.set_points(ch, fov_points);
            }
            _ => {
                // 다른 모드는 아직 구현되지 않음
            }
        }

        self.buffer.clear();
        Ok(Box::new(lidar_data))
    }
}

impl KanaviMobilityParser {
    /// 설정 데이터 파싱
    ///
    /// # Arguments
    /// * `product_line` - 제품 라인
    /// * `param` - 파라미터 (설정 타입)
    /// * `data` - 파싱할 바이트 데이터
    ///
    /// # Returns
    /// * `Result<Option<Box<dyn Any>>, ()>` - 성공 시 파싱된 설정 데이터, 실패 시 에러
    ///
    /// # 지원하는 설정 타입
    /// * 0x11: 기본 설정
    /// * 0x71: 버전 정보
    /// * 0xD1: 네트워크 소스 정보
    /// * 0xF1: 티칭 영역
    /// * 0x43: 네트워크 목적지 IP
    /// * 0x63: 모터 속도
    /// * 0x83: 경고 영역
    /// * 0xA3: 안개 필터
    /// * 0xC3: 오감지 필터
    /// * 0xE3: 최대 오감지 필터 거리
    /// * 0x05: 창 오염 감지 모드
    /// * 0x15: 티칭 모드
    /// * 0x35: 최소 오감지 필터 거리
    /// * 기타: ACK 응답
    fn parse_cf(
        &mut self,
        product_line: u8,
        param: u8,
        data: &[u8],
    ) -> Result<Option<Box<dyn Any>>, ()> {
        let mut data_idx = 0;
        match param {
            // Basic Config
            0x11 => {
                let output_channel = data[data_idx];
                data_idx += 1;
                let self_check_active_state = data[data_idx];
                data_idx += 1;
                let pulse_active_state = data[data_idx];
                data_idx += 1;
                let pulse_output_mode = data[data_idx];
                data_idx += 1;
                let pulse_pin_mode = data[data_idx];
                data_idx += 1;
                let pulse_pin_channel = data[data_idx];
                data_idx += 1;
                let start_angle = (data[data_idx] as u16) << 8 | data[data_idx + 1] as u16;
                data_idx += 2;
                let finish_angle = (data[data_idx] as u16) << 8 | data[data_idx + 1] as u16;
                data_idx += 2;
                let min_distance = data[data_idx];
                data_idx += 1;
                let max_distance = data[data_idx];
                data_idx += 1;
                let object_size = data[data_idx];
                data_idx += 1;
                let area_count = data[data_idx];
                data_idx += 1;

                let mut areas = Vec::new();
                if area_count > 0 {
                    for _i in 0..area_count as usize {
                        let point_count = data[data_idx];
                        data_idx += 1;
                        let points =
                            data[data_idx..(data_idx + (point_count * 4) as usize)].to_vec();
                        data_idx += (point_count * 4) as usize;
                        let area = UserArea::from_bytes(point_count, points);
                        areas.push(area);
                    }
                }

                return Ok(Some(Box::new(KMConfigData::BasicConfig(BasicConfig::new(
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
                )))));
            }
            // Version Info
            0x71 => {
                if self.buffer.len() < 7 {
                    error!("not enough data");
                    return Err(());
                }

                let firmware_version = [data[data_idx], data[data_idx + 1], data[data_idx + 2]];
                let hardware_version = [data[data_idx + 3], data[data_idx + 4], data[data_idx + 5]];
                let end_target = self.buffer[data_idx + 6];
                return Ok(Some(Box::new(KMConfigData::VersionInfo(VersionInfo::new(
                    firmware_version,
                    hardware_version,
                    end_target,
                )))));
            }
            // Network Source Info
            0xD1 => {
                if self.buffer.len() < 20 {
                    error!("not enough data");
                    return Err(());
                }

                let ip_address = [
                    data[data_idx],
                    data[data_idx + 1],
                    data[data_idx + 2],
                    data[data_idx + 3],
                ];
                let mac_address = [
                    data[data_idx + 4],
                    data[data_idx + 5],
                    data[data_idx + 6],
                    data[data_idx + 7],
                    data[data_idx + 8],
                    data[data_idx + 9],
                ];
                let subnet_mask = [
                    data[data_idx + 10],
                    data[data_idx + 11],
                    data[data_idx + 12],
                    data[data_idx + 13],
                ];
                let gateway = [
                    data[data_idx + 14],
                    data[data_idx + 15],
                    data[data_idx + 16],
                    data[data_idx + 17],
                ];
                let port = (data[data_idx + 18] as u16) << 8 | data[data_idx + 19] as u16;
                return Ok(Some(Box::new(KMConfigData::NetworkSourceInfo(
                    NetworkSourceInfo::new(ip_address, mac_address, subnet_mask, gateway, port),
                ))));
            }
            // Teaching Area
            0xF1 => {
                return Ok(Some(Box::new(KMConfigData::TeachingArea(
                    TeachingArea::parse(product_line, data[data_idx], data[data_idx + 1..].to_vec()),
                ))));
            }
            // Network Destination IP
            0x43 => {
                if data.len() < 4 {
                    error!("not enough data");
                    return Err(());
                }
                let ip_address = [
                    data[data_idx],
                    data[data_idx + 1],
                    data[data_idx + 2],
                    data[data_idx + 3],
                ];
                return Ok(Some(Box::new(KMConfigData::NetworkDestinationIP(
                    NetworkDestinationIP::new(ip_address),
                ))));
            }
            // Motor Speed
            0x63 => {
                if data.len() < 1 {
                    error!("not enough data");
                    return Err(());
                }

                let motor_speed = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::MotorSpeed(MotorSpeed::new(
                    motor_speed,
                )))));
            }
            // Warning Area
            0x83 => {
                if data.len() < 6 {
                    error!("not enough data");
                    return Err(());
                }

                let danger_area = [data[data_idx], data[data_idx + 1]];
                data_idx += 2;
                let warning_area = [data[data_idx], data[data_idx + 1]];
                data_idx += 2;
                let caution_area = [data[data_idx], data[data_idx + 1]];
                return Ok(Some(Box::new(KMConfigData::WarningArea(WarningArea::new(
                    danger_area,
                    warning_area,
                    caution_area,
                )))));
            }
            // Fog Filter
            0xA3 => {
                if data.len() < 1 {
                    error!("not enough data");
                    return Err(());
                }

                let filter_value = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::FogFilter(FogFilter::new(
                    filter_value,
                )))));
            }
            // Radius Filter
            0xC3 => {
                if data.len() < 1 {
                    error!("not enough data");
                    return Err(());
                }

                let filter_value = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::RadiusFilter(
                    RadiusFilter::new(filter_value),
                ))));
            }
            // Radius Filter Max Distance
            0xE3 => {
                if data.len() < 1 {
                    error!("not enough data");  
                    return Err(());
                }

                let max_distance = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::RadiusFilterMaxDistance(
                    RadiusFilterMaxDistance::new(max_distance),
                ))));
            }
            // Window Contamination Detection Mode
            0x05 => {
                if data.len() < 1 {
                    error!("not enough data");
                    return Err(());
                }

                let mode = data[data_idx];
                return Ok(Some(Box::new(
                    KMConfigData::WindowContaminationDetectionMode(
                        WindowContaminationDetectionMode::new(mode),
                    ),
                )));
            }
            // Teaching Mode
            0x15 => {
                if data.len() < 2 {
                    error!("not enough data");
                    return Err(());
                }

                let range = data[data_idx];
                data_idx += 1;
                let margin = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::TeachingMode(
                    TeachingMode::new(range, margin),
                ))));
            }
            // Radius Filter Min Distance
            0x35 => {
                if data.len() < 1 {
                    error!("not enough data");
                    return Err(());
                }

                let min_distance = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::RadiusFilterMinDistance(
                    RadiusFilterMinDistance::new(min_distance),
                ))));
            }
            // Ack
            0x01 | 0x21 | 0x31 | 0x41 | 0x51 | 0x61 | 0x81 | 0x91 | 0xA1 | 0xB1 | 0xC1 | 0xE1
            | 0x03 | 0x13 | 0x23 | 0x33 | 0x53 | 0x73 | 0x9d | 0xB3 | 0xD3 | 0xF3 | 0x25 | 0x45 => {
                let ack_code = data[data_idx];
                return Ok(Some(Box::new(KMConfigData::Ack(ack_code))));
            }
            _ => {
                error!("not supported param {}", param);
                return Err(());
            }
        }
    }
}
