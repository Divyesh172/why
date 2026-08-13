use crate::platform::get_process_info;

/// Inspects a running process by name or PID.
pub fn inspect_process(query: &str) {
    match get_process_info(query) {
        Some(proc) => {
            let mem_formatted = format!("{} MB", proc.mem_bytes / 1024 / 1024);

            println!("\n\x1b[1m\x1b[36m=== Process Report: {} ===\x1b[0m", proc.name);
            println!("\n\x1b[1mPID:\x1b[0m\n  {}", proc.pid);
            println!("\n\x1b[1mExecutable:\x1b[0m\n  {}", if proc.path.is_empty() { "(unknown)" } else { &proc.path });
            println!("\n\x1b[1mParent Process:\x1b[0m\n  {}", proc.parent);
            println!("\n\x1b[1mCPU Time:\x1b[0m\n  {}", if proc.cpu.is_empty() || proc.cpu == "s" { "(not available)" } else { &proc.cpu });
            println!("\n\x1b[1mMemory (Working Set):\x1b[0m\n  {}", mem_formatted);
            println!("\n\x1b[1mUser:\x1b[0m\n  {}", if proc.owner.is_empty() { "unknown" } else { &proc.owner });
            println!("\n\x1b[1mCommand Line:\x1b[0m\n  {}", if proc.cmd_line.is_empty() { "(unknown)" } else { &proc.cmd_line });
            println!();
        }
        None => {
            println!("\n\x1b[1m\x1b[33mNo running process found matching '{}'\x1b[0m", query);
        }
    }
}
