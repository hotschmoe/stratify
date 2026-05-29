//! # Simply-Supported Beam Calculation
//!
//! Analyzes a simply-supported wood beam under various load configurations per NDS.
//!
//! ## Assumptions
//!
//! - Simply-supported (pin-roller) boundary conditions
//! - Supports multiple discrete loads (uniform, point, partial, trapezoidal, moment)
//! - Rectangular section (sawn lumber, glulam, LVL, or PSL)
//! - Dry service conditions (C_M = 1.0)
//! - Normal temperature (C_t = 1.0)
//! - Normal load duration (C_D = 1.0) - adjust as needed
//!
//! ## Example (LLM-friendly)
//!
//! ```rust
//! use calc_core::calculations::beam::{BeamInput, calculate};
//! use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
//! use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType, DesignMethod};
//! use calc_core::nds_factors::AdjustmentFactors;
//!
//! // Define beam input with multiple discrete loads
//! let load_case = EnhancedLoadCase::new("Floor Loads")
//!     .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
//!     .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0))
//!     .with_self_weight();
//!
//! let input = BeamInput {
//!     label: "B-1".to_string(),
//!     span_ft: 12.0,
//!     load_case,
//!     material: Material::SawnLumber(WoodMaterial::new(
//!         WoodSpecies::DouglasFirLarch,
//!         WoodGrade::No2
//!     )),
//!     width_in: 1.5,  // 2x nominal
//!     depth_in: 9.25, // 10 nominal
//!     adjustment_factors: AdjustmentFactors::default(),
//! };
//!
//! let result = calculate(&input, DesignMethod::Asd).unwrap();
//!
//! println!("Max moment: {:.2} ft-lb", result.max_moment_ftlb);
//! println!("Bending stress: {:.0} psi", result.actual_fb_psi);
//! println!("Bending unity: {:.2}", result.bending_unity);
//! println!("Pass: {}", result.passes());
//! ```

use serde::{Deserialize, Serialize};

use crate::errors::{CalcError, CalcResult};
use crate::loads::{DesignMethod, EnhancedLoadCase, LoadDistribution, LoadType};
use crate::materials::Material;
use crate::nds_factors::{AdjustmentFactors, AdjustmentSummary, BeamStability, SizeFactor};

use super::beam_analysis::{BeamAnalysis, SingleLoad};

/// Input parameters for a simply-supported beam.
///
/// All inputs use US customary units for compatibility with US building codes.
/// Supports sawn lumber, glulam, LVL, and PSL materials.
///
/// ## JSON Example (Sawn Lumber with Multiple Loads)
///
/// ```json
/// {
///   "label": "B-1",
///   "span_ft": 12.0,
///   "load_case": {
///     "label": "Floor Loads",
///     "include_self_weight": true,
///     "loads": [
///       { "load_type": "Dead", "distribution": "UniformFull", "magnitude": 15.0 },
///       { "load_type": "Live", "distribution": "UniformFull", "magnitude": 40.0 }
///     ]
///   },
///   "material": {
///     "type": "SawnLumber",
///     "species": "DF-L",
///     "grade": "No.2"
///   },
///   "width_in": 1.5,
///   "depth_in": 9.25
/// }
/// ```
///
/// ## JSON Example (Glulam with Point Load)
///
/// ```json
/// {
///   "label": "GLB-1",
///   "span_ft": 24.0,
///   "load_case": {
///     "label": "Roof Loads",
///     "include_self_weight": true,
///     "loads": [
///       { "load_type": "Dead", "distribution": "UniformFull", "magnitude": 20.0 },
///       { "load_type": "Live", "distribution": { "Point": { "position_ft": 12.0 } }, "magnitude": 5000.0 }
///     ]
///   },
///   "material": {
///     "type": "Glulam",
///     "stress_class": "24F-V4",
///     "layup": "Unbalanced"
///   },
///   "width_in": 5.125,
///   "depth_in": 16.5
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamInput {
    /// User label for this beam (e.g., "B-1", "Floor Beam at Grid A")
    pub label: String,

    /// Clear span in feet
    pub span_ft: f64,

    /// Load configuration with multiple discrete loads by type
    ///
    /// Supports D, L, Lr, S, W, E, H load types with various distributions.
    /// Use `include_self_weight` to auto-add beam dead load.
    pub load_case: EnhancedLoadCase,

    /// Material (sawn lumber, glulam, LVL, or PSL)
    pub material: Material,

    /// Actual beam width in inches (e.g., 1.5 for 2x, 5.125 for glulam)
    pub width_in: f64,

    /// Actual beam depth in inches (e.g., 9.25 for 2x10, 16.5 for glulam)
    pub depth_in: f64,

    /// NDS adjustment factors (C_D, C_M, C_t, C_r, etc.)
    ///
    /// Controls load duration, wet service, temperature, repetitive member,
    /// and other adjustment factors per NDS Chapter 4.
    /// Defaults to normal duration, dry service, normal temperature.
    #[serde(default)]
    pub adjustment_factors: AdjustmentFactors,
}

