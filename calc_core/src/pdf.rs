//! # PDF Generation Module
//!
//! Generates professional PDF reports from structural calculations using Typst.
//!
//! ## Architecture
//!
//! - Typst templates are embedded as string constants
//! - Data is injected via string formatting before compilation
//! - Output is raw PDF bytes (`Vec<u8>`)
//!
//! ## Example
//!
//! ```rust,no_run
//! use calc_core::pdf::render_beam_pdf;
//! use calc_core::calculations::continuous_beam::{ContinuousBeamInput, calculate_continuous};
//! use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
//! use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType, DesignMethod};
//! use calc_core::nds_factors::AdjustmentFactors;
//!
//! let load_case = EnhancedLoadCase::new("Floor")
//!     .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
//!     .with_load(DiscreteLoad::uniform(LoadType::Live, 100.0));
//!
//! let input = ContinuousBeamInput::simple_span(
//!     "B-1",
//!     12.0,
//!     1.5,
//!     9.25,
//!     Material::SawnLumber(WoodMaterial::new(WoodSpecies::DouglasFirLarch, WoodGrade::No2)),
//!     load_case,
//! );
//!
//! let result = calculate_continuous(&input, DesignMethod::Asd).unwrap();
//! let pdf_bytes = render_beam_pdf(&input, &result, "John Engineer", "25-001").unwrap();
//! std::fs::write("beam_report.pdf", pdf_bytes).unwrap();
//! ```

use chrono::Utc;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_pdf::PdfOptions;

use crate::calculations::continuous_beam::{
    calculate_continuous, ContinuousBeamInput, ContinuousBeamResult,
};
use crate::calculations::CalculationItem;
use crate::equations::registry::{beam_calculation_equations, EquationTracker};
use crate::errors::{CalcError, CalcResult};
use crate::nds_factors::nds_ref;
use crate::project::Project;

// ============================================================================
// Typst World Implementation
// ============================================================================

/// A minimal Typst world for compiling documents without external files.
struct PdfWorld {
    /// The main source document
    main: Source,
    /// Font book
    book: LazyHash<FontBook>,
    /// Available fonts
    fonts: Vec<Font>,
    /// Library (standard functions)
    library: LazyHash<Library>,
}

impl PdfWorld {
    fn new(source: String) -> Self {
        let fonts = Self::load_fonts();
        let book = FontBook::from_fonts(&fonts);

        PdfWorld {
            main: Source::detached(source),
            book: LazyHash::new(book),
            fonts,
            library: LazyHash::new(Library::default()),
        }
    }

    fn load_fonts() -> Vec<Font> {
        let mut fonts = Vec::new();

        // Load BerkeleyMono fonts (embedded at compile time)
        const BERKELEY_MONO_REGULAR: &[u8] =
            include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Regular.otf");
        const BERKELEY_MONO_BOLD: &[u8] =
            include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Bold.otf");
        const BERKELEY_MONO_OBLIQUE: &[u8] =
            include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Oblique.otf");
        const BERKELEY_MONO_BOLD_OBLIQUE: &[u8] =
            include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Bold-Oblique.otf");

        // Load our custom fonts first (higher priority)
        for font_bytes in [
            BERKELEY_MONO_REGULAR,
            BERKELEY_MONO_BOLD,
            BERKELEY_MONO_OBLIQUE,
            BERKELEY_MONO_BOLD_OBLIQUE,
        ] {
            let buffer = Bytes::new(font_bytes.to_vec());
            for font in Font::iter(buffer) {
                fonts.push(font);
            }
        }

        // Load bundled fonts from typst-assets (fallback for math symbols, etc.)
        for font_bytes in typst_assets::fonts() {
            let buffer = Bytes::new(font_bytes.to_vec());
            for font in Font::iter(buffer) {
                fonts.push(font);
            }
        }

        fonts
    }
}

impl World for PdfWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        let now = Utc::now();
        Datetime::from_ymd(
            now.format("%Y").to_string().parse().ok()?,
            now.format("%m").to_string().parse().ok()?,
            now.format("%d").to_string().parse().ok()?,
        )
    }
}

