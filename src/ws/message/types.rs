use serde::{Deserialize, Serialize};

pub mod command {
    pub const GET: &str = "get";
    pub const SET: &str = "set";
}

pub mod config_type {
    pub const BASIC_CONFIG: &str = "basic_config";
    pub const VERSION_INFO: &str = "version_info";
    pub const NETWORK_SOURCE_INFO: &str = "network_source_info";
    pub const TEACHING_AREA: &str = "teaching_area";
    pub const NETWORK_DESTINATION_IP: &str = "network_destination_ip";
    pub const MOTOR_SPEED: &str = "motor_speed";
    pub const WARNING_AREA: &str = "warning_area";
    pub const FOG_FILTER: &str = "fog_filter";
    pub const RADIUS_FILTER: &str = "radius_filter";
    pub const RADIUS_FILTER_MAX_DISTANCE: &str = "radius_filter_max_distance";
    pub const WINDOW_CONTAMINATION_DETECTION_MODE: &str = "window_contamination_detection_mode";
    pub const TEACHING_MODE: &str = "teaching_mode";
    pub const RADIUS_FILTER_MIN_DISTANCE: &str = "radius_filter_min_distance";
}

pub mod response_status {
    pub const SUCCESS: &str = "success";
    pub const ERROR: &str = "error";
}

// Request types
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestMessage {
    pub command: String,  // "get" or "set"
    pub params: RequestParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestParams {
    pub r#type: String,
    pub data: Option<serde_json::Value>,
}

// Response types
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub status: String,  // "success" or "error"
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl ResponseMessage {
    pub fn success(data: Option<serde_json::Value>) -> Self {
        Self {
            status: "success".to_string(),
            message: "".to_string(),
            data,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            status: "error".to_string(),
            message,
            data: None,
        }
    }
}