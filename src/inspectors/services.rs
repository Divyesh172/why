use crate::platform::find_services;

/// Prints running services matching the query.
pub fn print_services_report(query: &str) {
    let services = find_services(query);
    if services.is_empty() {
        println!("  \x1b[90mnone\x1b[0m");
    } else {
        for service in services {
            if service.contains("(Running)") {
                println!("  {}", service.replace("(Running)", "\x1b[32m(Running)\x1b[0m"));
            } else if service.contains("(Stopped)") {
                println!("  {}", service.replace("(Stopped)", "\x1b[90m(Stopped)\x1b[0m"));
            } else {
                println!("  {}", service);
            }
        }
    }
}
