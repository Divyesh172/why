use crate::platform::get_port_info;

/// Inspects a TCP port to see if it is occupied and by which process.
pub fn inspect_port(port: u16) {
    match get_port_info(port) {
        Some(conn) => {
            println!("\n\x1b[1m\x1b[36m=== Port Report: {} ===\x1b[0m", port);
            println!("\n\x1b[1mStatus:\x1b[0m\n  \x1b[31mOCCUPIED\x1b[0m");
            println!("\n\x1b[1mListening on:\x1b[0m\n  {}", conn.address);
            println!("\n\x1b[1mProcess:\x1b[0m\n  {} (PID: {})", conn.name, conn.pid);
            println!("\n\x1b[1mParent Process:\x1b[0m\n  {}", conn.parent);
            println!("\n\x1b[1mCommand:\x1b[0m\n  {}", if conn.cmd_line.is_empty() { "(unknown)" } else { &conn.cmd_line });
            println!();
        }
        None => {
            println!("\n\x1b[1m\x1b[36m=== Port Report: {} ===\x1b[0m", port);
            println!("\n\x1b[1mStatus:\x1b[0m\n  \x1b[32mFREE\x1b[0m");
            println!();
        }
    }
}