// ============================================================================
// PDF Templates
// ============================================================================

/// Typst template for beam calculation report
const BEAM_TEMPLATE: &str = r##"
#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
  header: align(right)[
    #text(size: 9pt, fill: gray)[Stratify Structural Calculations]
  ],
  footer: context [
    #line(length: 100%, stroke: 0.5pt + gray)
    #v(4pt)
    #grid(
      columns: (1fr, 1fr, 1fr),
      align(left)[#text(size: 9pt)[Job: {{JOB_ID}}]],
      align(center)[#text(size: 9pt)[Page #counter(page).display()]],
      align(right)[#text(size: 9pt)[{{DATE}}]],
    )
  ]
)

#set text(font: "Berkeley Mono", size: 11pt)

// Title Block
#align(center)[
  #block(width: 100%, fill: rgb("#f0f0f0"), inset: 12pt, radius: 4pt)[
    #text(size: 18pt, weight: "bold")[Simply-Supported Beam Analysis]
    #v(4pt)
    #text(size: 14pt)[{{BEAM_LABEL}}]
  ]
]

#v(12pt)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    *Project Information*
    #v(4pt)
    #table(
      columns: (auto, 1fr),
      stroke: none,
      row-gutter: 4pt,
      [Engineer:], [{{ENGINEER}}],
      [Job ID:], [{{JOB_ID}}],
      [Date:], [{{DATE}}],
    )
  ],
  [
    *Code Reference*
    #v(4pt)
    NDS 2018 (National Design Specification for Wood Construction)
  ]
)

#v(16pt)
#line(length: 100%, stroke: 0.5pt)
#v(8pt)

== Input Parameters

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Parameter*], [*Value*], [*Unit*]),
  [Clear Span], [{{SPAN_FT}}], [ft],
  [Uniform Load], [{{LOAD_PLF}}], [plf],
  [Beam Width], [{{WIDTH_IN}}], [in],
  [Beam Depth], [{{DEPTH_IN}}], [in],
  [Material], [{{MATERIAL}}], [],
)

#v(12pt)

== Section Properties

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Property*], [*Value*], [*Unit*]),
  [Section Modulus (S)], [{{SECTION_MODULUS}}], [in#super[3]],
  [Moment of Inertia (I)], [{{MOMENT_INERTIA}}], [in#super[4]],
)

#v(12pt)

== Material Properties (Reference Values)

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Property*], [*Value*], [*Unit*]),
  [Bending Stress (F#sub[b])], [{{FB_REF}}], [psi],
  [Shear Stress (F#sub[v])], [{{FV_REF}}], [psi],
  [Modulus of Elasticity (E)], [{{E_REF}}], [psi],
)

#v(16pt)
#line(length: 100%, stroke: 0.5pt)
#v(8pt)

== Analysis Results

=== Applied Forces (Demand)

For a simply-supported beam with uniform load:

$ M_"max" = (w L^2) / 8 = {{MOMENT_FTLB}} "ft-lb" $

$ V_"max" = (w L) / 2 = {{SHEAR_LB}} "lb" $

$ delta_"max" = (5 w L^4) / (384 E I) = {{DEFLECTION_IN}} "in" $

#v(12pt)

=== Stress Checks

#table(
  columns: (1fr, auto, auto, auto, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, right, right, center, left),
  table.header([*Check*], [*Actual*], [*Allowable*], [*Unity*], [*Status*], [*NDS Ref*]),
  [Bending], [{{FB_ACTUAL}} psi], [{{FB_ALLOW}} psi], [{{BENDING_UNITY}}], [{{BENDING_STATUS}}], [{{NDS_BENDING}}],
  [Shear], [{{FV_ACTUAL}} psi], [{{FV_ALLOW}} psi], [{{SHEAR_UNITY}}], [{{SHEAR_STATUS}}], [{{NDS_SHEAR}}],
  [Deflection], [L/{{DEFL_RATIO}}], [L/{{DEFL_LIMIT}}], [{{DEFL_UNITY}}], [{{DEFL_STATUS}}], [{{NDS_DEFLECTION}}],
)

#v(16pt)

#let pass_status = "{{OVERALL_PASS}}"
#align(center)[
  #block(
    width: auto,
    fill: if pass_status == "PASS" { rgb("#d4edda") } else { rgb("#f8d7da") },
    inset: 16pt,
    radius: 4pt
  )[
    #text(size: 16pt, weight: "bold")[
      #if pass_status == "PASS" [
        DESIGN ADEQUATE
      ] else [
        DESIGN INADEQUATE
      ]
    ]
    #v(4pt)
    #text(size: 12pt)[Governing condition: {{GOVERNING}}]
  ]
]

