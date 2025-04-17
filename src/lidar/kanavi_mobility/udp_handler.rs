use std::{f32::consts::PI, net::Ipv4Addr};

use crate::lidar::{response_status, LiDARError, Point, PointCloud, ResponseMessage, UDPHandler};
use serde_json::Value;
use tracing::error;

use crate::lidar::kanavi_mobility::types::*;
pub struct KanaviUDPHandler;

impl UDPHandler for KanaviUDPHandler {
    fn parse(&mut self, ip: Ipv4Addr, port: u16, data: &[u8]) -> Result<Value, Box<LiDARError>> {
        if data.len() < 8 {
            return Err(Box::new(LiDARError::InvalidData(
                "not enough data".to_string(),
            )));
        }

        if data[0] != 0xFA {
            return Err(Box::new(LiDARError::InvalidData(
                "invalid header".to_string(),
            )));
        }

        let data_len = (data[5] as u16) << 8 | data[6] as u16;
        let total_len = data_len as usize + 7 + 1;
        if data.len() != total_len {
            return Err(Box::new(LiDARError::InvalidData(
                "not enough data".to_string(),
            )));
        }

        let product_line = data[1];
        let lidar_id = data[2];
        let mode = data[3];
        let param = data[4];

        let mut res: ResponseMessage = ResponseMessage::new();
        res.lidar_info = LiDARInfo::new(ip, port, product_line, lidar_id).to_json();

        match mode {
            0xCF => {
                let _ret = self.parse_cf(product_line, param, &data[7..7 + data_len as usize]);
                match _ret {
                    Ok(data) => {
                        res.data = Some(data);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            0xF0 => {
                res.status = response_status::ERROR.to_string();
                res.message = "NAK".to_string();
            }
            0xDD => {
                res.status = response_status::NONE.to_string();
                let _ret = self.parse_dd(product_line, param, &data[7..7 + data_len as usize]);
                match _ret {
                    Ok(data) => {
                        res.data = Some(data);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            _ => {}
        }

        Ok(res.to_json())
    }
}

impl KanaviUDPHandler {
    fn parse_cf(
        &mut self,
        product_line: u8,
        param: u8,
        data: &[u8],
    ) -> Result<Value, Box<LiDARError>> {
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

                return Ok(BasicConfig::new(
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
                )
                .to_json());
            }
            // Version Info
            0x71 => {
                let firmware_version = [data[data_idx], data[data_idx + 1], data[data_idx + 2]];
                let hardware_version = [data[data_idx + 3], data[data_idx + 4], data[data_idx + 5]];
                let end_target = data[data_idx + 6];
                return Ok(
                    VersionInfo::new(firmware_version, hardware_version, end_target).to_json(),
                );
            }
            // Network Source Info
            0xD1 => {
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
                return Ok(NetworkSourceInfo::new(
                    ip_address,
                    mac_address,
                    subnet_mask,
                    gateway,
                    port,
                )
                .to_json());
            }
            // Teaching Area
            0xF1 => {
                return Ok(TeachingArea::parse(
                    product_line,
                    data[data_idx],
                    data[data_idx + 1..].to_vec(),
                )
                .to_json());
            }
            // Network Destination IP
            0x43 => {
                let ip_address = [
                    data[data_idx],
                    data[data_idx + 1],
                    data[data_idx + 2],
                    data[data_idx + 3],
                ];
                return Ok(NetworkDestinationIP::new(ip_address).to_json());
            }
            // Motor Speed
            0x63 => {
                let motor_speed = data[data_idx];
                return Ok(MotorSpeed::new(motor_speed).to_json());
            }
            // Warning Area
            0x83 => {
                let danger_area = [data[data_idx], data[data_idx + 1]];
                data_idx += 2;
                let warning_area = [data[data_idx], data[data_idx + 1]];
                data_idx += 2;
                let caution_area = [data[data_idx], data[data_idx + 1]];
                return Ok(WarningArea::new(danger_area, warning_area, caution_area).to_json());
            }
            // Fog Filter
            0xA3 => {
                let filter_value = data[data_idx];
                return Ok(FogFilter::new(filter_value).to_json());
            }
            // Radius Filter
            0xC3 => {
                let filter_value = data[data_idx];
                return Ok(RadiusFilter::new(filter_value).to_json());
            }
            // Radius Filter Max Distance
            0xE3 => {
                let max_distance = data[data_idx];
                return Ok(RadiusFilterMaxDistance::new(max_distance).to_json());
            }
            // Window Contamination Detection Mode
            0x05 => {
                let mode = data[data_idx];
                return Ok(WindowContaminationDetectionMode::new(mode).to_json());
            }
            // Teaching Mode
            0x15 => {
                let range = data[data_idx];
                data_idx += 1;
                let margin = data[data_idx];
                return Ok(TeachingMode::new(range, margin).to_json());
            }
            // Radius Filter Min Distance
            0x35 => {
                let min_distance = data[data_idx];
                return Ok(RadiusFilterMinDistance::new(min_distance).to_json());
            }
            // Ack
            0x01 | 0x21 | 0x31 | 0x41 | 0x51 | 0x61 | 0x81 | 0x91 | 0xA1 | 0xB1 | 0xC1 | 0xE1
            | 0x03 | 0x13 | 0x23 | 0x33 | 0x53 | 0x73 | 0x9d | 0xB3 | 0xD3 | 0xF3 | 0x25 | 0x45 => {
                let ack_code = data[data_idx];
                return Ok(Ack::new(ack_code).to_json());
            }
            _ => {
                error!("not supported param {}", param);
                return Err(Box::new(LiDARError::InvalidData(
                    "not supported param".to_string(),
                )));
            }
        }
    }

    fn parse_dd(
        &mut self,
        product_line: u8,
        param: u8,
        data: &[u8],
    ) -> Result<Value, Box<LiDARError>> {
        let ch = param & 0x0F;
        let mut point_cloud_data = PointCloudData::new(PointCloud::new(), ch, data[data.len() - 1]);

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

            point_cloud_data.point_cloud.add_point(point);
        }

        Ok(point_cloud_data.to_json())
    }
}
