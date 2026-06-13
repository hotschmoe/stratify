# Compile-Speed Benchmark & Tuning

> Measured 2026-06-12 on the **x86_64 desktop** (Intel Core Ultra 7 270K Plus,
> Windows 11, `x86_64-pc-windows-msvc`, stable rustc 1.96.0). The aarch64 laptop
> is a separate target and is **not** measured here; cross-machine notes are called
> out where they matter.

This document records (a) the cold/warm build baselines, (b) the changes applied to
speed up the dev loop, (c) the after numbers, and (d) the levers we evaluated and
**rejected**, with the data behind each call. Reproduction scripts live in `archive/`
(`bench_*.ps1`) and are gitignored staging per RULE 1.

---

## TL;DR

1. **The dev loop's biggest cost is feature-unification thrash, not codegen or
   linking.** Alternating `cargo build -p calc_core`/`cargo cli` (Typst `pdf` feature
   **off**) with `cargo c`/`cargo build -p calc_gui`/`cargo t` (`pdf` **on**) forces a
   full ~10 s recompile of `calc_core` on the first build of each variant after every
   edit, because Cargo keeps a **separate incremental cache per feature set**. Stay in
   one feature lane while iterating and the same edit rebuilds in **~3 s**.
2. **`lld-link` is the top stable codegen-side win** on x86_64 — it speeds the
   *link* step on every exe build (GUI/CLI). Applied.
3. **`debug = false` for dependencies** trims cold-build and link time at zero cost
   (we never step into third-party crates). Applied.
4. **Nightly backends (Cranelift, parallel front-end) are not worth it here.** They
   only help full/cold recompiles, where Cranelift can't be used for the GUI anyway
   (forces `panic=abort`); avoiding thrash beats all of them. Kept as a documented,
   commented-out opt-in only.
5. **No crate splitting, no `opt-level` change, no forced feature unification** —
   each was evaluated and rejected with reasons below.

---

## Environment installed (this desktop was bare)

| Component | Version | Notes |
|---|---|---|
| rustup + stable | rustc/cargo 1.96.0 | targets `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown`, clippy, rustfmt |
| LLVM / clang / lld-link | 22.1.7 | clang for `ring`'s C build (CLAUDE.md mandate); `lld-link` now used as the x86_64 linker. Added to user PATH. |
| VS Build Tools 2022 | VCTools + Win11 SDK | MSVC `link.exe` + SDK (required to link MSVC target) |
| nightly (eval only) | rustc 1.98.0-nightly | + `rustc-codegen-cranelift` component; **not** adopted, used only to benchmark opt-in levers |

---

## Cold build (from `cargo clean`, full dependency graph per crate)

Each crate cleaned before building, so the number includes its whole dep graph.

| Build | Baseline | After (lld + deps `debug=false`) | Δ |
|---|---:|---:|---:|
| `calc_core` | 14.3 s | 11.4 s | −20% |
| `calc_cli`  | 15.7 s | 15.8 s | ~0 |
| `calc_gui`  | 135.9 s | 121.5 s | −11% |
| workspace   | 126.9 s | 135.3 s | within run noise |

Heavy units dominating the cold GUI build (`cargo build --timings`, top by self-time):
`typst-library` 55 s · `naga` 44 s · `write-fonts`/`read-fonts`/`skrifa` (fonts) ·
`image` 36 s · `wgpu-core`/`wgpu-hal` · `hayagriva`/`citationberg` (Typst bibliography)
· `hayro-*` (Typst PDF). Three independent heavy clusters: **Typst/PDF**, **wgpu/GPU**,
**fonts/image**. Cold-build improvement comes from deps carrying no debuginfo; the heavy
builds (`gui`, `workspace`) have ~±10% thermal run-to-run variance, so treat the cold
deltas as "modest but real," not precise.

---

## Warm / incremental dev loop (the number that actually matters)

**Methodology note.** Early "warm" numbers were misleading because of two traps:
(1) a freshly-primed artifact and an `mtime`-only touch can race so Cargo skips the
rebuild (no-op, ~0.5 s), and (2) **feature thrash** (below) masquerades as a slow
incremental. The numbers here use **real single-line source edits** and stay in a
fixed feature lane unless the row is explicitly testing the thrash.

### True incremental — edit `calc_core`, stay in one feature lane (pdf ON)

| Action after a real `calc_core` edit | Time |
|---|---:|
| `cargo build --workspace` (recompile core + relink gui & cli via lld) | **3.2 s** |
| `cargo build -p calc_gui` (recompile core + relink gui) | **2.9 s** |
| `cargo check -p calc_core` (lib only, no codegen) | **~1 s** |
| no-op (nothing changed) | 0.3–0.4 s |

This is the real dev loop: **~1 s to type-check, ~3 s to a runnable binary.** It is
already fast; lld-link is why the relink portion is cheap.

