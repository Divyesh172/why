use std::process::Command;

/// Inspects a running process by name or PID.
pub fn inspect_process(query: &str) {
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
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        
        if lines.contains(&"NOT_FOUND") || lines.is_empty() {
            println!("\n\x1b[1m\x1b[33mNo running process found matching '{}'\x1b[0m", query);
            return;
        }

        let mut pid = "";
        let mut name = "";
        let mut path = "";
        let mut parent = "";
        let mut cpu = "";
        let mut mem_str = "";
        let mut owner = "";
        let mut cmd_line = "";

        for line in lines {
            if let Some(val) = line.strip_prefix("PID:") { pid = val; }
            else if let Some(val) = line.strip_prefix("Name:") { name = val; }
            else if let Some(val) = line.strip_prefix("Path:") { path = val; }
            else if let Some(val) = line.strip_prefix("Parent:") { parent = val; }
            else if let Some(val) = line.strip_prefix("Cpu:") { cpu = val; }
            else if let Some(val) = line.strip_prefix("Mem:") { mem_str = val; }
            else if let Some(val) = line.strip_prefix("Owner:") { owner = val; }
            else if let Some(val) = line.strip_prefix("Cmd:") { cmd_line = val; }
        }

        let mem_formatted = if let Ok(bytes) = mem_str.parse::<u64>() {
            format!("{} MB", bytes / 1024 / 1024)
        } else {
            mem_str.to_string()
        };

        println!("\n\x1b[1m\x1b[36m=== Process Report: {} ===\x1b[0m", name);
        println!("\n\x1b[1mPID:\x1b[0m\n  {}", pid);
        println!("\n\x1b[1mExecutable:\x1b[0m\n  {}", if path.is_empty() { "(unknown)" } else { path });
        println!("\n\x1b[1mParent Process:\x1b[0m\n  {}", parent);
        println!("\n\x1b[1mCPU Time:\x1b[0m\n  {}", if cpu.is_empty() || cpu == "s" { "(not available)" } else { cpu });
        println!("\n\x1b[1mMemory (Working Set):\x1b[0m\n  {}", mem_formatted);
        println!("\n\x1b[1mUser:\x1b[0m\n  {}", if owner.is_empty() { "unknown" } else { owner });
        println!("\n\x1b[1mCommand Line:\x1b[0m\n  {}", if cmd_line.is_empty() { "(unknown)" } else { cmd_line });
        println!();
    } else {
        println!("\n\x1b[1m\x1b[31mError querying process information.\x1b[0m");
    }
}
