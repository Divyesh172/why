use std::process::Command;

/// Queries Windows services to find matches.
pub fn find_services(query: &str) -> Vec<String> {
    let mut services = Vec::new();
    let filter = format!("*{}*", query);
    let cmd = format!(
        "Get-Service -Name '{}' | ForEach-Object {{ `$_.Name + ' (' + `$_.Status + ')' }}",
        filter
    );
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &cmd])
        .output();
        
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    services.push(trimmed.to_string());
                }
            }
        }
    }
    
    services
}
