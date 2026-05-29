//! PDF Extraction Tool for NDS Supplement and Design Value Tables
//!
//! This tool extracts text and tables from PDF files for conversion to TOML format.
//! It's designed to assist with transcribing design values from structural engineering
//! code documents.
//!
//! Usage:
//!   cargo run --bin pdf-extractor -- <pdf-path> [options]
//!
//! Options:
//!   --pages <range>    Extract specific pages (e.g., "1-10" or "5,7,12")
//!   --output <dir>     Output directory (default: data/extracted/)
//!   --format <fmt>     Output format: text, csv, or toml-template
//!
//! Examples:
//!   cargo run --bin pdf-extractor -- "codes/2015/AWC NDS 2015 Supplement.pdf"
//!   cargo run --bin pdf-extractor -- "codes/2015/AWC NDS 2015 Supplement.pdf" --pages 20-30

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let pdf_path = &args[1];

    if pdf_path == "--help" || pdf_path == "-h" {
        print_usage();
        return;
    }

    // Parse options
    let mut output_dir = PathBuf::from("data/extracted");
    let mut page_range: Option<(usize, usize)> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--pages" => {
                if i + 1 < args.len() {
                    page_range = parse_page_range(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
            }
        }
        i += 1;
    }

    // Verify PDF exists
    let pdf_path = Path::new(pdf_path);
    if !pdf_path.exists() {
        eprintln!("Error: PDF file not found: {}", pdf_path.display());
        std::process::exit(1);
    }

    // Create output directory
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("PDF Extractor for Structural Engineering Design Values");
    println!("======================================================");
    println!();
    println!("Input:  {}", pdf_path.display());
    println!("Output: {}", output_dir.display());
    if let Some((start, end)) = page_range {
        println!("Pages:  {}-{}", start, end);
    }
    println!();

    // Extract text from PDF
    match extract_pdf_text(pdf_path) {
        Ok(text) => {
            // Save raw text
            let text_file = output_dir.join(format!(
                "{}_extracted.txt",
                pdf_path.file_stem().unwrap().to_string_lossy()
            ));
            fs::write(&text_file, &text).expect("Failed to write text file");
            println!("Extracted text saved to: {}", text_file.display());

            // Analyze for tables
            let tables = find_potential_tables(&text);
            println!();
            println!("Found {} potential table regions", tables.len());

            // Save table analysis
            let analysis_file = output_dir.join(format!(
                "{}_analysis.txt",
                pdf_path.file_stem().unwrap().to_string_lossy()
            ));
            let mut analysis = String::new();
            analysis.push_str("# Table Analysis\n\n");
            analysis.push_str("This file identifies potential tables in the PDF.\n");
            analysis.push_str("Review these sections for design values to transcribe.\n\n");

            for (i, (line_num, preview)) in tables.iter().enumerate() {
                analysis.push_str(&format!("## Table {} (around line {})\n", i + 1, line_num));
                analysis.push_str("```\n");
                analysis.push_str(preview);
                analysis.push_str("\n```\n\n");
            }

            fs::write(&analysis_file, &analysis).expect("Failed to write analysis");
            println!("Table analysis saved to: {}", analysis_file.display());

            // Generate TOML template hint
            println!();
            println!("Next steps:");
            println!("1. Review {}", text_file.display());
            println!("2. Identify tables with design values");
            println!("3. Transcribe values to TOML format in data/wood/");
            println!("4. Follow data/TRANSCRIPTION_GUIDE.md for format details");
        }
        Err(e) => {
            eprintln!("Error extracting PDF: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("PDF Extractor for NDS Supplement and Design Value Tables");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin pdf-extractor -- <pdf-path> [options]");
    println!();
    println!("ARGS:");
    println!("    <pdf-path>    Path to the PDF file to extract");
    println!();
    println!("OPTIONS:");
    println!("    --pages <range>    Extract specific pages (e.g., \"1-10\")");
    println!("    --output <dir>     Output directory (default: data/extracted/)");
    println!("    -h, --help         Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    cargo run --bin pdf-extractor -- \"codes/2015/AWC NDS 2015 Supplement.pdf\"");
    println!();
    println!("The tool extracts text from the PDF and identifies potential tables");
    println!("containing design values. You then manually transcribe the values to TOML.");
}

fn parse_page_range(s: &str) -> Option<(usize, usize)> {
    if let Some(idx) = s.find('-') {
        let start = s[..idx].parse().ok()?;
        let end = s[idx + 1..].parse().ok()?;
        Some((start, end))
    } else {
        let page = s.parse().ok()?;
        Some((page, page))
    }
}

fn extract_pdf_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read PDF: {}", e))?;

    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => Ok(text),
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("decrypt")
                || error_msg.contains("password")
                || error_msg.contains("encryption")
            {
                Err(format!(
                    "PDF appears to be encrypted or password-protected.\n\n\
                     The NDS Supplement PDFs are often protected by AWC.\n\n\
                     Alternative approaches:\n\
                     1. Purchase the official NDS Supplement (print version has tables)\n\
                     2. Contact AWC (info@awc.org) for digital data licensing\n\
                     3. Use official AWC DCA documents (some are public)\n\
                     4. Manually transcribe from a printed copy\n\n\
                     See bead Stratify-q38 for AWC licensing contact task.\n\n\
                     Original error: {}",
                    e
                ))
            } else {
                Err(format!("Failed to extract text: {}", e))
            }
        }
    }
}

fn find_potential_tables(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut tables = Vec::new();

    // Look for patterns that suggest table headers
    let table_indicators = [
        "Species",
        "Grade",
        "Fb",
        "Ft",
        "Fv",
        "Fc",
        "E",
        "Emin",
        "Table 4A",
        "Table 4B",
        "Table 5A",
        "Table 5B",
        "Douglas",
        "Southern",
        "Hem-Fir",
        "Spruce",
        "Select Structural",
        "No. 1",
        "No. 2",
        "No. 3",
        "LVL",
        "PSL",
        "LSL",
        "Glulam",
        "Modulus",
        "Bending",
        "Tension",
        "Shear",
        "Compression",
    ];

    for (line_num, line) in lines.iter().enumerate() {
        for indicator in &table_indicators {
            if line.contains(indicator) {
                // Capture context (5 lines before and after)
                let start = line_num.saturating_sub(5);
                let end = (line_num + 6).min(lines.len());
                let preview: String = lines[start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        if start + i == line_num {
                            format!(">>> {}", l)
                        } else {
                            format!("    {}", l)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Avoid duplicates (same region)
                let already_found = tables
                    .iter()
                    .any(|(ln, _): &(usize, String)| (*ln as isize - line_num as isize).abs() < 10);

                if !already_found {
                    tables.push((line_num + 1, preview));
                }
                break;
            }
        }
    }

    tables
}