> `cargo c` (the alias = `check --workspace --all-targets`) is **not** the fast inner
> loop — `--all-targets` also compiles every test/bench target (calc_core's 221 unit
> tests), which is tens of seconds on first run. For tight iteration use
> `bacon check-core` / `cargo check -p calc_core` (lib only), or `bacon` keyed jobs.

### Feature thrash — the trap to avoid

Switching the `pdf` (Typst) feature on/off between commands, **no code change**:

| Sequence | Time | Why |
|---|---:|---|
| `build --workspace` (prime, pdf ON cached) → `build -p calc_core` (pdf **OFF**) | **10.8 s** | first build of the pdf-off variant — full recompile |
| → `build --workspace` (pdf **ON** again) | 0.9 s | pdf-on variant still cached |
| → `build -p calc_core` (pdf **OFF** again) | 0.3 s | pdf-off variant now cached too |

Cargo fingerprints each feature set separately and keeps a **separate incremental
cache** for each. The ~10 s is paid once per variant — **but editing `calc_core`
invalidates both variants**, so if you exercise both lanes after each edit you pay the
full recompile twice per edit. `calc_gui` pulls `calc_core/pdf`; `calc_cli` and bare
`calc_core` do not — so `--workspace`/`-p calc_gui`/`cargo t` are the **pdf-ON lane**
and `-p calc_core`/`-p calc_cli`/`cargo cli` are the **pdf-OFF lane**.

**Rule of thumb:** pick a lane for the task and stay in it. GUI/engine work →
pdf-ON commands. CLI work → pdf-OFF commands. Don't ping-pong.

---

## Changes applied (stable, committed)

All low-risk, target-scoped, and reversible. Cross-machine safe: the aarch64 laptop
uses a different `[target.*]` block and is unaffected.

