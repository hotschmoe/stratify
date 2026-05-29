//! Integration tests for the `stratify-cli` binary.
//!
//! These are golden JSON-in / JSON-out tests against the actual compiled
//! binary. They spawn the bin via `CARGO_BIN_EXE_calc_cli`, pipe a fixed
//! input on stdin, and assert on the parsed stdout JSON.
//!
//! Use these as smoke tests: they catch a regression in the JSON I/O
//! contract (renamed fields, changed exit codes, broken error envelope)
//! that a unit test on calc_core would miss.

use std::io::Write;
use std::process::{Command, Stdio};

use calc_core::calculations::beam::BeamResult;
use calc_core::calculations::continuous_beam::ContinuousBeamResult;

/// Path to the compiled CLI binary, supplied by Cargo to integration tests.
const CLI_BIN: &str = env!("CARGO_BIN_EXE_calc_cli");

/// 2x10 DF-L No.2, 12 ft simply-supported, 15 plf dead + 40 plf live.
/// A representative residential floor beam — chosen so reactions, moment,
/// and unity checks all land in the "passes by a comfortable margin" range.
const BEAM_INPUT_JSON: &str = r#"{
    "label": "T-1",
    "span_ft": 12.0,
    "load_case": {
        "label": "Floor",
        "loads": [
            {"load_type": "Dead", "magnitude": 15.0, "distribution": {"type": "UniformFull"}},
            {"load_type": "Live", "magnitude": 40.0, "distribution": {"type": "UniformFull"}}
        ],
        "include_self_weight": false
    },
    "material": {
        "type": "SawnLumber",
        "species": "DF-L",
        "grade": "No.2"
    },
    "width_in": 1.5,
    "depth_in": 9.25
}"#;

/// Two equal 10 ft spans, pinned at all three supports, 100 plf dead load.
/// Classic textbook case: interior support moment ≈ -wL²/8 = -1250 ft-lb.
///
/// Fixed UUIDs make the JSON deterministic (so a diff on stdout is meaningful).
const CONTINUOUS_INPUT_JSON: &str = r#"{
    "label": "CB-1",
    "spans": [
        {
            "id": "00000000-0000-0000-0000-000000000001",
            "length_ft": 10.0,
            "width_in": 1.5,
            "depth_in": 9.25,
            "material": {"type": "SawnLumber", "species": "DF-L", "grade": "No.2"}
        },
        {
            "id": "00000000-0000-0000-0000-000000000002",
            "length_ft": 10.0,
            "width_in": 1.5,
            "depth_in": 9.25,
            "material": {"type": "SawnLumber", "species": "DF-L", "grade": "No.2"}
        }
    ],
    "supports": ["Pinned", "Pinned", "Pinned"],
    "load_case": {
        "label": "Dead",
        "loads": [
            {"load_type": "Dead", "magnitude": 100.0, "distribution": {"type": "UniformFull"}}
        ],
        "include_self_weight": false
    }
}"#;

