use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod status;
pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u128,
    pub message_type: types::MessageType,
    pub metadata: HashMap<String, String>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    operation: types::ControlOperationType,
    service: String,
    function: String,
}