### 1. `lld-link` linker for x86_64 — `.cargo/config.toml`
```toml
[target.x86_64-pc-windows-msvc]
linker = "lld-link.exe"
```
LLVM's MSVC-compatible LLD is markedly faster than MSVC `link.exe` on the
edit→rebuild loop (linking runs on every build). Validated against the full
Iced/wgpu/Typst link graph (Bevy ships LLD as its default Windows linker for the same
dependency shape). Requires `C:\Program Files\LLVM\bin` on PATH — already a project
mandate (CLAUDE.md: `ring`'s C build needs clang). **Switching linkers forces one full
rebuild** (the linker is part of the build fingerprint); incremental is fast after.
*CI note:* GitHub `windows-latest` ships LLVM on PATH, so the committed setting should
work in CI; watch the first `windows` job on the next PR and, if `lld-link` is absent,
either add an LLVM step or drop the one line.

### 2. Dependencies carry no debuginfo — `Cargo.toml`
```toml
[profile.dev.package."*"]
opt-level = 3
debug = false        # added
```
We never step into third-party crates, so their debuginfo is dead weight on every cold
build and link. Pure win. (`opt-level = 3` retained — see "rejected" below.)

### 3. On-demand full debuginfo — `Cargo.toml`
```toml
[profile.debugging]
inherits = "dev"
debug = true
```
`cargo build --profile debugging -p calc_gui` when you actually need variable/type
inspection in a debugger, without slowing the default loop.

---

## Levers evaluated and REJECTED (with data)

### Cranelift codegen backend (nightly)
- **Does it work here?** Yes — `calc_core` built cleanly under Cranelift on x86_64
  Windows (exit 0), contradicting the common "panic=abort breaks everything" worry
  *for a library build*.
- **Full `calc_core` recompile:** stable-LLVM 12.1 s → Cranelift **8.7 s (−29%)**.
- **On the actual dev loop (incremental):** no benefit — a ~1–3 s incremental has
  almost no codegen to accelerate (Cranelift 1.06 s vs LLVM 1.01 s).
- **Verdict: reject as default.** Nightly (we pin stable); forces `panic=abort`, which
  is unsafe for the Iced GUI's unwinding on Windows, so it could only ever apply to
  `calc_core`/`calc_cli`; and avoiding thrash (3 s) already beats its 8.7 s full build.
  Left as a commented opt-in for engine-only experimentation.

### Parallel rustc front-end `-Zthreads` (nightly)
- **Full `calc_core` recompile:** nightly-LLVM 12.2 s → `-Zthreads=8` **10.2 s (−16%)**.
- **On the dev loop (incremental):** noise (0.58 s vs 0.56 s) — nothing to parallelize.
- **Verdict: reject as default.** Nightly-only; helps clean/full builds, which we
  attack by *not* triggering them. Commented opt-in in `.cargo/config.toml`.

### `opt-level = 1` for dependencies (instead of 3)
- The single biggest *cold*-build lever (the Typst/wgpu graph is built at `-O3`), but
  `opt-level = 3` is a **deliberate runtime choice** for smooth wgpu/Typst in debug,
  and it does **not** affect the warm loop (deps are cached, never recompiled on a
  workspace edit). Lowering it trades GUI debug-runtime smoothness for cold-build time.
- **Verdict: keep `3`.** Documented the tradeoff in `Cargo.toml`. Flip to `1` only if
  cold-build time starts to outweigh debug GUI feel — it's a one-line change.

### `feature-unification = "workspace"` / forcing `pdf` everywhere
- Would eliminate the thrash by making `calc_core`'s feature set identical across
  commands — but only by compiling **Typst into the CLI lane too**, which is the exact
  opposite of the design (the CLI is deliberately Typst-free and cold-builds in ~16 s).
- **Verdict: reject.** The thrash is better solved by lane discipline than by making
  every command pay for Typst.

### Splitting `calc_core` into sub-crates
- Tempting seams exist (`materials/`, `equations/`, generated data), but the modules
  are tightly coupled (`calculations → materials → nds_factors`), `calc_core` is modest
  (~17 K LOC), it cold-builds in ~11–14 s, and a true incremental edit is already ~3 s.
  Splitting adds per-crate link/coordination overhead for a likely *negative* warm-loop
  return.
- **Verdict: reject.** The architecture is already at the right granularity.

### sccache
- Cannot cache incrementally-compiled crates, so it's a no-op for the warm loop. It
  helps cold/CI builds of the `-O3` dep graph, but only with `CARGO_INCREMENTAL=0`,
  which would hurt local iteration.
- **Verdict: CI-only at most; not wired into local dev.**

### mold / wild linkers, `bevy_dylib`-style dynamic linking
- mold/wild are Linux-only (N/A on Windows; LLD is our linker). Iced ships no dylib
  feature, so the Bevy dynamic-linking trick is unavailable.

---

## How to get the fast loop (cheat sheet)

- **Pick a feature lane and stay in it** while editing `calc_core`:
  - Engine/GUI work → `cargo build -p calc_gui`, `cargo c`*, `cargo t` (pdf ON).
  - CLI work → `cargo cli`, `cargo check -p calc_cli`, `cargo build -p calc_core` (pdf OFF).
- **Use `bacon`** (`bacon check-core` / `check-cli` / `gui`) — each job is a single
  fixed lane, so it never thrashes.
- **Tightest inner loop is `cargo check -p <crate>` (lib only)** (~1 s), not
  `cargo c` (`--all-targets` compiles all the test code).
- **Need a debugger?** `cargo build --profile debugging -p calc_gui`.
- **Ensure `C:\Program Files\LLVM\bin` is on PATH** (lld-link + clang). Already set in
  the user PATH on this desktop.

---

## Appendix — raw results

```
COLD (clean per crate)            baseline   ->  after
  calc_core                        14.3 s        11.4 s
  calc_cli                         15.7 s        15.8 s
  calc_gui                        135.9 s       121.5 s
  workspace                       126.9 s       135.3 s  (heavy-build run noise ~±10%)

TRUE INCREMENTAL (real edit to calc_core, pdf-ON lane, after-config)
  edit -> build --workspace          3.2 s
  edit -> build -p calc_gui          2.9 s
  edit -> check -p calc_core (lib)  ~1.0 s
  no-op build/check                 0.3-0.4 s

FEATURE THRASH (no edit; pdf on<->off switch)
  --workspace -> -p calc_core       10.8 s   (first pdf-off build)
  -> --workspace                     0.9 s   (pdf-on cached)
  -> -p calc_core                    0.3 s   (pdf-off now cached)

FULL clean calc_core build (one thrash hit's worth of work)
  stable-LLVM                       12.1 s
  nightly-LLVM                      12.2 s
  nightly -Zthreads=8               10.2 s   (-16% vs nightly-LLVM)
  nightly cranelift                  8.7 s   (-29% vs nightly-LLVM)
```

### Sources (research, 2025-2026)
- Cargo, *Optimizing Build Performance* — https://doc.rust-lang.org/cargo/guide/build-performance.html
- Cargo *Profiles* — https://doc.rust-lang.org/cargo/reference/profiles.html
- rust-lld on 1.90 (Linux-only stabilization) — https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable
- rustc_codegen_cranelift — https://github.com/rust-lang/rustc_codegen_cranelift and June 2025 report https://bjorn3.github.io/2025/06/30/progress-report-june-2025.html
- Parallel front-end — https://blog.rust-lang.org/2023/11/09/parallel-rustc/
- Kobzol, *Disable debuginfo to improve compile times* — https://kobzol.github.io/rust/rustc/2025/05/20/disable-debuginfo-to-improve-rust-compile-times.html
- matklad, *Fast Rust Builds* — https://matklad.github.io/2021/09/04/fast-rust-builds.html
- Bevy `config_fast_builds.toml` — https://github.com/bevyengine/bevy/blob/main/.cargo/config_fast_builds.toml
- sccache Rust docs — https://github.com/mozilla/sccache/blob/main/docs/Rust.md
