#![allow(dead_code)]
use super::{ProcessData, PortData};

/// Stubs for services lookup on Linux (would read systemd / init).
pub fn find_services(_query: &str) -> Vec<String> {
    Vec::new()
}

/// Stubs for process lookup on Linux (would read /proc).
pub fn get_process_info(_query: &str) -> Option<ProcessData> {
    None
}

/// Stubs for port connection lookup on Linux (would use netstat or ss).
pub fn get_port_info(_port: u16) -> Option<PortData> {
    None
}
