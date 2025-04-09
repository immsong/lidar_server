use crate::lidar::kanavi_mobility::types::*;
use crate::ws::message::types::*;

pub struct MessageHandler;

impl MessageHandler {
    pub fn handle_message(&self, msg: RequestMessage) -> ResponseMessage {
        match msg.command.as_str() {
            command::GET => self.handle_get(msg.params),
            command::SET => self.handle_set(msg.params),
            _ => ResponseMessage::error(format!("Unknown command: {}", msg.command)),
        }
    }

    fn handle_get(&self, params: RequestParams) -> ResponseMessage {
        match params.r#type.as_str() {
            config_type::VERSION_INFO => { 
                // TODO: 실제 LiDAR에서 버전 정보를 가져오는 로직 구현
                ResponseMessage::success(None)
                
            }
            _ => ResponseMessage::error(format!("Unknown type: {}", params.r#type)),
        }
    }

    fn handle_set(&self, params: RequestParams) -> ResponseMessage {
        match params.r#type.as_str() {
            config_type::BASIC_CONFIG => {
                if let Some(data) = params.data {
                    if let Ok(config) = serde_json::from_value::<BasicConfig>(data) {
                        // TODO: LiDAR에 설정을 적용하는 로직 구현
                        ResponseMessage::success(None)
                    } else {
                        ResponseMessage::error("Invalid basic config format".to_string())
                    }
                } else {
                    ResponseMessage::error("Missing data for basic config".to_string())
                }
            }
            _ => ResponseMessage::error(format!("Unknown type: {}", params.r#type)),
        }
    }
}