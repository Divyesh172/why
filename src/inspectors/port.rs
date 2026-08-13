use std::process::Command;

/// Inspects a TCP port to see if it is occupied and by which process.
pub fn inspect_port(port: u16) {
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
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        
        println!("\n\x1b[1m\x1b[36m=== Port Report: {} ===\x1b[0m", port);

        if lines.contains(&"STATUS:FREE") || lines.is_empty() {
            println!("\n\x1b[1mStatus:\x1b[0m\n  \x1b[32mFREE\x1b[0m");
            println!();
            return;
        }

        let mut pid = "";
        let mut name = "";
        let mut address = "";
        let mut parent = "";
        let mut cmd_line = "";

        for line in lines {
            if let Some(val) = line.strip_prefix("PID:") { pid = val; }
            else if let Some(val) = line.strip_prefix("Name:") { name = val; }
            else if let Some(val) = line.strip_prefix("Address:") { address = val; }
            else if let Some(val) = line.strip_prefix("Parent:") { parent = val; }
            else if let Some(val) = line.strip_prefix("Cmd:") { cmd_line = val; }
        }

        println!("\n\x1b[1mStatus:\x1b[0m\n  \x1b[31mOCCUPIED\x1b[0m");
        println!("\n\x1b[1mListening on:\x1b[0m\n  {}", address);
        println!("\n\x1b[1mProcess:\x1b[0m\n  {} (PID: {})", name, pid);
        println!("\n\x1b[1mParent Process:\x1b[0m\n  {}", parent);
        println!("\n\x1b[1mCommand:\x1b[0m\n  {}", if cmd_line.is_empty() { "(unknown)" } else { cmd_line });
        println!();
    } else {
        println!("\n\x1b[1m\x1b[31mError querying port status.\x1b[0m");
    }
}
