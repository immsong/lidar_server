use std::{f32::consts::E, net::Ipv4Addr, sync::Arc};

use serde_json::Value;

use crate::{
    lidar::{
        request_command, response_status, LiDARError, RequestMessage, ResponseMessage, WsHandler,
    },
    ws::server::AppState,
};

use super::{request_types, LiDARInfo};

pub struct KanaviMobilityWsHandler {
    state: Arc<AppState>,
    client_id: uuid::Uuid,
}

impl KanaviMobilityWsHandler {
    pub fn new(state: Arc<AppState>, client_id: uuid::Uuid) -> Self {
        Self { state, client_id }
    }
}

impl WsHandler for KanaviMobilityWsHandler {
    async fn parse(
        &mut self,
        ip: Ipv4Addr,
        port: u16,
        data: Value,
    ) -> Result<(Value, Vec<u8>), Box<LiDARError>> {
        let mut ret = (ResponseMessage::new().to_json(), vec![]);

        if let Ok(request_message) = serde_json::from_str::<RequestMessage>(&data.to_string()) {
            let req_lidar_info = request_message.lidar_info.clone();
            match request_message.command.as_str() {
                request_command::GET => {
                    if let Ok(_ret) = self.parse_get(request_message).await {
                        ret = _ret;
                    }
                }
                request_command::SET => {
                    if let Ok(_ret) = self.parse_set(request_message).await {
                        ret = _ret;
                    }
                }
                _ => {
                    return Err(Box::new(LiDARError::InvalidData(
                        "not supported request command".to_string(),
                    )));
                }
            }

            ret.0["lidar_info"] = req_lidar_info;
            return Ok(ret);
        }

        Err(Box::new(LiDARError::InvalidData(
            "invalid request".to_string(),
        )))
    }
}

impl KanaviMobilityWsHandler {
    async fn parse_set(
        &self,
        request_message: RequestMessage,
    ) -> Result<(Value, Vec<u8>), Box<LiDARError>> {
        let mut res = ResponseMessage::new();

        match request_message.r#type.as_str() {
            request_types::REGISTER_LIDAR => {
                res.status = response_status::SUCCESS.to_string();
            }
            _ => {
                return Err(Box::new(LiDARError::InvalidData(
                    "not supported request type".to_string(),
                )));
            }
        }

        Ok((res.to_json(), vec![]))
    }
}

impl KanaviMobilityWsHandler {
    async fn parse_get(
        &self,
        request_message: RequestMessage,
    ) -> Result<(Value, Vec<u8>), Box<LiDARError>> {
        let mut res = ResponseMessage::new();
        let mut raw_data = vec![];

        match request_message.r#type.as_str() {
            request_types::LIDAR_LIST => {
                {
                    let mut lidar_infos = self.state.lidar_infos.lock().await;
                    lidar_infos.clear();
                }
                // sleep 1000ms
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

                res.status = response_status::SUCCESS.to_string();
                res.data = Some(
                    serde_json::to_value(
                        &self
                            .state
                            .lidar_infos
                            .lock()
                            .await
                            .iter()
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                );
            }
            request_types::BASIC_CONFIG => {
                if let Ok(lidar_info) =
                    serde_json::from_value::<LiDARInfo>(request_message.lidar_info)
                {
                    let data = vec![0xED];
                    res.status = response_status::NONE.to_string();

                    raw_data.push(0xFA);
                    raw_data.push(lidar_info.product_line);
                    raw_data.push(lidar_info.lidar_id);
                    raw_data.push(0xCF);
                    raw_data.push(0x10);
                    raw_data.push((data.len() >> 8) as u8);
                    raw_data.push((data.len() & 0xFF) as u8);
                    raw_data.extend(data);
                    let mut checksum = raw_data[0];
                    for i in 1..raw_data.len() {
                        checksum ^= raw_data[i];
                    }
                    raw_data.push(checksum);
                } else {
                    return Err(Box::new(LiDARError::InvalidData(
                        "lidar_info is invalid".to_string(),
                    )));
                }
            }
            _ => {
                return Err(Box::new(LiDARError::InvalidData(
                    "not supported request type".to_string(),
                )));
            }
        }

        Ok((res.to_json(), raw_data))
    }
}