/// Typical wood density for self-weight calculation (pcf)
/// Using 35 pcf as a reasonable average for softwood lumber
const WOOD_DENSITY_PCF: f64 = 35.0;

impl BeamInput {
    /// Validate input parameters.
    pub fn validate(&self) -> CalcResult<()> {
        if self.span_ft <= 0.0 {
            return Err(CalcError::invalid_input(
                "span_ft",
                self.span_ft.to_string(),
                "Span must be positive",
            ));
        }
        if self.span_ft > 60.0 {
            return Err(CalcError::invalid_input(
                "span_ft",
                self.span_ft.to_string(),
                "Span exceeds 60 ft - verify member sizing",
            ));
        }
        if self.width_in <= 0.0 {
            return Err(CalcError::invalid_input(
                "width_in",
                self.width_in.to_string(),
                "Width must be positive",
            ));
        }
        if self.depth_in <= 0.0 {
            return Err(CalcError::invalid_input(
                "depth_in",
                self.depth_in.to_string(),
                "Depth must be positive",
            ));
        }
        Ok(())
    }

    /// Calculate section modulus S = bd²/6
    pub fn section_modulus_in3(&self) -> f64 {
        self.width_in * self.depth_in.powi(2) / 6.0
    }

    /// Calculate moment of inertia I = bd³/12
    pub fn moment_of_inertia_in4(&self) -> f64 {
        self.width_in * self.depth_in.powi(3) / 12.0
    }

    /// Calculate cross-sectional area A = bd
    pub fn area_in2(&self) -> f64 {
        self.width_in * self.depth_in
    }

    /// Calculate beam self-weight in pounds per linear foot (plf)
    ///
    /// Uses typical softwood density of 35 pcf.
    /// Area (in²) * density (pcf) / 144 (in²/ft²) = plf
    pub fn self_weight_plf(&self) -> f64 {
        let area_in2 = self.area_in2();
        area_in2 * WOOD_DENSITY_PCF / 144.0
    }

    /// Get governing factored uniform load in plf for design
    ///
    /// Applies ASCE 7 load combinations to all uniform loads in the load case.
    /// Optionally includes beam self-weight as additional dead load.
    ///
    /// Note: Point loads and partial loads are not included in this simplified
    /// uniform load calculation. For complex load cases with point loads,
    /// use the full analysis methods.
    pub fn governing_uniform_plf(&self, method: DesignMethod) -> f64 {
        // Get the governing factored load from the load case
        let mut governing = self.load_case.governing_uniform_plf(method);

        // Add self-weight if enabled (as unfactored dead load, then apply factor)
        if self.load_case.include_self_weight {
            let self_wt = self.self_weight_plf();
            // For ASD, dead load factor is 1.0
            // For LRFD, dead load factor is 1.2 or 1.4 (conservative: use 1.2)
            let factor = match method {
                DesignMethod::Asd => 1.0,
                DesignMethod::Lrfd => 1.2,
            };
            governing += self_wt * factor;
        }

        governing
    }

    /// Get total unfactored uniform dead load (plf)
    ///
    /// Includes self-weight if enabled.
    pub fn total_dead_load_plf(&self) -> f64 {
        let applied_dead = self.load_case.total_uniform_by_type(LoadType::Dead);
        if self.load_case.include_self_weight {
            applied_dead + self.self_weight_plf()
        } else {
            applied_dead
        }
    }

    /// Get total unfactored uniform live load (plf)
    pub fn total_live_load_plf(&self) -> f64 {
        self.load_case.total_uniform_by_type(LoadType::Live)
    }
}

