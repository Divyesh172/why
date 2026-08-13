pub mod windows;
pub mod linux;
pub mod macos;

#[derive(Debug, Clone)]
pub struct ProcessData {
    pub pid: String,
    pub name: String,
    pub path: String,
    pub parent: String,
    pub cpu: String,
    pub mem_bytes: u64,
    pub owner: String,
    pub cmd_line: String,
}

#[derive(Debug, Clone)]
pub struct PortData {
    pub pid: String,
    pub name: String,
    pub address: String,
    pub parent: String,
    pub cmd_line: String,
}

#[cfg(target_os = "windows")]
pub use windows::{find_services, get_process_info, get_port_info};

#[cfg(target_os = "linux")]
pub use linux::{find_services, get_process_info, get_port_info};

#[cfg(target_os = "macos")]
pub use macos::{find_services, get_process_info, get_port_info};

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use linux::{find_services, get_process_info, get_port_info};
