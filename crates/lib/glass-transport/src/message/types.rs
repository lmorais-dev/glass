use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Control,
    DataStream,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ControlOperationType {
    Unary,
    ClientStreaming,
    ServerStreaming,
    BidirectionalStreaming,
}