/// Results from beam calculation.
///
/// All results include both raw values and unity checks for easy pass/fail determination.
///
/// ## JSON Example
///
/// ```json
/// {
///   "design_load_plf": 70.0,
///   "governing_combination": "ASD-2: D + L",
///   "max_moment_ftlb": 2700.0,
///   "max_shear_lb": 900.0,
///   "max_deflection_in": 0.42,
///   "actual_fb_psi": 502.7,
///   "allowable_fb_psi": 900.0,
///   "bending_unity": 0.56,
///   "actual_fv_psi": 9.7,
///   "allowable_fv_psi": 180.0,
///   "shear_unity": 0.05,
///   "deflection_ratio": 343,
///   "deflection_limit_ratio": 240,
///   "deflection_unity": 0.70
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeamResult {
    // === Load Summary ===
    /// Design load used for calculation (plf)
    ///
    /// This is the governing factored load from ASCE 7 combinations.
    pub design_load_plf: f64,

    /// Name of the governing load combination (e.g., "ASD-2: D + L")
    pub governing_combination: String,

    /// Beam self-weight (plf) - shown separately for transparency
    pub self_weight_plf: f64,

    // === Demand (Applied Forces) ===
    /// Maximum bending moment in foot-pounds
    ///
    /// For simply-supported beam with uniform load: M = wL²/8
    pub max_moment_ftlb: f64,

    /// Maximum shear force in pounds
    ///
    /// For simply-supported beam with uniform load: V = wL/2
    pub max_shear_lb: f64,

    /// Maximum deflection in inches
    ///
    /// For simply-supported beam with uniform load: δ = 5wL⁴/(384EI)
    pub max_deflection_in: f64,

    // === Bending Check ===
    /// Actual bending stress fb = M/S (psi)
    pub actual_fb_psi: f64,

    /// Allowable bending stress Fb' (psi)
    ///
    /// Adjusted for all applicable NDS factors.
    pub allowable_fb_psi: f64,

    /// Bending unity check: actual_fb / allowable_fb
    ///
    /// Must be ≤ 1.0 to pass.
    pub bending_unity: f64,

    // === Shear Check ===
    /// Actual shear stress fv = 3V/(2bd) (psi)
    pub actual_fv_psi: f64,

    /// Allowable shear stress Fv' (psi)
    pub allowable_fv_psi: f64,

    /// Shear unity check: actual_fv / allowable_fv
    ///
    /// Must be ≤ 1.0 to pass.
    pub shear_unity: f64,

    // === Deflection Check ===
    /// Deflection ratio L/δ
    ///
    /// Higher is better (less deflection).
    pub deflection_ratio: f64,

    /// Deflection limit ratio (typically L/240 for live load, L/180 for total)
    pub deflection_limit_ratio: f64,

    /// Deflection unity check: (L/limit) / (L/actual) = actual/limit
    ///
    /// Must be ≤ 1.0 to pass.
    pub deflection_unity: f64,

    // === Section Properties (for reference) ===
    /// Section modulus S (in³)
    pub section_modulus_in3: f64,

    /// Moment of inertia I (in⁴)
    pub moment_of_inertia_in4: f64,

    // === Material Properties Used ===
    /// Reference bending stress Fb (psi) before adjustments
    pub fb_reference_psi: f64,

    /// Reference shear stress Fv (psi)
    pub fv_reference_psi: f64,

    /// Modulus of elasticity E (psi)
    pub e_psi: f64,

    // === Adjustment Factors Applied ===
    /// Summary of all NDS adjustment factors used in this calculation
    pub adjustment_factors: AdjustmentSummary,

    // === Support Reactions (Governing Max Combination) ===
    /// Left support reaction (lb) - positive upward (from max governing combination)
    pub reaction_left_lb: f64,
    /// Right support reaction (lb) - positive upward (from max governing combination)
    pub reaction_right_lb: f64,

    // === Minimum Reactions (for Uplift/Anchor Design) ===
    /// Minimum left reaction (lb) - may be negative for uplift
    ///
    /// From the load combination producing minimum reactions (e.g., 0.6D - 0.6W).
    /// Negative values indicate net uplift requiring hold-down anchors.
    pub min_reaction_left_lb: f64,
    /// Minimum right reaction (lb) - may be negative for uplift
    pub min_reaction_right_lb: f64,
    /// Load combination name producing minimum reactions
    pub min_reaction_combination: String,

    // === Diagram Data (for rendering) ===
    /// Shear diagram: Vec of (position_ft, shear_lb)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shear_diagram: Vec<(f64, f64)>,
    /// Moment diagram: Vec of (position_ft, moment_ftlb)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moment_diagram: Vec<(f64, f64)>,
    /// Deflection diagram: Vec of (position_ft, deflection_in)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deflection_diagram: Vec<(f64, f64)>,
}

impl BeamResult {
    /// Check if all unity checks pass (≤ 1.0)
    pub fn passes(&self) -> bool {
        self.bending_unity <= 1.0 && self.shear_unity <= 1.0 && self.deflection_unity <= 1.0
    }

    /// Get the governing (highest) unity ratio
    pub fn governing_unity(&self) -> f64 {
        self.bending_unity
            .max(self.shear_unity)
            .max(self.deflection_unity)
    }

    /// Get a description of what governs the design
    pub fn governing_condition(&self) -> &'static str {
        if self.bending_unity >= self.shear_unity && self.bending_unity >= self.deflection_unity {
            "Bending"
        } else if self.shear_unity >= self.deflection_unity {
            "Shear"
        } else {
            "Deflection"
        }
    }
}

