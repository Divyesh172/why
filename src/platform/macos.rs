#![allow(dead_code)]
use super::{ProcessData, PortData};

/// Stubs for services lookup on macOS (would query launchctl).
pub fn find_services(_query: &str) -> Vec<String> {
    Vec::new()
}

/// Stubs for process lookup on macOS (would query ps/sysctl).
pub fn get_process_info(_query: &str) -> Option<ProcessData> {
    None
}

/// Stubs for port connection lookup on macOS (would use lsof).
pub fn get_port_info(_port: u16) -> Option<PortData> {
    None
}
