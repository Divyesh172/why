#![allow(dead_code)]
pub mod node;
pub mod python;
pub mod rust;
pub mod docker;
pub mod java;

/// Dispatch to specific ecosystem detailed printers.
pub fn print_ecosystem_details(query_name: &str) {
    match query_name {
        "node" | "npm" | "npx" => node::print_node_details(),
        "python" | "pip" => python::print_python_details(),
        "docker" => docker::print_docker_details(),
        "cargo" | "rustc" | "rustup" => rust::print_rust_details(),
        "java" | "javac" => java::print_java_details(),
        _ => {}
    }
}