#v(24pt)
#line(length: 100%, stroke: 0.5pt)
#v(8pt)

#text(size: 9pt, fill: gray)[
  Generated by Stratify Structural Engineering Suite \
  Calculations should be verified by a licensed professional engineer.
]
"##;

// ============================================================================
// PDF Rendering Functions
// ============================================================================

/// Render a beam calculation to PDF.
///
/// # Arguments
///
/// * `input` - The beam input parameters
/// * `result` - The calculation results
/// * `engineer` - Engineer name for the report
/// * `job_id` - Job/project ID
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - PDF file as bytes
/// * `Err(CalcError)` - If rendering fails
///
/// # Example
///
/// ```rust,no_run
/// use calc_core::pdf::render_beam_pdf;
/// use calc_core::calculations::continuous_beam::{ContinuousBeamInput, calculate_continuous};
/// use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
/// use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType, DesignMethod};
/// use calc_core::nds_factors::AdjustmentFactors;
///
/// let load_case = EnhancedLoadCase::new("Floor")
///     .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
///     .with_load(DiscreteLoad::uniform(LoadType::Live, 100.0));
///
/// let input = ContinuousBeamInput::simple_span(
///     "B-1",
///     12.0,
///     1.5,
///     9.25,
///     Material::SawnLumber(WoodMaterial::new(WoodSpecies::DouglasFirLarch, WoodGrade::No2)),
///     load_case,
/// );
///
/// let result = calculate_continuous(&input, DesignMethod::Asd).unwrap();
/// let pdf = render_beam_pdf(&input, &result, "John Engineer", "25-001").unwrap();
/// ```
pub fn render_beam_pdf(
    input: &ContinuousBeamInput,
    result: &ContinuousBeamResult,
    engineer: &str,
    job_id: &str,
) -> CalcResult<Vec<u8>> {
    // Use the first span's properties (for single-span beams) or primary span
    let first_span = input.spans.first().ok_or_else(|| {
        CalcError::invalid_input("spans", "empty", "At least one span is required")
    })?;

    // Get span result for detailed data
    let span_result = result
        .span_results
        .first()
        .ok_or_else(|| CalcError::Internal {
            message: "No span results available".to_string(),
        })?;

    // Calculate total design load (from load case)
    let design_load_plf = input.load_case.total_uniform_plf();

    // Calculate deflection ratio (L/delta)
    let l_in = input.total_length_ft() * 12.0;
    let deflection_ratio = if result.max_deflection_in > 0.0 {
        l_in / result.max_deflection_in
    } else {
        9999.0
    };
    let deflection_limit_ratio = 240.0; // L/240 for floor beams

    let first_span_props = first_span.material.base_properties()?;
    let first_span_e_psi = first_span.e_psi()?;

    // Format the template with calculation data
    let source = BEAM_TEMPLATE
        .replace("{{BEAM_LABEL}}", &input.label)
        .replace("{{ENGINEER}}", engineer)
        .replace("{{JOB_ID}}", job_id)
        .replace("{{DATE}}", &Utc::now().format("%Y-%m-%d").to_string())
        .replace("{{SPAN_FT}}", &format!("{:.1}", input.total_length_ft()))
        .replace("{{LOAD_PLF}}", &format!("{:.0}", design_load_plf))
        .replace("{{WIDTH_IN}}", &format!("{:.2}", first_span.width_in))
        .replace("{{DEPTH_IN}}", &format!("{:.2}", first_span.depth_in))
        .replace("{{MATERIAL}}", &first_span.material.display_name())
        .replace(
            "{{SECTION_MODULUS}}",
            &format!("{:.2}", first_span.section_modulus_in3()),
        )
        .replace(
            "{{MOMENT_INERTIA}}",
            &format!("{:.2}", first_span.moment_of_inertia_in4()),
        )
        .replace("{{FB_REF}}", &format!("{:.0}", first_span_props.fb_psi))
        .replace("{{FV_REF}}", &format!("{:.0}", first_span_props.fv_psi))
        .replace("{{E_REF}}", &format!("{:.0}", first_span_e_psi))
        .replace(
            "{{MOMENT_FTLB}}",
            &format!("{:.0}", result.max_positive_moment_ftlb),
        )
        .replace("{{SHEAR_LB}}", &format!("{:.0}", result.max_shear_lb))
        .replace(
            "{{DEFLECTION_IN}}",
            &format!("{:.3}", result.max_deflection_in),
        )
        .replace(
            "{{FB_ACTUAL}}",
            &format!("{:.0}", span_result.actual_fb_psi),
        )
        .replace(
            "{{FB_ALLOW}}",
            &format!("{:.0}", span_result.allowable_fb_psi),
        )
        .replace(
            "{{BENDING_UNITY}}",
            &format!("{:.2}", span_result.bending_unity),
        )
        .replace(
            "{{BENDING_STATUS}}",
            if span_result.bending_unity <= 1.0 {
                "OK"
            } else {
                "FAIL"
            },
        )
        .replace(
            "{{FV_ACTUAL}}",
            &format!("{:.0}", span_result.actual_fv_psi),
        )
        .replace(
            "{{FV_ALLOW}}",
            &format!("{:.0}", span_result.allowable_fv_psi),
        )
        .replace(
            "{{SHEAR_UNITY}}",
            &format!("{:.2}", span_result.shear_unity),
        )
        .replace(
            "{{SHEAR_STATUS}}",
            if span_result.shear_unity <= 1.0 {
                "OK"
            } else {
                "FAIL"
            },
        )
        .replace("{{DEFL_RATIO}}", &format!("{:.0}", deflection_ratio))
        .replace("{{DEFL_LIMIT}}", &format!("{:.0}", deflection_limit_ratio))
        .replace(
            "{{DEFL_UNITY}}",
            &format!("{:.2}", span_result.deflection_unity),
        )
        .replace(
            "{{DEFL_STATUS}}",
            if span_result.deflection_unity <= 1.0 {
                "OK"
            } else {
                "FAIL"
            },
        )
        .replace("{{NDS_BENDING}}", nds_ref::BENDING)
        .replace("{{NDS_SHEAR}}", nds_ref::SHEAR)
        .replace("{{NDS_DEFLECTION}}", nds_ref::DEFLECTION)
        .replace(
            "{{OVERALL_PASS}}",
            if result.passes() { "PASS" } else { "FAIL" },
        )
        .replace("{{GOVERNING}}", &result.governing_condition);

    // Compile the Typst document
    let world = PdfWorld::new(source);

    let warned = typst::compile(&world);

    let document = warned.output.map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        CalcError::Internal {
            message: format!("Typst compilation failed: {}", error_msgs.join("; ")),
        }
    })?;

    // Render to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        CalcError::Internal {
            message: format!("PDF rendering failed: {}", error_msgs.join("; ")),
        }
    })?;

    Ok(pdf_bytes)
}

