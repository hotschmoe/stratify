# Agent Instructions - Stratify

> This file is the canonical instruction set for any AI coding agent working on Stratify. The repo exposes it under both `CLAUDE.md` (Claude Code) and `AGENTS.md` (cross-tool convention, e.g. Codex); the latter is a symlink to the former. **CLAUDE.md is the master - edit it, not the symlink.**

## RULE 1 - ARCHIVE INSTEAD OF DELETE

You must MOVE files and directories to the `archive/` directory instead of deleting them. The `archive/` directory is gitignored and serves as a staging area for cleanup.

- This applies to all files you want to remove (tests, tmp files, scripts, obsolete code, etc.).
- Use `mv <path> archive/` instead of `rm` or `rm -rf`. Create the `archive/` directory if it doesn't exist.
- A human will routinely review the `archive/` directory and permanently delete files from there.
- You do not need explicit permission to move files to `archive/` when cleaning up.

Treat "never permanently delete files, always move to archive" as a hard invariant.

---

### IRREVERSIBLE GIT & FILESYSTEM ACTIONS

Absolutely forbidden unless the user gives the **exact command and explicit approval** in the same message:

- `git reset --hard`
- `git clean -fd`
- `rm -rf`
- Any command that can permanently delete or overwrite code/data

Rules:

1. If you are not 100% sure what a command will affect, do not propose or run it. Ask first.
2. Prefer safe tools: `git status`, `git diff`, `git stash`, or moving to `archive/`.
3. If an actual destructive command is explicitly authorized by the user, record in your response:
   - The exact user text authorizing it
   - The command run
   - When you ran it

If that audit trail is missing for a destructive action, then you must act as if the operation never happened.

### Version Updates (SemVer, master-only)

`Cargo.toml`'s `[workspace.package] version` follows [Semantic Versioning](https://semver.org/) **at the master branch**, not on every dev commit. Dev branches inherit master's version and only bump it in the final pre-merge commit (or the merge commit itself). The bump magnitude reflects the *cumulative* diff being merged into master.

Pre-v1.0, the minor track is **coupled to feature milestones**: while v0.X work is in progress, master sits at `0.X.Y`. The first commit of v0.(X+1) work to land on master bumps to `0.(X+1).0`. Patches accumulate within a milestone.

- **MAJOR** (X.0.0): breaking CLI shape, breaking project-file (`.stf`) schema, breaking calc_core public API - rare pre-v1.0 (current X is 0)
- **MINOR** (0.X.0): milestone advances - first commit of the next v0.X cycle on master
- **PATCH** (0.X.Y): every other master merge - bug fixes, refactors, doc changes, internal cycles within the in-progress milestone

Tag master with `v0.X.Y` when the bump lands. CI (`cargo test --workspace` + `cargo clippy` + WASM check) is the release gate; there is no separate release branch. **Dev-branch hygiene:** during work on a dev branch, leave `version` alone - the bump is a single deliberate edit at PR time.

The `.stf` project file's `SCHEMA_VERSION` (see `calc_core/src/project.rs`) is a separate version line. It bumps when the saved JSON shape changes in a way that breaks load. Bump it independently of the workspace version.

---

### Commits at Milestones (Save Points)

Commits at logical save points are encouraged. You may commit at your discretion when work hits a clean checkpoint - no need to wait for explicit per-commit approval. Good save points: a focused refactor with `cargo test --workspace` green, a new module landing with tests passing, a non-trivial doc/RFC update, the end of a logical chunk worth bisecting to.

Discipline:
- HEAD must compile and pass `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic`. Never commit a broken state to mark progress - use a task or a `.note` instead.
- Commits go on the current dev branch. Never to `master` directly.
- Never `git push` without explicit user authorization.
- Stage only files the change actually touches; avoid `git add -A` / `git add .`.
- Use the HEREDOC commit-message form with an `Assisted-By: <your model identifier and context window>` trailer - e.g. `Assisted-By: Claude Opus 4.7 (1M context)` or `Assisted-By: Codex GPT-5 (200K context)`. (Per LLVM's AI Tool Policy convention - "assisted by," not "co-authored by," since AI output isn't a copyrightable authorship claim.)
- Destructive git ops (`reset --hard`, `push --force`, `branch -D`) still require explicit authorization.

If unsure whether a state is commit-worthy, default to committing - small atomic commits are cheaper to revert than a tangled WIP.

---

### Code Editing Discipline

