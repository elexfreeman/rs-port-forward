use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub enum LogEvent {
    ConnectionStarted {
        ts: DateTime<Utc>,
        name: String,
        local_port: u16,
        remote_address: String,
        remote_port: u16,
        client_addr: Option<String>,
    },
    ConnectionClosed {
        ts: DateTime<Utc>,
        name: String,
        local_port: u16,
        remote_address: String,
        remote_port: u16,
        client_addr: Option<String>,
        bytes_from_to: u64,
        bytes_to_from: u64,
    },
    ConnectionError {
        ts: DateTime<Utc>,
        name: String,
        local_port: u16,
        remote_address: String,
        remote_port: u16,
        client_addr: Option<String>,
        error: String,
    },
    ConnectionTimeout {
        ts: DateTime<Utc>,
        name: String,
        local_port: u16,
        remote_address: String,
        remote_port: u16,
        client_addr: Option<String>,
        error: String,
    },
}
