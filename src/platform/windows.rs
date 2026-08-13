use std::process::Command;
use super::{ProcessData, PortData};

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

/// Retrieves process information for a given name or PID.
pub fn get_process_info(query: &str) -> Option<ProcessData> {
    let is_pid = query.chars().all(|c| c.is_ascii_digit());
    
    let filter = if is_pid {
        format!("ProcessId = {}", query)
    } else {
        let name_clean = query.trim_end_matches(".exe");
        format!("Name = '{}.exe' or Name = '{}'", name_clean, name_clean)
    };

    let cmd = format!(
        "$p = Get-CimInstance Win32_Process -Filter \"{}\" | Sort-Object -Descending WorkingSetSize | Select-Object -First 1; \
         if ($p) {{ \
             $owner = (Invoke-CimMethod -InputObject $p -MethodName GetOwner).User; \
             $parent = Get-CimInstance Win32_Process -Filter \"ProcessId = $($p.ParentProcessId)\" | Select-Object -First 1; \
             $pname = if ($parent) {{ $parent.Name }} else {{ 'unknown' }}; \
             $cpu = (Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue).CPU; \
             Write-Output \"PID:$($p.ProcessId)\"; \
             Write-Output \"Name:$($p.Name)\"; \
             Write-Output \"Path:$($p.ExecutablePath)\"; \
             Write-Output \"Parent:$($pname) ($($p.ParentProcessId))\"; \
             Write-Output \"Cpu:$($cpu)s\"; \
             Write-Output \"Mem:$($p.WorkingSetSize)\"; \
             Write-Output \"Owner:$($owner)\"; \
             Write-Output \"Cmd:$($p.CommandLine)\"; \
         }} else {{ \
             Write-Output \"NOT_FOUND\"; \
         }}",
        filter
    );

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &cmd])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    
    if lines.contains(&"NOT_FOUND") || lines.is_empty() {
        return None;
    }

    let mut pid = String::new();
    let mut name = String::new();
    let mut path = String::new();
    let mut parent = String::new();
    let mut cpu = String::new();
    let mut mem_str = String::new();
    let mut owner = String::new();
    let mut cmd_line = String::new();

    for line in lines {
        if let Some(val) = line.strip_prefix("PID:") { pid = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Name:") { name = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Path:") { path = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Parent:") { parent = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Cpu:") { cpu = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Mem:") { mem_str = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Owner:") { owner = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Cmd:") { cmd_line = val.to_string(); }
    }

    let mem_bytes = mem_str.parse::<u64>().unwrap_or(0);

    Some(ProcessData {
        pid,
        name,
        path,
        parent,
        cpu,
        mem_bytes,
        owner,
        cmd_line,
    })
}

/// Retrieves TCP port listener connection information.
pub fn get_port_info(port: u16) -> Option<PortData> {
    let cmd = format!(
        "$conn = Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | Select-Object -First 1; \
         if ($conn) {{ \
             $pid = $conn.OwningProcess; \
             $proc = Get-CimInstance Win32_Process -Filter \"ProcessId = $pid\" | Select-Object -First 1; \
             $parent = Get-CimInstance Win32_Process -Filter \"ProcessId = $($proc.ParentProcessId)\" | Select-Object -First 1; \
             $pname = if ($parent) {{ $parent.Name }} else {{ 'unknown' }}; \
             Write-Output \"STATUS:OCCUPIED\"; \
             Write-Output \"PID:$pid\"; \
             Write-Output \"Name:$($proc.Name)\"; \
             Write-Output \"Address:$($conn.LocalAddress):$($conn.LocalPort)\"; \
             Write-Output \"Parent:$pname\"; \
             Write-Output \"Cmd:$($proc.CommandLine)\"; \
         }} else {{ \
             Write-Output \"STATUS:FREE\"; \
         }}",
        port
    );

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &cmd])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    
    if lines.contains(&"STATUS:FREE") || lines.is_empty() {
        return None;
    }

    let mut pid = String::new();
    let mut name = String::new();
    let mut address = String::new();
    let mut parent = String::new();
    let mut cmd_line = String::new();

    for line in lines {
        if let Some(val) = line.strip_prefix("PID:") { pid = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Name:") { name = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Address:") { address = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Parent:") { parent = val.to_string(); }
        else if let Some(val) = line.strip_prefix("Cmd:") { cmd_line = val.to_string(); }
    }

    Some(PortData {
        pid,
        name,
        address,
        parent,
        cmd_line,
    })
}