- Do **not** run scripts that bulk-modify code (codemods, invented one-off scripts, giant `sed`/regex refactors).
- Large mechanical changes: break into smaller, explicit edits and review diffs.
- Subtle/complex changes: edit by hand, file-by-file, with careful reasoning.
- **NO EMOJIS** - do not use emojis or non-textual characters.
- ASCII diagrams are encouraged for visualizing flows.
- Keep in-line comments to a minimum. Architecture/design lives in module docstrings and `docs/` notes; in-line comments are for non-obvious *why*, not *what*.

---

### No Legacy Code - Full Migrations Only

We optimize for clean architecture, not backwards compatibility. **When we refactor, we fully migrate.**

- No "compat shims", "v2" file clones, or deprecation wrappers
- When changing behavior, migrate ALL callers and remove old code **in the same commit**
- No `_legacy` suffixes, no `_old` prefixes, no "will remove later" comments
- New files are only for genuinely new domains that don't fit existing modules
- The bar for adding files is very high

**Rationale**: Legacy compatibility code creates technical debt that compounds. A clean break is always better than a gradual migration that never completes.

### No Hardcoded Shortcuts

Every hardcoded value is a deferred decision the compiler can't see. The fix isn't "remember to come back to it later" - it's structuring the system so the choice has to be made visibly at the right level.

- **calc_core ships generic primitives.** Sealed `enum`s for species/grade/load-type, structured `BeamInput`/`ContinuousBeamInput` records, NDS factor structs. Not "the common case + an escape hatch we'll generalise later."
- **User values live in user source.** Loads, factors, dimensions are declared as named fields in JSON inputs or in `Project` instances. calc_core never synthesises a magic "default beam" that hides where a value came from.
- **No "ship subset, fix later."** If the design demands the full NDS 2018 Table 4A species/grade matrix, generate all of it from `data/wood/sawn_lumber/*.toml` from day one - not the five most common entries. A subset means downstream consumers (GUI dropdowns, JSON-schema export, LLM tool-calling surfaces) all carry the same gap.
- **No magic numbers in fixtures.** Every literal in example `.stf` files or doc-comment JSON should either be a named constant in source or come from a `Material::default()` style helper that documents the choice.
- **Forbidden phrases**: "we'll generalise this later," "v0.X cleanup," "good enough for now," "TODO: lift this constant."

---

## Specialized Subagents (tool-specific)

If your tool supports subagent definitions (Claude Code does, via `.claude/agents/` with YAML frontmatter - `name`, `description`, `model`, `tools`), our pre-defined subagents live there. Other tools without subagent infrastructure can ignore this section.

After long sessions or large refactors, consider running the code-simplifier subagent - it focuses on recently modified files and reduces complexity accumulated during extended development, preserving functionality.

---

# PROJECT-LANGUAGE-SPECIFIC: Stratify (Rust 2021)

## Read this first

Stratify is a structural engineering calculation suite. Three workspace crates:

- **`calc_core`** - the calculation engine. Pure functions; JSON-serializable inputs and outputs; structured error types. This is the LLM-facing surface and the source of truth for every formula.
- **`calc_gui`** - Iced 0.14 GUI (native + WASM via wgpu/WebGPU). Authors interact here; the GUI is a thin shell over calc_core.
- **`calc_cli`** - clap-based JSON-in / JSON-out CLI. The natural integration surface for LLMs, scripts, and MCP-style tool calling. **Not** a TUI - the original Ratatui direction was deferred indefinitely in favor of a JSON-only CLI (see "CLI direction" below).

The non-negotiable constraints:

- Every formula traces to a code provision (NDS, ASCE 7, AISC, ACI 318). No hidden math; the relevant clause is cited in the surrounding module docstring or NDS reference constant.
- Inputs and outputs are JSON-serializable. `Serialize + Deserialize` on every public type is mandatory, not optional.
- Errors are structured (`thiserror` enum variants), not strings.
- The GUI is a presentation of calc_core, not a parallel implementation. Calculation logic NEVER lives in `calc_gui` source files.

## Workspace Layout