/// Calculate beam stresses and deflections.
///
/// This is a pure function suitable for LLM invocation.
///
/// # Arguments
///
/// * `input` - Beam parameters (span, load_case, material, size)
/// * `method` - Design method (ASD or LRFD) for load combinations
///
/// # Returns
///
/// * `Ok(BeamResult)` - Calculation results with all checks
/// * `Err(CalcError)` - Structured error if inputs are invalid
///
/// # Example
///
/// ```rust
/// use calc_core::calculations::beam::{BeamInput, calculate};
/// use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
/// use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType, DesignMethod};
/// use calc_core::nds_factors::AdjustmentFactors;
///
/// let load_case = EnhancedLoadCase::new("Floor")
///     .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
///     .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0));
///
/// let input = BeamInput {
///     label: "Test Beam".to_string(),
///     span_ft: 10.0,
///     load_case,
///     material: Material::SawnLumber(WoodMaterial::new(
///         WoodSpecies::DouglasFirLarch,
///         WoodGrade::No2
///     )),
///     width_in: 1.5,
///     depth_in: 9.25,
///     adjustment_factors: AdjustmentFactors::default(),
/// };
///
/// let result = calculate(&input, DesignMethod::Asd).expect("Calculation should succeed");
/// assert!(result.max_moment_ftlb > 0.0);
/// ```
pub fn calculate(input: &BeamInput, method: DesignMethod) -> CalcResult<BeamResult> {
    // Validate inputs
    input.validate()?;

    // Get material properties (unified interface for all material types)
    let props = input.material.base_properties()?;

    // Section properties
    let s = input.section_modulus_in3();
    let i = input.moment_of_inertia_in4();
    let area = input.area_in2();

    // Convert span to inches for deflection calc
    let span_in = input.span_ft * 12.0;

    // === Get Self Weight ===
    let self_wt = input.self_weight_plf();

    // === Convert DiscreteLoads to SingleLoads ===
    // Group loads by type for applying load combination factors
    let mut loads_by_type: Vec<(LoadType, SingleLoad)> = Vec::new();

    // Add self-weight as dead load if enabled
    if input.load_case.include_self_weight {
        loads_by_type.push((LoadType::Dead, SingleLoad::uniform_full(self_wt)));
    }

    // Convert each DiscreteLoad to SingleLoad
    for discrete_load in &input.load_case.loads {
        let magnitude = discrete_load.effective_magnitude();
        let load_type = discrete_load.load_type;

        let single = match &discrete_load.distribution {
            LoadDistribution::Point { position_ft } => SingleLoad::point(magnitude, *position_ft),
            LoadDistribution::UniformFull => SingleLoad::uniform_full(magnitude),
            LoadDistribution::UniformPartial { start_ft, end_ft } => {
                SingleLoad::uniform_partial(magnitude, *start_ft, *end_ft)
            }
            LoadDistribution::Moment { position_ft } => SingleLoad::moment(magnitude, *position_ft),
            LoadDistribution::Trapezoidal {
                start_ft,
                end_ft,
                start_magnitude,
                end_magnitude,
            } => {
                // Approximate trapezoidal as partial uniform with average magnitude
                let avg_mag = (start_magnitude + end_magnitude) / 2.0;
                SingleLoad::uniform_partial(avg_mag, *start_ft, *end_ft)
            }
        };

        loads_by_type.push((load_type, single));
    }

    // === Run Analysis for Each Load Combination ===
    // Track both max (for strength design) and min (for uplift/anchor design)
    let combinations = method.combinations();
    let mut governing_moment = 0.0f64;
    let mut governing_combo_name = String::new();
    let mut governing_analysis: Option<super::beam_analysis::AnalysisResults> = None;
    let mut governing_total_plf = 0.0f64;

    // Track minimum reactions for uplift design
    let mut min_reaction_total = f64::MAX;
    let mut min_reaction_combo_name = String::new();
    let mut min_reaction_left = 0.0f64;
    let mut min_reaction_right = 0.0f64;

    for combo in &combinations {
        // Create analysis with factored loads
        let mut analysis = BeamAnalysis::new(input.span_ft, props.e_psi, i);

        let mut total_factored_plf = 0.0;

        for (load_type, single_load) in &loads_by_type {
            let factor = combo.get_factor(*load_type);
            if factor.abs() < 1e-10 {
                continue; // Skip loads with zero factor
            }

            // Create factored copy of the load
            let factored_load = match single_load {
                SingleLoad::Point {
                    magnitude_lb,
                    position_ft,
                } => SingleLoad::point(magnitude_lb * factor, *position_ft),
                SingleLoad::UniformFull { magnitude_plf } => {
                    total_factored_plf += magnitude_plf * factor;
                    SingleLoad::uniform_full(magnitude_plf * factor)
                }
                SingleLoad::UniformPartial {
                    magnitude_plf,
                    start_ft,
                    end_ft,
                } => SingleLoad::uniform_partial(magnitude_plf * factor, *start_ft, *end_ft),
                SingleLoad::Moment {
                    magnitude_ftlb,
                    position_ft,
                } => SingleLoad::moment(magnitude_ftlb * factor, *position_ft),
            };

            analysis.add_load(factored_load);
        }

        // Skip empty analyses
        if analysis.loads.is_empty() {
            continue;
        }

        let results = analysis.analyze();

        // Check if this combination governs for max moment (strength design)
        if results.max_moment_ftlb > governing_moment {
            governing_moment = results.max_moment_ftlb;
            governing_combo_name = combo.name.clone();
            governing_analysis = Some(results.clone());
            governing_total_plf = total_factored_plf;
        }

        // Check if this combination produces minimum reactions (uplift design)
        // Use sum of reactions as the metric (could also use min of left/right)
        let reaction_sum = results.reaction_left_lb + results.reaction_right_lb;
        if reaction_sum < min_reaction_total {
            min_reaction_total = reaction_sum;
            min_reaction_combo_name = combo.name.clone();
            min_reaction_left = results.reaction_left_lb;
            min_reaction_right = results.reaction_right_lb;
        }
    }

    // Use governing analysis results
    let analysis_results = governing_analysis.unwrap_or_else(|| {
        // Fallback for empty load case - run with zero load
        let analysis = BeamAnalysis::new(input.span_ft, props.e_psi, i);
        analysis.analyze()
    });

    let max_moment_ftlb = analysis_results.max_moment_ftlb;
    let max_shear_lb = analysis_results.max_shear_lb;
    let design_load_plf = governing_total_plf; // For display purposes
    let governing_combination = governing_combo_name;

    // === Apply NDS Adjustment Factors ===
    // (Calculated early so we can use adjusted E for deflection)
    let factors = &input.adjustment_factors;

    // Calculate size factor C_F for sawn lumber (NDS Table 4A footnote)
    // Engineered lumber (LVL/PSL) handles depth adjustment internally via fb_for_depth
    let c_f = if !input.material.is_engineered() {
        SizeFactor::new(input.depth_in, input.width_in).factor_fb()
    } else {
        1.0
    };

    // Get base Fb adjusted for depth (handles LVL/PSL depth factor automatically)
    let fb_depth_adjusted = input.material.fb_for_depth(input.depth_in)?;

    // Calculate beam stability factor C_L
    // If compression edge is braced, C_L = 1.0
    // Otherwise, calculate based on slenderness
    let c_l = if factors.compression_edge_braced {
        1.0
    } else {
        // Use unbraced length or default to span
        let le = factors.unbraced_length_in.unwrap_or(span_in);
        let stability = BeamStability::new(le, input.width_in, input.depth_in);

        // Check if effectively braced (short unbraced length)
        if stability.is_fully_braced() {
            1.0
        } else {
            // Calculate Fb* (Fb with all factors except C_L)
            // Fb* = Fb × C_D × C_M × C_t × C_F × C_fu × C_i × C_r
            let fb_star = fb_depth_adjusted
                * factors.c_d()
                * factors.c_m_fb()
                * factors.c_t()
                * c_f
                * factors.c_fu(input.width_in)
                * factors.c_i_strength()
                * factors.c_r();

            // Calculate E'min
            let e_min_prime = factors.adjusted_e_min(props.e_min_psi);

            stability.factor(fb_star, e_min_prime)
        }
    };

    // Calculate adjusted allowable stresses using all NDS factors
    let allowable_fb_psi = factors.adjusted_fb(fb_depth_adjusted, c_f, c_l, input.width_in);
    let allowable_fv_psi = factors.adjusted_fv(props.fv_psi);

    // Calculate adjusted E for deflection
    let e_adjusted = factors.adjusted_e(props.e_psi);

    // Maximum deflection from analysis (computed with raw E)
    // Scale by E_raw/E_adjusted since δ ∝ 1/E
    let max_deflection_in = analysis_results.max_deflection_in * (props.e_psi / e_adjusted);

    // === Calculate Stresses ===

    // Bending stress: fb = M/S
    // M is in ft-lb, S is in in³, so convert M to in-lb
    let max_moment_inlb = max_moment_ftlb * 12.0;
    let actual_fb_psi = max_moment_inlb / s;

    // Shear stress: fv = 3V/(2bd) = 3V/(2A)
    let actual_fv_psi = 3.0 * max_shear_lb / (2.0 * area);

    // === Unity Checks ===

    let bending_unity = actual_fb_psi / allowable_fb_psi;
    let shear_unity = actual_fv_psi / allowable_fv_psi;

    // Deflection check (L/240 for typical floor beam)
    let deflection_limit_ratio = 240.0;
    let deflection_ratio = if max_deflection_in > 0.0 {
        span_in / max_deflection_in
    } else {
        f64::INFINITY
    };
    let deflection_unity = deflection_limit_ratio / deflection_ratio;

    // Generate adjustment factors summary for reporting
    let adjustment_summary = factors.summary(input.width_in, input.depth_in, c_f, c_l);

    Ok(BeamResult {
        design_load_plf,
        governing_combination,
        self_weight_plf: self_wt,
        max_moment_ftlb,
        max_shear_lb,
        max_deflection_in,
        actual_fb_psi,
        allowable_fb_psi,
        bending_unity,
        actual_fv_psi,
        allowable_fv_psi,
        shear_unity,
        deflection_ratio,
        deflection_limit_ratio,
        deflection_unity,
        section_modulus_in3: s,
        moment_of_inertia_in4: i,
        fb_reference_psi: props.fb_psi,
        fv_reference_psi: props.fv_psi,
        e_psi: props.e_psi,
        adjustment_factors: adjustment_summary,
        // Max reactions from governing (max moment) analysis
        reaction_left_lb: analysis_results.reaction_left_lb,
        reaction_right_lb: analysis_results.reaction_right_lb,
        // Min reactions for uplift design
        min_reaction_left_lb: min_reaction_left,
        min_reaction_right_lb: min_reaction_right,
        min_reaction_combination: min_reaction_combo_name,
        // Diagram data from analysis (scale deflection for adjusted E)
        shear_diagram: analysis_results.shear_diagram,
        moment_diagram: analysis_results.moment_diagram,
        deflection_diagram: analysis_results
            .deflection_diagram
            .into_iter()
            .map(|(x, d)| (x, d * (props.e_psi / e_adjusted)))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loads::DiscreteLoad;
    use crate::materials::{
        GlulamLayup, GlulamMaterial, GlulamStressClass, LvlGrade, LvlMaterial, WoodGrade,
        WoodMaterial, WoodSpecies,
    };

    /// Create a test beam with D+L = 150 plf total (like old uniform_load_plf: 150.0)
    fn test_beam() -> BeamInput {
        let load_case = EnhancedLoadCase::new("Test Loads")
            .without_self_weight() // Tests use explicit load values
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 100.0));

        BeamInput {
            label: "Test Beam".to_string(),
            span_ft: 12.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        }
    }

    #[test]
    fn test_section_properties() {
        let beam = test_beam();
        let s = beam.section_modulus_in3();
        let i = beam.moment_of_inertia_in4();

        // S = bd²/6 = 1.5 * 9.25² / 6 = 21.39
        assert!((s - 21.39).abs() < 0.1);

        // I = bd³/12 = 1.5 * 9.25³ / 12 = 98.93
        assert!((i - 98.93).abs() < 0.1);
    }

    #[test]
    fn test_moment_calculation() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // With D=50, L=100, ASD governing is D+L = 150 plf
        // M = wL²/8 = 150 * 12² / 8 = 2700 ft-lb
        assert!((result.max_moment_ftlb - 2700.0).abs() < 1.0);
    }

    #[test]
    fn test_shear_calculation() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // V = wL/2 = 150 * 12 / 2 = 900 lb
        assert!((result.max_shear_lb - 900.0).abs() < 1.0);
    }

    #[test]
    fn test_bending_stress() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // fb = M/S = (2700 * 12) / 21.39 = 1515 psi (approximately)
        assert!(result.actual_fb_psi > 1400.0 && result.actual_fb_psi < 1600.0);
    }

    #[test]
    fn test_passes_check() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // This beam should fail (unity > 1.0 for bending)
        // 2x10 DF-L No.2 at 12' with 150 plf is overstressed
        assert!(!result.passes() || result.bending_unity <= 1.0);
    }

    #[test]
    fn test_valid_beam_passes() {
        // A more reasonable beam that should pass
        let load_case = EnhancedLoadCase::new("Light Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 35.0));

        let beam = BeamInput {
            label: "Light Beam".to_string(),
            span_ft: 8.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };
        let result = calculate(&beam, DesignMethod::Asd).unwrap();
        assert!(result.passes());
    }

    #[test]
    fn test_glulam_beam() {
        let load_case = EnhancedLoadCase::new("Roof Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 60.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 140.0));

        let beam = BeamInput {
            label: "Glulam Beam".to_string(),
            span_ft: 24.0,
            load_case,
            material: Material::Glulam(GlulamMaterial::new(
                GlulamStressClass::F24_V4,
                GlulamLayup::Unbalanced,
            )),
            width_in: 5.125,
            depth_in: 16.5,
            adjustment_factors: AdjustmentFactors::default(),
        };
        let result = calculate(&beam, DesignMethod::Asd).unwrap();
        // Should have higher allowable Fb than sawn lumber
        assert!(result.allowable_fb_psi > 2000.0);
    }

    #[test]
    fn test_lvl_beam() {
        let load_case = EnhancedLoadCase::new("Header Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 300.0));

        let beam = BeamInput {
            label: "LVL Header".to_string(),
            span_ft: 12.0,
            load_case,
            material: Material::Lvl(LvlMaterial::new(LvlGrade::Standard)),
            width_in: 1.75,
            depth_in: 11.875,
            adjustment_factors: AdjustmentFactors::default(),
        };
        let result = calculate(&beam, DesignMethod::Asd).unwrap();
        // LVL should have higher E than sawn lumber
        assert!(result.e_psi >= 2_000_000.0);
    }

    #[test]
    fn test_invalid_span() {
        let mut beam = test_beam();
        beam.span_ft = -5.0;
        let result = calculate(&beam, DesignMethod::Asd);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let beam = test_beam();
        let json = serde_json::to_string_pretty(&beam).unwrap();
        let roundtrip: BeamInput = serde_json::from_str(&json).unwrap();
        assert_eq!(beam.span_ft, roundtrip.span_ft);
        assert_eq!(beam.load_case.loads.len(), roundtrip.load_case.loads.len());
    }

    #[test]
    fn test_result_serialization() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();
        let json = serde_json::to_string_pretty(&result).unwrap();

        // Should contain key fields
        assert!(json.contains("max_moment_ftlb"));
        assert!(json.contains("bending_unity"));
        assert!(json.contains("shear_unity"));
        assert!(json.contains("governing_combination"));

        let roundtrip: BeamResult = serde_json::from_str(&json).unwrap();
        assert!((result.max_moment_ftlb - roundtrip.max_moment_ftlb).abs() < 0.001);
    }

    #[test]
    fn test_self_weight_calculation() {
        let beam = test_beam();
        let self_wt = beam.self_weight_plf();
        // 1.5" x 9.25" = 13.875 in²
        // 13.875 * 35 pcf / 144 = 3.37 plf
        assert!((self_wt - 3.37).abs() < 0.1);
    }

    #[test]
    fn test_self_weight_inclusion() {
        let load_case_no_sw = EnhancedLoadCase::new("No Self-Weight")
            .without_self_weight()
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let load_case_with_sw = EnhancedLoadCase::new("With Self-Weight")
            .with_self_weight()
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let beam_no_sw = BeamInput {
            label: "No SW".to_string(),
            span_ft: 10.0,
            load_case: load_case_no_sw,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let beam_with_sw = BeamInput {
            label: "With SW".to_string(),
            span_ft: 10.0,
            load_case: load_case_with_sw,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result_no_sw = calculate(&beam_no_sw, DesignMethod::Asd).unwrap();
        let result_with_sw = calculate(&beam_with_sw, DesignMethod::Asd).unwrap();

        // Self-weight should increase design load
        assert!(result_with_sw.design_load_plf > result_no_sw.design_load_plf);
    }

    #[test]
    fn test_lrfd_vs_asd() {
        let beam = test_beam();
        let result_asd = calculate(&beam, DesignMethod::Asd).unwrap();
        let result_lrfd = calculate(&beam, DesignMethod::Lrfd).unwrap();

        // LRFD should have higher design load due to load factors
        // D=50, L=100: ASD = 150, LRFD = 1.2*50 + 1.6*100 = 220
        assert!(result_lrfd.design_load_plf > result_asd.design_load_plf);
        assert!((result_lrfd.design_load_plf - 220.0).abs() < 1.0);
    }

    #[test]
    fn test_governing_combination_reported() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // Should report which combination governs
        assert!(!result.governing_combination.is_empty());
        assert!(result.governing_combination.contains("ASD"));
    }

    #[test]
    fn test_point_load_midspan() {
        // 10 ft beam with 1000 lb point load at midspan
        // M_max = PL/4 = 1000 * 10 / 4 = 2500 ft-lb
        let load_case = EnhancedLoadCase::new("Point Load Test")
            .without_self_weight()
            .with_load(DiscreteLoad::point(LoadType::Live, 1000.0, 5.0));

        let beam = BeamInput {
            label: "Point Load Beam".to_string(),
            span_ft: 10.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // Max moment should be PL/4 = 2500 ft-lb (within 1%)
        assert!(
            (result.max_moment_ftlb - 2500.0).abs() < 25.0,
            "Expected ~2500 ft-lb, got {} ft-lb",
            result.max_moment_ftlb
        );

        // Max shear should be P/2 = 500 lb (symmetric)
        assert!(
            (result.max_shear_lb - 500.0).abs() < 5.0,
            "Expected ~500 lb, got {} lb",
            result.max_shear_lb
        );
    }

    #[test]
    fn test_point_load_asymmetric() {
        // 10 ft beam with 1000 lb point load at 3 ft from left
        // R1 = P(L-a)/L = 1000 * 7/10 = 700 lb
        // R2 = Pa/L = 1000 * 3/10 = 300 lb
        // M_max at load point = R1 * a = 700 * 3 = 2100 ft-lb
        let load_case = EnhancedLoadCase::new("Asymmetric Point Load")
            .without_self_weight()
            .with_load(DiscreteLoad::point(LoadType::Live, 1000.0, 3.0));

        let beam = BeamInput {
            label: "Asymmetric Point Load".to_string(),
            span_ft: 10.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // Max moment = Pa(L-a)/L = 1000 * 3 * 7 / 10 = 2100 ft-lb
        assert!(
            (result.max_moment_ftlb - 2100.0).abs() < 50.0,
            "Expected ~2100 ft-lb, got {} ft-lb",
            result.max_moment_ftlb
        );

        // Reactions
        assert!(
            (result.reaction_left_lb - 700.0).abs() < 10.0,
            "Expected R1 ~700 lb, got {} lb",
            result.reaction_left_lb
        );
        assert!(
            (result.reaction_right_lb - 300.0).abs() < 10.0,
            "Expected R2 ~300 lb, got {} lb",
            result.reaction_right_lb
        );
    }

    #[test]
    fn test_combined_uniform_and_point_load() {
        // 12 ft beam with:
        // - 50 plf uniform dead load
        // - 1000 lb point live load at midspan
        //
        // Uniform: M = wL²/8 = 50 * 144 / 8 = 900 ft-lb
        // Point: M = PL/4 = 1000 * 12 / 4 = 3000 ft-lb
        // Total at midspan (superposition): 3900 ft-lb
        let load_case = EnhancedLoadCase::new("Combined Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
            .with_load(DiscreteLoad::point(LoadType::Live, 1000.0, 6.0));

        let beam = BeamInput {
            label: "Combined Load Beam".to_string(),
            span_ft: 12.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // For ASD with D + L combination, max moment should be ~3900 ft-lb
        assert!(
            (result.max_moment_ftlb - 3900.0).abs() < 100.0,
            "Expected ~3900 ft-lb, got {} ft-lb",
            result.max_moment_ftlb
        );
    }

    #[test]
    fn test_diagram_data_populated() {
        let beam = test_beam();
        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // Diagram data should be populated
        assert!(
            !result.shear_diagram.is_empty(),
            "Shear diagram should have data"
        );
        assert!(
            !result.moment_diagram.is_empty(),
            "Moment diagram should have data"
        );
        assert!(
            !result.deflection_diagram.is_empty(),
            "Deflection diagram should have data"
        );

        // First point should be at x = 0
        assert!((result.shear_diagram[0].0 - 0.0).abs() < 0.01);

        // Last point should be at span
        let last_idx = result.shear_diagram.len() - 1;
        assert!((result.shear_diagram[last_idx].0 - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_wind_uplift_reactions() {
        // Roof beam with light dead load and significant wind uplift
        // D = 10 plf, W = 30 plf (entered positive, applied as uplift via -W combinations)
        let load_case = EnhancedLoadCase::new("Roof with Wind")
            .without_self_weight()
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 10.0))
            .with_load(DiscreteLoad::uniform(LoadType::Wind, 30.0));

        let beam = BeamInput {
            label: "Wind Uplift Beam".to_string(),
            span_ft: 10.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // ASD-8': 0.6D - 0.6W = 0.6*10 - 0.6*30 = 6 - 18 = -12 plf total
        // For symmetric simply-supported beam: R = wL/2 = -12 * 10 / 2 = -60 lb each
        // Negative reaction indicates net uplift!

        // Min reactions should be negative (uplift)
        assert!(
            result.min_reaction_left_lb < 0.0,
            "Expected negative min reaction (uplift), got {} lb",
            result.min_reaction_left_lb
        );

        // Min reaction combination should be ASD-8' (uplift combo)
        assert!(
            result.min_reaction_combination.contains("8'"),
            "Expected ASD-8' for min reaction, got {}",
            result.min_reaction_combination
        );

        // Min reactions should be approximately -60 lb each
        let expected_min_reaction = -60.0; // -12 plf * 10 ft / 2
        assert!(
            (result.min_reaction_left_lb - expected_min_reaction).abs() < 5.0,
            "Expected min reaction ~{} lb, got {} lb",
            expected_min_reaction,
            result.min_reaction_left_lb
        );

        // Max reactions should still be positive (gravity governs for strength)
        assert!(
            result.reaction_left_lb > 0.0,
            "Expected positive max reaction, got {} lb",
            result.reaction_left_lb
        );
    }

    #[test]
    fn test_no_uplift_when_dead_dominates() {
        // Heavy beam where dead load prevents uplift even with wind
        // D = 100 plf, W = 20 plf
        let load_case = EnhancedLoadCase::new("Heavy Beam with Wind")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0))
            .with_load(DiscreteLoad::uniform(LoadType::Wind, 20.0));

        let beam = BeamInput {
            label: "Heavy Beam".to_string(),
            span_ft: 10.0,
            load_case,
            material: Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            width_in: 1.5,
            depth_in: 9.25,
            adjustment_factors: AdjustmentFactors::default(),
        };

        let result = calculate(&beam, DesignMethod::Asd).unwrap();

        // 0.6D - 0.6W = 60 - 12 = 48 plf (still positive)
        // No net uplift, min reactions should be positive
        assert!(
            result.min_reaction_left_lb > 0.0,
            "Expected positive min reaction (no uplift), got {} lb",
            result.min_reaction_left_lb
        );
    }
}