/// Render an entire project (all beams) to a single PDF.
///
/// # Arguments
///
/// * `project` - The project containing all calculation items
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - PDF file as bytes
/// * `Err(CalcError)` - If rendering fails or project has no beams
///
/// # Example
///
/// ```rust,no_run
/// use calc_core::pdf::render_project_pdf;
/// use calc_core::project::Project;
///
/// let project = Project::new("John Engineer", "25-001", "ACME Corp");
/// let pdf = render_project_pdf(&project).unwrap();
/// ```
// Nested `format!` calls inside the outer per-beam template are deliberate:
// they format each numeric value to its physical-engineering precision (psi,
// in, ft-lb) at the call site, alongside the named arg, keeping the template
// readable as a Typst block. Inlining precision into the template would split
// each value's identity across two places.
#[allow(clippy::format_in_format_args)]
pub fn render_project_pdf(project: &Project) -> CalcResult<Vec<u8>> {
    // Collect all beams and calculate their results
    let mut beams: Vec<(&ContinuousBeamInput, ContinuousBeamResult)> = Vec::new();
    let design_method = project.settings.design_method;

    for item in project.items.values() {
        if let CalculationItem::Beam(beam) = item {
            match calculate_continuous(beam, design_method) {
                Ok(result) => beams.push((beam, result)),
                Err(e) => {
                    return Err(CalcError::Internal {
                        message: format!("Failed to calculate beam '{}': {}", beam.label, e),
                    });
                }
            }
        }
    }

    if beams.is_empty() {
        return Err(CalcError::Internal {
            message: "Project has no beams to export".to_string(),
        });
    }

    // Sort beams by label for consistent ordering
    beams.sort_by(|a, b| a.0.label.cmp(&b.0.label));

    // Build multi-beam Typst source
    let mut source = format!(
        r##"
#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
  header: align(right)[
    #text(size: 9pt, fill: gray)[Stratify Structural Calculations]
  ],
  footer: context [
    #line(length: 100%, stroke: 0.5pt + gray)
    #v(4pt)
    #grid(
      columns: (1fr, 1fr, 1fr),
      align(left)[#text(size: 9pt)[Job: {job_id}]],
      align(center)[#text(size: 9pt)[Page #counter(page).display()]],
      align(right)[#text(size: 9pt)[{date}]],
    )
  ]
)

#set text(font: "Berkeley Mono", size: 11pt)

// Cover Page / Table of Contents
#align(center)[
  #block(width: 100%, fill: rgb("#f0f0f0"), inset: 20pt, radius: 4pt)[
    #text(size: 24pt, weight: "bold")[Structural Calculation Package]
    #v(8pt)
    #text(size: 16pt)[{client}]
  ]
]

#v(24pt)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    *Project Information*
    #v(4pt)
    #table(
      columns: (auto, 1fr),
      stroke: none,
      row-gutter: 4pt,
      [Engineer:], [{engineer}],
      [Job ID:], [{job_id}],
      [Client:], [{client}],
      [Date:], [{date}],
    )
  ],
  [
    *Code Reference*
    #v(4pt)
    NDS 2018 (National Design Specification for Wood Construction)
  ]
)