```
stratify/
  Cargo.toml                          # workspace root: deps, profiles, lints
  .cargo/config.toml                  # aliases (c, t, cli, gui, audit-unwraps)
  rust-toolchain.toml                 # pinned stable + clippy + rustfmt + wasm32
  bacon.toml                          # live-feedback jobs
  calc_core/                          # calculation engine
    Cargo.toml                        # features: pdf (Typst, opt-in), pdf-extract, json-schema
    build.rs                          # TOML material data -> generated Rust source
    src/calculations/                 # beam, beam_analysis, column, continuous_beam, moment_distribution
    src/equations/                    # documented statics formulas (Roark's references)
    src/materials/                    # sawn_lumber, engineered_wood, steel, lumber_sizes
    src/loads/                        # load_types, combinations (ASCE 7), discrete, EnhancedLoadCase
    src/generated/                    # build-script output - never hand-edit
    src/pdf.rs                        # gated by `pdf` feature
    src/file_io.rs                    # atomic save + std::fs::File locking (no fs2/fs4)
  calc_gui/                           # Iced 0.14 native + WASM
    Cargo.toml                        # enables calc_core/pdf
    index.html                        # Trunk entrypoint
  calc_cli/                           # clap JSON CLI
    Cargo.toml                        # calc_core without `pdf` feature
  assets/                             # fonts (Berkeley Mono), steel/AISC data
  archive/                            # gitignored cleanup staging (RULE 1)
```

Per-module purpose lives in module docstrings. Read those, not a duplicate listing here.

## Rust Toolchain

- **Rust**: stable (currently 1.95+), pinned via `rust-toolchain.toml`
- **Edition**: 2021 (edition 2024 migration is a backlog item)
- **Target host**: Windows ARM64 (`aarch64-pc-windows-msvc`) is the primary dev target; Linux + Windows x86_64 + macOS are CI/release targets; `wasm32-unknown-unknown` is a first-class target for the GUI

### Build Commands

```bash
cargo c                              # check workspace (alias for `check --workspace --all-targets`)
cargo t                              # test workspace (alias for `test --workspace`)
cargo cli                            # check just calc_cli (alias - SKIPS Typst graph, fast)
cargo gui                            # run calc_gui with dev-fast profile (alias)
cargo audit-unwraps                  # deny unwrap/expect/panic in lib+bin code
cargo build --target wasm32-unknown-unknown -p calc_gui    # WASM build (Trunk handles bundling)
```

The `audit-unwraps` alias and the `clippy::unwrap_used`/`expect_used`/`panic` lints in `[workspace.lints.clippy]` are the discipline for keeping production code panic-free. Test code is allowed to use them freely.

### Build Speed Notes (Windows ARM64)

