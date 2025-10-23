use std::process::Command;
use std::env;
use std::path::Path;

fn main() {
    // Only run diesel print-schema in development builds
    if env::var("PROFILE").unwrap_or_default() == "debug" {
        println!("cargo:rerun-if-changed=migrations/");
        
        // Check if diesel CLI is available
        if Command::new("diesel").arg("--version").output().is_ok() {
            let output = Command::new("diesel")
                .args(&["print-schema"])
                .output();
                
            match output {
                Ok(output) if output.status.success() => {
                    // Write schema to src/schema.rs
                    let schema_path = Path::new("src/schema.rs");
                    if let Ok(schema_content) = std::str::from_utf8(&output.stdout) {
                        if std::fs::write(schema_path, schema_content).is_ok() {
                            println!("cargo:warning=Schema generated successfully");
                        }
                    }
                },
                Ok(output) => {
                    eprintln!("diesel print-schema failed: {}", 
                        String::from_utf8_lossy(&output.stderr));
                },
                Err(e) => {
                    eprintln!("Failed to execute diesel CLI: {}", e);
                }
            }
        } else {
            println!("cargo:warning=diesel CLI not found, skipping schema generation");
        }
    }
}