#v(24pt)

== Calculation Summary

#table(
  columns: (auto, 1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, left, right, center),
  table.header([*No.*], [*Item*], [*Governing Unity*], [*Status*]),
{summary_rows}
)

#v(24pt)
#text(size: 9pt, fill: gray)[
  Generated by Stratify Structural Engineering Suite \
  Calculations should be verified by a licensed professional engineer.
]
"##,
        job_id = escape_typst(&project.meta.job_id),
        date = Utc::now().format("%Y-%m-%d"),
        client = escape_typst(&project.meta.client),
        engineer = escape_typst(&project.meta.engineer),
        summary_rows = build_summary_rows(&beams),
    );

    // Add individual beam pages
    for (i, (input, result)) in beams.iter().enumerate() {
        // Get first span's properties
        let first_span = match input.spans.first() {
            Some(s) => s,
            None => continue,
        };

        // Get first span result
        let span_result = match result.span_results.first() {
            Some(r) => r,
            None => continue,
        };

        let design_load_plf = input.load_case.total_uniform_plf();
        let l_in = input.total_length_ft() * 12.0;
        let deflection_ratio = if result.max_deflection_in > 0.0 {
            l_in / result.max_deflection_in
        } else {
            9999.0
        };

        let first_span_props = first_span.material.base_properties()?;
        let first_span_e_psi = first_span.e_psi()?;

        source.push_str(&format!(
            r##"
#pagebreak()

// Beam {} of {}
#align(center)[
  #block(width: 100%, fill: rgb("#f0f0f0"), inset: 12pt, radius: 4pt)[
    #text(size: 18pt, weight: "bold")[Beam Analysis]
    #v(4pt)
    #text(size: 14pt)[{beam_label}]
  ]
]

#v(12pt)

== Input Parameters

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Parameter*], [*Value*], [*Unit*]),
  [Total Span], [{span_ft}], [ft],
  [Uniform Load], [{load_plf}], [plf],
  [Beam Width], [{width_in}], [in],
  [Beam Depth], [{depth_in}], [in],
  [Material], [{material}], [],
)