- `rustc_codegen_cranelift` is **not distributed** for `aarch64-pc-windows-msvc` as of 2026. Do not try to enable it.
- `rust-lld` was explicitly removed as the default linker on this target (rust#54290). Stay on the bundled lld-link / MSVC link.exe.
- LLVM (clang) is required for `ring`'s C build script. Install via `winget install LLVM.LLVM` and put `C:\Program Files\LLVM\bin` on PATH before running cargo.
- `cargo check -p calc_cli` cold builds in ~10s (Typst feature gate keeps the heavy dep graph out). Workspace cold is ~40s.
- `cargo run --profile dev-fast -p calc_gui` skips debuginfo for the fastest GUI iteration.

If you change these, document the reason in `.cargo/config.toml` or the workspace `Cargo.toml` profile section - the rejection rationale matters as much as the choice.

### CLI direction

`calc_cli` is a **JSON-in / JSON-out CLI built on `clap`**. Not a TUI. The original `ratatui` direction was deferred indefinitely in May 2026 - rationale:

- Calc_core's whole pitch is "JSON-first, LLM-friendly." A JSON CLI is the natural MCP-style tool-calling surface.
- Iced GUI already covers the interactive-engineer use case. A second interactive UI adds maintenance surface for minor user benefit.
- LLMs / scripts / external tools speak JSON. They don't speak Ratatui.

If a TUI ever becomes necessary, it should be a separate crate (`calc_tui` or similar) that imports calc_cli or calc_core, not a replacement for the JSON CLI.

---

## Compilation Pipeline

```
JSON input (stdin or --input PATH)
  -> calc_cli::main           (clap subcommand dispatch)
  -> serde_json::from_slice   (-> BeamInput | ContinuousBeamInput | ...)
  -> calc_core::calculations  (pure function: input -> Result<output, CalcError>)
  -> serde_json::to_string    (output struct -> JSON)
  -> stdout (or --output PATH)
```

Errors are emitted to stderr as JSON for machine consumption:
```
{"error":"<message>","kind":"<machine-readable-kind>"}
```

GUI path is the same calc_core call wrapped in an Iced `Message::Calculate` handler. PDF path runs after calc, only enabled when the `pdf` feature is on.

When a new pipeline stage lands (or an existing one shifts), update this diagram in the same commit.

---

## Source-of-Truth Discipline

The compiler is the source of truth for data:

- `calc_core/src/generated/material_data.rs` is produced by `calc_core/build.rs` from `calc_core/data/wood/**/*.toml`. **Never hand-edit it.** If the generated content disagrees with the TOML, fix the generator or the TOML, not the output.
- NDS reference values (`F_b`, `F_v`, `E`, `E_min` per species/grade) live in TOML. Adding a species or grade is a data edit + rebuild, not a code change.
- The JSON I/O shape is the calc_core public API. Breaking changes to serde-derived structs are user-visible breaking changes. Treat `BeamInput`, `ContinuousBeamInput`, `BeamResult`, `ContinuousBeamResult`, `Project` field renames as MAJOR-version-bump events.

Never hand-edit generated artifacts. If a generated file disagrees with the spec, fix the generator.

---

## Rust Best Practices (calc_core + calc_gui + calc_cli)

Stratify is a regular Rust 2021 workspace. The load-bearing patterns:

- **Stateless calculations.** Pure functions that take `BeamInput`, return `Result<BeamResult, CalcError>`. No global state. No side effects. This is what makes calc_core swappable into the GUI, the CLI, and (eventually) MCP servers.
- **`Result<T, CalcError>` everywhere.** Public API never returns plain `Result<T, String>` or `anyhow::Result`. The CalcError enum is documented and JSON-serializable - LLM consumers depend on the variants being stable.
- **No `unwrap()` / `expect()` / `panic!` in lib + bin code.** Use `?` with the `CalcError` enum, or `.ok_or_else(|| CalcError::...)`. The `cargo audit-unwraps` alias and CI audit job enforce this discipline. Test code is exempt.
- **Builder-pattern construction.** `BeamInput::new(...).with_material(...).with_loads(...)`. Avoid 12-positional-argument constructors.
- **`#[serde(default)]` on optional fields.** If a field is not strictly required for calculation, make JSON consumers able to omit it. Critical for CLI ergonomics and JSON-schema export.
- **Module docstrings document the formula.** Every calculation module's `//!` header cites the NDS / ASCE 7 / AISC clause it implements and shows a runnable example. The `///` docstring on the calculation function repeats the relevant equation.

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum CalcError {
    #[error("Invalid input for {field}: {value} - {reason}")]
    InvalidInput { field: String, value: String, reason: String },
    #[error("Material not found: {0}")]
    MaterialNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    // ...
}

pub fn analyze_beam(input: BeamInput) -> Result<BeamResult, CalcError> {
    if input.span_ft <= 0.0 {
        return Err(CalcError::invalid_input("span_ft", input.span_ft.to_string(), "must be positive"));
    }
    // ...
}
```

### Option handling

```rust
// Prefer ok_or_else over unwrap()
let material = materials.get(&name)
    .ok_or_else(|| CalcError::MaterialNotFound(name.clone()))?;

// Use unwrap_or_default() / unwrap_or() for safe defaults
let deflection_limit = config.deflection_limit.unwrap_or(360.0);
```

---

## Testing Philosophy: Diagnostics, Not Verdicts

**Tests are diagnostic tools, not success criteria.** A passing test suite does not mean the calculations are correct. A failing test does not mean the code is wrong.

When a test fails, ask three questions in order:
1. Is the test itself correct and valuable? (Does it match the NDS / ASCE 7 / AISC clause?)
2. Does the test align with our current design? (Did we deliberately change a struct shape?)
3. Is the code actually broken?

Only if all three answers are yes should you fix the code. Tests encode assumptions, and structural engineering math has reference values - outdated assumptions get discarded, not appeased.

**What tests ARE good for here**: closed-form formula verification (Roark's beam tables, NDS Appendix C), JSON round-trip / idempotency properties, doc-tests on calc_core's public API, smoke tests per crate.

**What tests are NOT**: a definition of correctness (the NDS / ASCE 7 / AISC clauses are), a measure of calculation quality, something to "make pass" at all costs.

### Test Coverage Baseline

As of May 2026:
- `calc_core`: 221 unit tests + 40 doc tests. Good coverage of beam, continuous_beam, equations, loads, materials.
- `calc_gui`: 0 tests (gap - a smoke test on default app state would be cheap).
- `calc_cli`: 0 tests (gap - golden JSON I/O tests against fixed inputs would be cheap).

When adding a calculation: write the test before the implementation. The test should encode the closed-form expected value with the NDS / Roark reference cited in a comment.

---

## Testing Guidelines

```bash
cargo test --workspace                      # all tests
cargo test -p calc_core                     # calc_core only (no Iced/wgpu rebuild)
cargo test --workspace -- --nocapture       # see println! / dbg! output
cargo test --workspace -- --ignored         # run slow tests
```

### Adding a Test

- Module-level invariants: inline `#[cfg(test)] mod tests` block in the module file. Picked up automatically.
- Doc-tests: triple-slash `///` example block on the public function. These are user-facing examples and run as tests.
- Cross-module / integration tests: file in `<crate>/tests/`.

---

## Common Development Workflows

### Adding a Material (sawn lumber, glulam, LVL, PSL)

1. Edit the relevant `calc_core/data/wood/**/*.toml` file. Cite the NDS clause in the TOML comment.
2. `cargo build -p calc_core` regenerates `src/generated/material_data.rs`.
3. The material is automatically available via `Material::lookup()` and in GUI dropdowns. No code change needed.

### Adding a Calculation Type

1. Define `*Input` and `*Result` structs in `calc_core/src/calculations/`. Both must derive `Serialize + Deserialize + Debug + Clone`.
2. Implement the calculation as a pure function: `pub fn calculate(input: &MyInput, method: DesignMethod) -> Result<MyResult, CalcError>`. Cite NDS / ASCE 7 clauses in the doc-comment.
3. Add unit tests against a worked example (Roark, NDS Appendix C, AISC manual). Cite the source in a comment.
4. Re-export the public types at the calc_core crate root if they form part of the public API.
5. Wire into calc_gui (input panel + result panel) and calc_cli (clap subcommand) in separate commits.

### Adding a CLI Subcommand

1. Add a variant to `Command` enum in `calc_cli/src/main.rs` with a doc comment that becomes `--help` text.
2. Add a `run_<name>` function modelled on `run_beam` / `run_continuous_beam`.
3. The function reads input bytes, deserializes into the appropriate calc_core input type, calls the calculation, and writes the result via `write_json`. No business logic in the CLI.

### Adding an Iced GUI Panel

1. New file in `calc_gui/src/ui/input_<type>.rs` or `result_<type>.rs`.
2. Add `Message` variants for user inputs.
3. Wire into `update` and `view` in `calc_gui/src/main.rs`.
4. The panel reads calc_core types and emits calc_core types - never duplicates calc logic.

---

## CI Matrix

Per-commit on PR + push-to-master (`.github/workflows/ci.yml`):

| Job | Runner | Notes |
|-----|--------|-------|
| `fmt` | ubuntu-latest | `cargo fmt --all --check` |
| `clippy` | ubuntu-latest | `-D warnings` with unwrap/expect/panic carve-outs |
| `test (ubuntu)` | ubuntu-latest | full workspace tests |
| `test (windows)` | windows-latest | full workspace tests |
| `wasm` | ubuntu-latest | `cargo check --target wasm32-unknown-unknown -p calc_gui` |
| `audit-unwraps` | ubuntu-latest | non-blocking; surfaces unwrap/expect/panic count on PR |

`release.yml` (separate) handles tagged release builds: native binaries for Win/Linux/Mac (x86_64 and aarch64), WASM via Trunk, GitHub Pages deploy.

---

## Development Philosophy

**Make it work, make it right, make it fast** - in that order. Ship the working calculation first, harden the API after, optimize last.

**Closed-loop testing**: each landed change ends with a green `cargo c && cargo t` and a clean `cargo clippy --workspace --all-targets -- -D warnings -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic`.

**Boring inputs over edge cases**. Exercise calc_core by running JSON inputs that match real-world member sizes (2x10 DF-L No.2, 5-1/8" x 16-1/2" 24F-V4 glulam, common spans 8-24 ft) - these surface ergonomic issues and bad defaults faster than synthetic edge cases.

---

## Related Projects (context, not dependencies)

- **`../zenercalc`** is hotschmoe's prior Zig-based structural calculator. Its JSON-in / JSON-out CLI shape (stdin -> dispatch by `"module"` -> stdout JSON) informs Stratify's `calc_cli`. Differences: Stratify uses clap subcommands for discoverability (`stratify-cli beam --help`) rather than `"module"` field detection, but the JSON I/O contract is the same.
- **`../numen`** is hotschmoe's Numen language project. Stratify's `CLAUDE.md` structural rules (RULE 1 archive, version SemVer, code editing discipline, no legacy, no hardcoded shortcuts) were adopted from Numen's instruction set. Not a runtime dependency.

---

we love you! do your best today
