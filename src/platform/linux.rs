use std::process::Command;
use super::{ProcessData, PortData};

/// Queries systemctl on Linux to locate active services.
pub fn find_services(query: &str) -> Vec<String> {
    let mut services = Vec::new();
    let output = Command::new("systemctl")
        .args(&["list-units", "--type=service", "--all", "--no-pager"])
        .output();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.contains(query) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0];
                    let status = parts[3];
                    let display_status = if status == "running" { "Running" } else { "Stopped" };
                    services.push(format!("{} ({})", name, display_status));
                }
            }
        }
    }
    services
}

/// Fetches process details using pgrep and ps on Linux.
pub fn get_process_info(query: &str) -> Option<ProcessData> {
    let is_pid = query.chars().all(|c| c.is_ascii_digit());
    let pid = if is_pid {
        query.to_string()
    } else {
        let output = Command::new("pgrep")
            .arg(query)
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?;
        first_line.trim().to_string()
    };

    let output = Command::new("ps")
        .args(&["-p", &pid, "-o", "pid,comm,args,rss,user"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("PID") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let pid_val = parts[0].to_string();
            let comm = parts[1].to_string();
            let user = parts[4].to_string();
            
            let rss_kb = parts[3].parse::<u64>().unwrap_or(0);
            let mem_bytes = rss_kb * 1024;
            let cmd_line = parts[2..].join(" ");
            
            // Read executable link from proc filesystem
            let exe_path = format!("/proc/{}/exe", pid_val);
            let path = std::fs::read_link(exe_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            
            let ppid_output = Command::new("ps")
                .args(&["-p", &pid_val, "-o", "ppid="])
                .output()
                .ok()?;
            let parent = String::from_utf8_lossy(&ppid_output.stdout).trim().to_string();

            return Some(ProcessData {
                pid: pid_val,
                name: comm,
                path,
                parent,
                cpu: "unknown".to_string(),
                mem_bytes,
                owner: user,
                cmd_line,
            });
        }
    }
    None
}

/// Identifies the process listening on a given port using lsof on Linux.
pub fn get_port_info(port: u16) -> Option<PortData> {
    let port_str = port.to_string();
    let output = Command::new("lsof")
        .args(&["-i", &format!(":{}", port_str), "-sTCP:LISTEN", "-t"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pid = stdout.lines().next()?.trim();
    if pid.is_empty() {
        return None;
    }

    let proc = get_process_info(pid)?;
    Some(PortData {
        pid: proc.pid.clone(),
        name: proc.name.clone(),
        address: format!("*:{}", port),
        parent: proc.parent.clone(),
        cmd_line: proc.cmd_line.clone(),
    })
}
