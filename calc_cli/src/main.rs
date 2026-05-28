//! # Stratify CLI
//!
//! JSON-in / JSON-out structural engineering calculator. Each subcommand reads
//! a JSON input document (from `--input PATH`, `-`, or stdin), runs the
//! corresponding calc_core calculation, and writes JSON output to stdout
//! (or `--output PATH`). Errors are emitted to stderr as JSON for machine
//! consumption:
//!
//! ```text
//! {"error":"<message>","kind":"<machine-readable-kind>"}
//! ```
//!
//! ## Exit codes
//!
//! - `0` — success
//! - `1` — I/O failure (file not found, stdin read error, write error) or
//!         malformed JSON input
//! - `2` — calculation error (validation failure, code-check failure surfaced
//!         as an error, etc.)
//! - `3` — serialization of the result failed (should never happen in practice)
//!
//! ## Examples
//!
//! ```text
//! stratify-cli beam --input beam.json --pretty
//! cat beam.json | stratify-cli beam
//! stratify-cli continuous-beam -i cb.json -o cb-result.json
//! stratify-cli beam -i beam.json --method lrfd
//! ```

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use calc_core::calculations::beam::{calculate, BeamInput};
use calc_core::calculations::continuous_beam::{calculate_continuous, ContinuousBeamInput};
use calc_core::{CalcError, DesignMethod};

// ============================================================================
// CLI Surface
// ============================================================================

#[derive(Parser)]
#[command(
    name = "stratify-cli",
    about = "Stratify - JSON-in / JSON-out structural calculator",
    long_about = "Reads structural calculation inputs as JSON and emits results \
as JSON. Inputs may come from a file (--input PATH) or stdin; outputs go to \
stdout or --output PATH. Errors are JSON on stderr.",
    version,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a simply-supported beam (NDS, wood).
    Beam(IoArgs),

    /// Analyze a multi-span continuous beam via Hardy Cross moment distribution.
    #[command(name = "continuous-beam")]
    ContinuousBeam(IoArgs),
}

#[derive(Args)]
struct IoArgs {
    /// Input JSON file. Use `-` or omit to read from stdin.
    #[arg(short, long, value_name = "PATH")]
    input: Option<PathBuf>,

    /// Output JSON file. Omit to write to stdout.
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Pretty-print the output JSON (default: compact).
    #[arg(short, long)]
    pretty: bool,

    /// Design method.
    #[arg(short, long, value_enum, default_value_t = MethodArg::Asd)]
    method: MethodArg,
}

#[derive(Clone, Copy, ValueEnum)]
enum MethodArg {
    /// Allowable Stress Design (ASCE 7 ASD load combinations)
    Asd,
    /// Load and Resistance Factor Design (ASCE 7 LRFD load combinations)
    Lrfd,
}

impl From<MethodArg> for DesignMethod {
    fn from(m: MethodArg) -> Self {
        match m {
            MethodArg::Asd => DesignMethod::Asd,
            MethodArg::Lrfd => DesignMethod::Lrfd,
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Beam(io) => run_beam(io),
        Command::ContinuousBeam(io) => run_continuous_beam(io),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            emit_error_json(&e);
            e.exit_code()
        }
    }
}

fn run_beam(io: IoArgs) -> Result<(), CliError> {
    let bytes = read_input(io.input.as_deref())?;
    let input: BeamInput = serde_json::from_slice(&bytes).map_err(CliError::ParseInput)?;
    let result = calculate(&input, io.method.into()).map_err(CliError::Calc)?;
    write_json(io.output.as_deref(), &result, io.pretty)
}

fn run_continuous_beam(io: IoArgs) -> Result<(), CliError> {
    let bytes = read_input(io.input.as_deref())?;
    let input: ContinuousBeamInput =
        serde_json::from_slice(&bytes).map_err(CliError::ParseInput)?;
    let result = calculate_continuous(&input, io.method.into()).map_err(CliError::Calc)?;
    write_json(io.output.as_deref(), &result, io.pretty)
}

// ============================================================================
// I/O helpers
// ============================================================================

fn read_input(path: Option<&Path>) -> Result<Vec<u8>, CliError> {
    // Treat `-` as an explicit stdin sentinel.
    let from_stdin = match path {
        None => true,
        Some(p) => p.as_os_str() == "-",
    };
    if from_stdin {
        let mut buf = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut buf)
            .map_err(|source| CliError::Io {
                op: "read stdin",
                path: "<stdin>".into(),
                source,
            })?;
        return Ok(buf);
    }
    let Some(p) = path else {
        // Unreachable: from_stdin is false only when path is Some.
        return Err(CliError::Io {
            op: "read input",
            path: "<none>".into(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "no input path"),
        });
    };
    fs::read(p).map_err(|source| CliError::Io {
        op: "read input file",
        path: p.display().to_string(),
        source,
    })
}

fn write_json<T: Serialize>(
    path: Option<&Path>,
    value: &T,
    pretty: bool,
) -> Result<(), CliError> {
    let serialized = if pretty {
        serde_json::to_string_pretty(value).map_err(CliError::SerializeOutput)?
    } else {
        serde_json::to_string(value).map_err(CliError::SerializeOutput)?
    };
    match path {
        Some(p) => fs::write(p, serialized).map_err(|source| CliError::Io {
            op: "write output file",
            path: p.display().to_string(),
            source,
        }),
        None => {
            let mut out = io::stdout().lock();
            out.write_all(serialized.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .map_err(|source| CliError::Io {
                    op: "write stdout",
                    path: "<stdout>".into(),
                    source,
                })
        }
    }
}

// ============================================================================
// Error surface
// ============================================================================

#[derive(Debug)]
enum CliError {
    ParseInput(serde_json::Error),
    SerializeOutput(serde_json::Error),
    Io {
        op: &'static str,
        path: String,
        source: io::Error,
    },
    Calc(CalcError),
}

impl CliError {
    fn exit_code(&self) -> ExitCode {
        match self {
            CliError::ParseInput(_) | CliError::Io { .. } => ExitCode::from(1),
            CliError::Calc(_) => ExitCode::from(2),
            CliError::SerializeOutput(_) => ExitCode::from(3),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            CliError::ParseInput(_) => "invalid_json_input",
            CliError::SerializeOutput(_) => "serialize_failed",
            CliError::Io { .. } => "io_error",
            CliError::Calc(_) => "calculation_error",
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::ParseInput(e) => write!(f, "invalid JSON input: {e}"),
            CliError::SerializeOutput(e) => write!(f, "could not serialize output: {e}"),
            CliError::Io { op, path, source } => write!(f, "{op} {path}: {source}"),
            CliError::Calc(e) => write!(f, "{e}"),
        }
    }
}

fn emit_error_json(e: &CliError) {
    let payload = serde_json::json!({
        "error": e.to_string(),
        "kind": e.kind(),
    });
    // If stderr is gone we can't surface anything anyway; intentionally drop.
    let _ = writeln!(io::stderr(), "{payload}");
}