/// Spawn the CLI with the given subcommand args, write `stdin_bytes` to
/// stdin, and return (exit_code, stdout, stderr).
fn run_cli(args: &[&str], stdin_bytes: &[u8]) -> (i32, String, String) {
    let mut child = Command::new(CLI_BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn calc_cli");

    child
        .stdin
        .as_mut()
        .expect("stdin pipe missing")
        .write_all(stdin_bytes)
        .expect("failed to write stdin");

    let out = child
        .wait_with_output()
        .expect("failed to wait for calc_cli");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(out.stdout).expect("stdout not UTF-8");
    let stderr = String::from_utf8(out.stderr).expect("stderr not UTF-8");
    (code, stdout, stderr)
}

#[test]
fn beam_subcommand_emits_valid_result_json() {
    let (code, stdout, stderr) = run_cli(&["beam"], BEAM_INPUT_JSON.as_bytes());
    assert_eq!(code, 0, "expected exit 0, got {code}; stderr: {stderr}");
    assert!(
        stderr.is_empty(),
        "stderr should be empty on success: {stderr}"
    );

    let result: BeamResult =
        serde_json::from_str(stdout.trim()).expect("stdout should parse as BeamResult");

    // Section properties are pure geometry: S = bd²/6, I = bd³/12.
    let expected_s = 1.5 * 9.25_f64.powi(2) / 6.0;
    let expected_i = 1.5 * 9.25_f64.powi(3) / 12.0;
    assert!((result.section_modulus_in3 - expected_s).abs() < 1e-9);
    assert!((result.moment_of_inertia_in4 - expected_i).abs() < 1e-9);

    // Governing ASD combination on D+L (15 + 40 plf): max moment = wL²/8.
    let expected_moment_ftlb = 55.0 * 12.0_f64.powi(2) / 8.0; // 990 ft-lb
    assert!(
        (result.max_moment_ftlb - expected_moment_ftlb).abs() < 1.0,
        "max moment {} vs expected {}",
        result.max_moment_ftlb,
        expected_moment_ftlb
    );

    // 2x10 DF-L No.2 at 12' with 55 plf is well within capacity — sanity check.
    assert!(result.bending_unity > 0.0 && result.bending_unity < 1.0);
    assert!(result.shear_unity > 0.0 && result.shear_unity < 1.0);
    assert!(result.passes(), "beam should pass all checks");
}

#[test]
fn beam_method_flag_changes_design_load() {
    // ASD baseline.
    let (asd_code, asd_stdout, _) = run_cli(&["beam"], BEAM_INPUT_JSON.as_bytes());
    assert_eq!(asd_code, 0);
    let asd: BeamResult = serde_json::from_str(asd_stdout.trim()).unwrap();

    // LRFD load combo is 1.2D + 1.6L for this case; should be heavier than D + L.
    let (lrfd_code, lrfd_stdout, _) =
        run_cli(&["beam", "--method", "lrfd"], BEAM_INPUT_JSON.as_bytes());
    assert_eq!(lrfd_code, 0);
    let lrfd: BeamResult = serde_json::from_str(lrfd_stdout.trim()).unwrap();

    assert!(
        lrfd.design_load_plf > asd.design_load_plf,
        "LRFD design load ({}) should exceed ASD ({})",
        lrfd.design_load_plf,
        asd.design_load_plf
    );
    assert!(
        lrfd.governing_combination.starts_with("LRFD"),
        "LRFD combination name should start with 'LRFD', got {}",
        lrfd.governing_combination
    );
}

#[test]
fn continuous_beam_subcommand_returns_textbook_interior_moment() {
    let (code, stdout, stderr) = run_cli(&["continuous-beam"], CONTINUOUS_INPUT_JSON.as_bytes());
    assert_eq!(code, 0, "expected exit 0, got {code}; stderr: {stderr}");

    let result: ContinuousBeamResult =
        serde_json::from_str(stdout.trim()).expect("stdout should parse as ContinuousBeamResult");

    // Two equal spans with uniform load: interior support carries the negative
    // peak. Closed-form (Hibbeler Ch. 11) gives |M_int| = wL²/8 = 1250 ft-lb.
    // Hardy Cross converges to this; allow a small tolerance.
    let expected = 100.0 * 10.0_f64.powi(2) / 8.0; // 1250
    let interior = result.max_negative_moment_ftlb.abs();
    assert!(
        (interior - expected).abs() < 50.0,
        "interior moment {} vs textbook {}",
        interior,
        expected
    );
}

#[test]
fn malformed_json_input_returns_exit_1_with_typed_error() {
    let (code, stdout, stderr) = run_cli(&["beam"], b"{not valid json");
    assert_eq!(code, 1, "malformed JSON should exit 1");
    assert!(stdout.is_empty(), "no result on malformed input");

    let err: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be JSON envelope");
    assert_eq!(err["kind"], "invalid_json_input");
    assert!(err["error"].is_string());
}

#[test]
fn invalid_calc_input_returns_exit_2_with_typed_error() {
    // Negative span trips calc_core's validate(); the CLI should surface it as
    // a calculation error (exit 2), not a JSON parse error.
    let bad_input = BEAM_INPUT_JSON.replace("\"span_ft\": 12.0", "\"span_ft\": -5.0");
    let (code, stdout, stderr) = run_cli(&["beam"], bad_input.as_bytes());
    assert_eq!(code, 2, "calc error should exit 2");
    assert!(stdout.is_empty());

    let err: serde_json::Value = serde_json::from_str(stderr.trim()).expect("stderr JSON");
    assert_eq!(err["kind"], "calculation_error");
}
