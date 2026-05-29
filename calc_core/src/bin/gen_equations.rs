//! Generate EQUATIONS.md from the equation registry.
//!
//! This binary generates the equations reference documentation from the
//! centralized equation registry in calc_core.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin gen-equations
//! ```
//!
//! The generated file is written to `calc_core/src/equations/EQUATIONS.md`.

use std::fs;
use std::path::Path;

use calc_core::equations::generate_equations_markdown;

fn main() {
    println!("Generating EQUATIONS.md...");

    // Generate the markdown content
    let markdown = generate_equations_markdown();

    // Determine output path (relative to workspace root)
    let output_path = Path::new("calc_core/src/equations/EQUATIONS.md");

    // Write the file
    match fs::write(output_path, &markdown) {
        Ok(()) => {
            println!(
                "Successfully wrote {} bytes to {}",
                markdown.len(),
                output_path.display()
            );
            println!("EQUATIONS.md has been updated.");
        }
        Err(e) => {
            eprintln!("Error writing file: {}", e);
            std::process::exit(1);
        }
    }
}