#v(12pt)

== Section Properties

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Property*], [*Value*], [*Unit*]),
  [Section Modulus (S)], [{section_modulus}], [in#super[3]],
  [Moment of Inertia (I)], [{moment_inertia}], [in#super[4]],
)

#v(12pt)

== Material Properties (Reference Values)

#table(
  columns: (1fr, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, left),
  table.header([*Property*], [*Value*], [*Unit*]),
  [Bending Stress (F#sub[b])], [{fb_ref}], [psi],
  [Shear Stress (F#sub[v])], [{fv_ref}], [psi],
  [Modulus of Elasticity (E)], [{e_ref}], [psi],
)

#v(12pt)

== Analysis Results

=== Applied Forces (Demand)

$ M_"max" = {moment_ftlb} "ft-lb" $

$ V_"max" = {shear_lb} "lb" $

$ delta_"max" = {deflection_in} "in" $

#v(12pt)

=== Stress Checks

#table(
  columns: (1fr, auto, auto, auto, auto, auto),
  inset: 8pt,
  stroke: 0.5pt,
  align: (left, right, right, right, center, left),
  table.header([*Check*], [*Actual*], [*Allowable*], [*Unity*], [*Status*], [*NDS Ref*]),
  [Bending], [{fb_actual} psi], [{fb_allow} psi], [{bending_unity}], [{bending_status}], [{nds_bending}],
  [Shear], [{fv_actual} psi], [{fv_allow} psi], [{shear_unity}], [{shear_status}], [{nds_shear}],
  [Deflection], [L/{defl_ratio}], [L/{defl_limit}], [{defl_unity}], [{defl_status}], [{nds_deflection}],
)

#v(16pt)

#align(center)[
  #block(
    width: auto,
    fill: {status_color},
    inset: 16pt,
    radius: 4pt
  )[
    #text(size: 16pt, weight: "bold")[{status_text}]
    #v(4pt)
    #text(size: 12pt)[Governing condition: {governing}]
  ]
]
"##,
            i + 1,
            beams.len(),
            beam_label = escape_typst(&input.label),
            span_ft = format!("{:.1}", input.total_length_ft()),
            load_plf = format!("{:.0}", design_load_plf),
            width_in = format!("{:.2}", first_span.width_in),
            depth_in = format!("{:.2}", first_span.depth_in),
            material = first_span.material.display_name(),
            section_modulus = format!("{:.2}", first_span.section_modulus_in3()),
            moment_inertia = format!("{:.2}", first_span.moment_of_inertia_in4()),
            fb_ref = format!("{:.0}", first_span_props.fb_psi),
            fv_ref = format!("{:.0}", first_span_props.fv_psi),
            e_ref = format!("{:.0}", first_span_e_psi),
            moment_ftlb = format!("{:.0}", result.max_positive_moment_ftlb),
            shear_lb = format!("{:.0}", result.max_shear_lb),
            deflection_in = format!("{:.3}", result.max_deflection_in),
            fb_actual = format!("{:.0}", span_result.actual_fb_psi),
            fb_allow = format!("{:.0}", span_result.allowable_fb_psi),
            bending_unity = format!("{:.2}", span_result.bending_unity),
            bending_status = if span_result.bending_unity <= 1.0 { "OK" } else { "FAIL" },
            fv_actual = format!("{:.0}", span_result.actual_fv_psi),
            fv_allow = format!("{:.0}", span_result.allowable_fv_psi),
            shear_unity = format!("{:.2}", span_result.shear_unity),
            shear_status = if span_result.shear_unity <= 1.0 { "OK" } else { "FAIL" },
            defl_ratio = format!("{:.0}", deflection_ratio),
            defl_limit = format!("{:.0}", 240.0),
            defl_unity = format!("{:.2}", span_result.deflection_unity),
            defl_status = if span_result.deflection_unity <= 1.0 { "OK" } else { "FAIL" },
            nds_bending = nds_ref::BENDING,
            nds_shear = nds_ref::SHEAR,
            nds_deflection = nds_ref::DEFLECTION,
            status_color = if result.passes() {
                "rgb(\"#d4edda\")"
            } else {
                "rgb(\"#f8d7da\")"
            },
            status_text = if result.passes() {
                "DESIGN ADEQUATE"
            } else {
                "DESIGN INADEQUATE"
            },
            governing = &result.governing_condition,
        ));
    }

    // Build equation tracker for the appendix
    let mut equation_tracker = EquationTracker::new();
    for (input, _result) in &beams {
        // Record the standard beam calculation equations for each beam
        for equation in beam_calculation_equations() {
            equation_tracker.record_for_member(equation, "Beam analysis", input.label.clone());
        }
    }

    // Add the equations appendix
    source.push_str(&equation_tracker.generate_appendix_typst());

    // Compile the Typst document
    let world = PdfWorld::new(source);
    let warned = typst::compile(&world);

    let document = warned.output.map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        CalcError::Internal {
            message: format!("Typst compilation failed: {}", error_msgs.join("; ")),
        }
    })?;

    // Render to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        CalcError::Internal {
            message: format!("PDF rendering failed: {}", error_msgs.join("; ")),
        }
    })?;

    Ok(pdf_bytes)
}

