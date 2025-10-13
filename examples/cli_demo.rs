use std::env;

/// Example demonstrating CLI argument parsing
/// Run with: cargo run --example cli_demo -- --help
/// Or: cargo run --example cli_demo -- --bind-address 0.0.0.0 --port 9090
fn main() {
    println!("CLI Demo - showing how to use the WebSocket server CLI");
    println!("Current command line arguments:");
    
    let args: Vec<String> = env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        println!("  {}: {}", i, arg);
    }
    
    println!("\nTo see the full CLI help, run:");
    println!("cargo run -- --help");
    
    println!("\nExample usage:");
    println!("cargo run -- --bind-address 0.0.0.0 --port 8080 --max-connections 500 --log-level debug");
}