/// Escape special Typst characters in user-provided text
fn escape_typst(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '*' => "\\*".to_string(),
            '_' => "\\_".to_string(),
            '#' => "\\#".to_string(),
            '$' => "\\$".to_string(),
            '@' => "\\@".to_string(),
            '<' => "\\<".to_string(),
            '>' => "\\>".to_string(),
            '\\' => "\\\\".to_string(),
            '`' => "\\`".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Build summary table rows for the cover page
fn build_summary_rows(beams: &[(&ContinuousBeamInput, ContinuousBeamResult)]) -> String {
    beams
        .iter()
        .enumerate()
        .map(|(i, (input, result))| {
            let max_unity = result.governing_unity;
            let status = if result.passes() { "OK" } else { "FAIL" };
            format!(
                "  [{}], [Beam: {}], [{:.2}], [{}],",
                i + 1,
                escape_typst(&input.label),
                max_unity,
                status
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loads::{DesignMethod, DiscreteLoad, EnhancedLoadCase, LoadType};
    use crate::materials::{Material, WoodGrade, WoodMaterial, WoodSpecies};

    #[test]
    fn test_pdf_generation() {
        let load_case = EnhancedLoadCase::new("Test Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 30.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 70.0));

        let input = ContinuousBeamInput::simple_span(
            "B-1 Test Beam",
            12.0,
            1.5,
            9.25,
            Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            load_case,
        );

        let result = calculate_continuous(&input, DesignMethod::Asd).unwrap();
        let pdf = render_beam_pdf(&input, &result, "Test Engineer", "TEST-001");

        // Should succeed
        assert!(pdf.is_ok(), "PDF generation failed: {:?}", pdf.err());

        let pdf_bytes = pdf.unwrap();
        // PDF should start with %PDF
        assert!(pdf_bytes.starts_with(b"%PDF"), "Output is not a valid PDF");
        // Should be a reasonable size (at least 1KB)
        assert!(pdf_bytes.len() > 1000, "PDF seems too small");
    }
}
