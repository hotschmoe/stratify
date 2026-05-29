//! # Continuous Beam Analysis
//!
//! Multi-span beam analysis with configurable support conditions.
//! Supports pinned, fixed, and free (cantilever) end conditions.
//!
//! ## Analysis Methods
//!
//! - **Single-span with fixed ends**: Closed-form solutions from Roark's Formulas
//! - **Multi-span continuous beams**: Hardy Cross moment distribution
//!
//! ## Notation
//!
//! - N spans creates N+1 nodes (support locations)
//! - Nodes are numbered 0 to N (left to right)
//! - Spans are numbered 0 to N-1 (left to right)
//!
//! ## Example
//!
//! ```rust
//! use calc_core::calculations::continuous_beam::{
//!     ContinuousBeamInput, SupportType, SpanSegment
//! };
//! use calc_core::materials::Material;
//! use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType};
//!
//! // Two-span continuous beam: 12' + 10', pinned at all supports
//! let input = ContinuousBeamInput {
//!     label: "CB-1".to_string(),
//!     spans: vec![
//!         SpanSegment::new(12.0, 1.5, 9.25, Material::default()),
//!         SpanSegment::new(10.0, 1.5, 9.25, Material::default()),
//!     ],
//!     supports: vec![
//!         SupportType::Pinned,  // Left end
//!         SupportType::Pinned,  // Interior support
//!         SupportType::Pinned,  // Right end
//!     ],
//!     load_case: EnhancedLoadCase::new("Floor")
//!         .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
//!         .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0)),
//!     ..Default::default()
//! };
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{CalcError, CalcResult};
use crate::loads::{EnhancedLoadCase, LoadDistribution, LoadType};
use crate::materials::Material;
use crate::nds_factors::AdjustmentFactors;
use crate::section_deductions::SectionDeductions;

// =============================================================================
// SUPPORT TYPE
// =============================================================================

/// Support condition at a node (support location)
///
/// Each node in a continuous beam can have one of these support types,
/// which determines its boundary conditions for analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SupportType {
    /// Free end - no restraint (cantilever end)
    ///
    /// - Vertical displacement: free
    /// - Rotation: free
    /// - Use for cantilever overhangs
    Free,

    /// Pinned/hinged support - restrains vertical displacement, allows rotation
    ///
    /// - Vertical displacement: restrained (Δ = 0)
    /// - Rotation: free
    /// - Most common support type
    #[default]
    Pinned,

    /// Roller support - same as pinned for vertical beam analysis
    ///
    /// - Vertical displacement: restrained (Δ = 0)
    /// - Rotation: free
    /// - Equivalent to Pinned for gravity load analysis
    Roller,

    /// Fixed support - restrains both displacement and rotation
    ///
    /// - Vertical displacement: restrained (Δ = 0)
    /// - Rotation: restrained (θ = 0)
    /// - Creates moment reaction at support
    Fixed,
}

impl SupportType {
    /// All available support types for UI selection
    pub const ALL: [SupportType; 4] = [
        SupportType::Pinned,
        SupportType::Roller,
        SupportType::Fixed,
        SupportType::Free,
    ];

    /// Returns true if this support restrains vertical displacement
    pub fn restrains_vertical(&self) -> bool {
        matches!(
            self,
            SupportType::Pinned | SupportType::Roller | SupportType::Fixed
        )
    }

    /// Returns true if this support restrains rotation
    pub fn restrains_rotation(&self) -> bool {
        matches!(self, SupportType::Fixed)
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            SupportType::Free => "Free",
            SupportType::Pinned => "Pinned",
            SupportType::Roller => "Roller",
            SupportType::Fixed => "Fixed",
        }
    }

    /// Get short symbol for diagrams
    pub fn symbol(&self) -> &'static str {
        match self {
            SupportType::Free => "",
            SupportType::Pinned => "△",
            SupportType::Roller => "○",
            SupportType::Fixed => "▣",
        }
    }
}

impl std::fmt::Display for SupportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// SPAN SEGMENT
// =============================================================================

/// A single span segment between two nodes
///
/// Each span can have its own section properties and material.
/// For uniform beams, all spans will have the same properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanSegment {
    /// Unique identifier for this span
    pub id: Uuid,

    /// Span length in feet
    pub length_ft: f64,

    /// Actual beam width in inches
    pub width_in: f64,

    /// Actual beam depth in inches
    pub depth_in: f64,

    /// Material for this span
    pub material: Material,

    /// Optional user label for this span (e.g., "Span 1", "Over kitchen")
    #[serde(default)]
    pub label: String,
}

impl SpanSegment {
    /// Create a new span segment
    pub fn new(length_ft: f64, width_in: f64, depth_in: f64, material: Material) -> Self {
        Self {
            id: Uuid::new_v4(),
            length_ft,
            width_in,
            depth_in,
            material,
            label: String::new(),
        }
    }

    /// Create with a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Create with a specific UUID
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Calculate moment of inertia I = bd³/12 (in⁴)
    pub fn moment_of_inertia_in4(&self) -> f64 {
        self.width_in * self.depth_in.powi(3) / 12.0
    }

    /// Calculate section modulus S = bd²/6 (in³)
    pub fn section_modulus_in3(&self) -> f64 {
        self.width_in * self.depth_in.powi(2) / 6.0
    }

    /// Calculate cross-sectional area A = bd (in²)
    pub fn area_in2(&self) -> f64 {
        self.width_in * self.depth_in
    }

    /// Get modulus of elasticity from material (psi)
    pub fn e_psi(&self) -> CalcResult<f64> {
        Ok(self.material.base_properties()?.e_psi)
    }

    /// Get minimum modulus for stability (psi)
    pub fn e_min_psi(&self) -> CalcResult<f64> {
        Ok(self.material.base_properties()?.e_min_psi)
    }

    /// Calculate flexural stiffness EI (lb-in²)
    pub fn ei(&self) -> CalcResult<f64> {
        Ok(self.e_psi()? * self.moment_of_inertia_in4())
    }

    /// Calculate stiffness factor K = EI/L (lb-in)
    ///
    /// This is used for moment distribution calculations.
    /// Returns K in consistent units (converts L to inches).
    pub fn stiffness_k(&self) -> CalcResult<f64> {
        let l_in = self.length_ft * 12.0;
        Ok(self.ei()? / l_in)
    }

    /// Calculate modified stiffness for a pin-ended far end (3EI/L)
    ///
    /// Used when the far end of the span is pinned/roller.
    pub fn stiffness_k_modified(&self) -> CalcResult<f64> {
        Ok(self.stiffness_k()? * 0.75) // 3EI/L = 0.75 * 4EI/L
    }

    /// Self-weight in plf (assuming 35 pcf wood density)
    pub fn self_weight_plf(&self) -> f64 {
        const WOOD_DENSITY_PCF: f64 = 35.0;
        self.area_in2() * WOOD_DENSITY_PCF / 144.0
    }

    /// Validate span parameters
    pub fn validate(&self) -> CalcResult<()> {
        if self.length_ft <= 0.0 {
            return Err(CalcError::invalid_input(
                "length_ft",
                self.length_ft.to_string(),
                "Span length must be positive",
            ));
        }
        if self.length_ft > 60.0 {
            return Err(CalcError::invalid_input(
                "length_ft",
                self.length_ft.to_string(),
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
}

impl Default for SpanSegment {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            length_ft: 12.0,
            width_in: 1.5,
            depth_in: 9.25,
            material: Material::default(),
            label: String::new(),
        }
    }
}

// =============================================================================
// CONTINUOUS BEAM INPUT
// =============================================================================

/// Input for continuous beam analysis
///
/// A continuous beam consists of multiple spans connected at nodes (supports).
/// Each span can have different section properties and materials.
/// Each node can have different support conditions.
///
/// ## Node/Span Relationship
///
/// For N spans, there are N+1 nodes (support locations):
///
/// ```text
/// Node 0    Node 1    Node 2    Node 3
///   |--------|---------|---------|
///    Span 0    Span 1    Span 2
/// ```
///
/// ## Support Configuration Examples
///
/// **Simply-supported single span:**
/// - 1 span, 2 nodes: [Pinned, Roller]
///
/// **Two-span continuous:**
/// - 2 spans, 3 nodes: [Pinned, Pinned, Roller]
///
/// **Cantilever:**
/// - 1 span, 2 nodes: [Fixed, Free]
///
/// **Propped cantilever:**
/// - 1 span, 2 nodes: [Fixed, Roller]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousBeamInput {
    /// User label for this beam
    pub label: String,

    /// Span segments (ordered left to right)
    ///
    /// Must have at least one span. For a single-span beam, this has one entry.
    pub spans: Vec<SpanSegment>,

    /// Support conditions at each node
    ///
    /// Length must be exactly `spans.len() + 1`.
    /// - `supports[0]` is the left end
    /// - `supports[spans.len()]` is the right end
    /// - Interior indices are interior supports
    pub supports: Vec<SupportType>,

    /// Load case with discrete loads
    ///
    /// Load positions are measured from the left end of the entire beam.
    /// For multi-span beams, loads may span across multiple spans.
    pub load_case: EnhancedLoadCase,

    /// NDS adjustment factors
    #[serde(default)]
    pub adjustment_factors: AdjustmentFactors,

    /// Section deductions (notches, holes)
    #[serde(default)]
    pub section_deductions: SectionDeductions,
}

impl ContinuousBeamInput {
    /// Create a simple single-span beam (simply-supported)
    ///
    /// This is the most common configuration and matches the behavior
    /// of the original `BeamInput` struct.
    pub fn simple_span(
        label: impl Into<String>,
        span_ft: f64,
        width_in: f64,
        depth_in: f64,
        material: Material,
        load_case: EnhancedLoadCase,
    ) -> Self {
        let span = SpanSegment::new(span_ft, width_in, depth_in, material);
        Self {
            label: label.into(),
            spans: vec![span],
            supports: vec![SupportType::Pinned, SupportType::Pinned],
            load_case,
            adjustment_factors: AdjustmentFactors::default(),
            section_deductions: SectionDeductions::default(),
        }
    }

    /// Create a cantilever beam (fixed at left, free at right)
    pub fn cantilever(
        label: impl Into<String>,
        span_ft: f64,
        width_in: f64,
        depth_in: f64,
        material: Material,
        load_case: EnhancedLoadCase,
    ) -> Self {
        let span = SpanSegment::new(span_ft, width_in, depth_in, material);
        Self {
            label: label.into(),
            spans: vec![span],
            supports: vec![SupportType::Fixed, SupportType::Free],
            load_case,
            adjustment_factors: AdjustmentFactors::default(),
            section_deductions: SectionDeductions::default(),
        }
    }

    /// Create a fixed-fixed beam
    pub fn fixed_fixed(
        label: impl Into<String>,
        span_ft: f64,
        width_in: f64,
        depth_in: f64,
        material: Material,
        load_case: EnhancedLoadCase,
    ) -> Self {
        let span = SpanSegment::new(span_ft, width_in, depth_in, material);
        Self {
            label: label.into(),
            spans: vec![span],
            supports: vec![SupportType::Fixed, SupportType::Fixed],
            load_case,
            adjustment_factors: AdjustmentFactors::default(),
            section_deductions: SectionDeductions::default(),
        }
    }

    /// Create a multi-span beam with explicit spans and supports
    ///
    /// The supports vector must have length = spans.len() + 1
    /// (one support at each node including both ends).
    pub fn new(
        label: impl Into<String>,
        spans: Vec<SpanSegment>,
        supports: Vec<SupportType>,
        load_case: EnhancedLoadCase,
    ) -> Self {
        Self {
            label: label.into(),
            spans,
            supports,
            load_case,
            adjustment_factors: AdjustmentFactors::default(),
            section_deductions: SectionDeductions::default(),
        }
    }

    /// Total length of all spans combined (ft)
    pub fn total_length_ft(&self) -> f64 {
        self.spans.iter().map(|s| s.length_ft).sum()
    }

    /// Number of spans
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Number of nodes (always span_count + 1)
    pub fn node_count(&self) -> usize {
        self.spans.len() + 1
    }

    /// Get cumulative positions of each node from left end (ft)
    ///
    /// Returns a vector of length `node_count()` where:
    /// - positions[0] = 0.0 (left end)
    /// - positions[i] = sum of lengths of spans 0 to i-1
    pub fn node_positions(&self) -> Vec<f64> {
        let mut positions = vec![0.0];
        let mut cumulative = 0.0;
        for span in &self.spans {
            cumulative += span.length_ft;
            positions.push(cumulative);
        }
        positions
    }

    /// Check if this is a single-span beam
    pub fn is_single_span(&self) -> bool {
        self.spans.len() == 1
    }

    /// Check if beam is simply-supported (single span, pin-roller)
    pub fn is_simply_supported(&self) -> bool {
        self.spans.len() == 1
            && matches!(
                self.supports.as_slice(),
                [SupportType::Pinned, SupportType::Roller]
                    | [SupportType::Pinned, SupportType::Pinned]
                    | [SupportType::Roller, SupportType::Pinned]
            )
    }

    /// Check if beam is a cantilever (fixed at one end, free at other)
    pub fn is_cantilever(&self) -> bool {
        self.spans.len() == 1
            && matches!(
                self.supports.as_slice(),
                [SupportType::Fixed, SupportType::Free] | [SupportType::Free, SupportType::Fixed]
            )
    }

    /// Check if beam is fixed at both ends
    pub fn is_fixed_fixed(&self) -> bool {
        self.spans.len() == 1
            && matches!(
                self.supports.as_slice(),
                [SupportType::Fixed, SupportType::Fixed]
            )
    }

    /// Check if beam has any fixed supports (requires indeterminate analysis)
    pub fn has_fixed_support(&self) -> bool {
        self.supports.contains(&SupportType::Fixed)
    }

    /// Check if structure is statically indeterminate
    ///
    /// Indeterminate if:
    /// - Multiple spans (continuous), OR
    /// - Any fixed support
    pub fn is_indeterminate(&self) -> bool {
        self.spans.len() > 1 || self.has_fixed_support()
    }

    /// Validate input parameters
    pub fn validate(&self) -> CalcResult<()> {
        // Must have at least one span
        if self.spans.is_empty() {
            return Err(CalcError::invalid_input(
                "spans",
                "empty",
                "At least one span is required",
            ));
        }

        // Must have correct number of supports
        let expected_supports = self.spans.len() + 1;
        if self.supports.len() != expected_supports {
            return Err(CalcError::invalid_input(
                "supports",
                self.supports.len().to_string(),
                format!(
                    "Expected {} supports for {} spans",
                    expected_supports,
                    self.spans.len()
                ),
            ));
        }

        // Validate each span
        for (i, span) in self.spans.iter().enumerate() {
            span.validate().map_err(|e| {
                CalcError::invalid_input(format!("spans[{i}]"), "invalid", e.to_string())
            })?;
        }

        // Must have at least one vertical support for stability
        let vertical_supports: usize = self
            .supports
            .iter()
            .filter(|s| s.restrains_vertical())
            .count();

        if vertical_supports == 0 {
            return Err(CalcError::invalid_input(
                "supports",
                "none restrained",
                "Structure is unstable - at least one vertical support is required",
            ));
        }

        // Check for unstable cantilever configuration
        // (free end without a fixed support somewhere)
        let has_free = self.supports.contains(&SupportType::Free);
        let has_fixed = self.supports.contains(&SupportType::Fixed);

        if has_free && !has_fixed && vertical_supports < 2 {
            return Err(CalcError::invalid_input(
                "supports",
                "unstable cantilever",
                "Cantilever requires a fixed support or two vertical supports",
            ));
        }

        Ok(())
    }

    /// Add a span to the right end
    pub fn add_span(&mut self, span: SpanSegment) {
        self.spans.push(span);
        // Add a default support for the new node (pinned for standard convention)
        self.supports.push(SupportType::Pinned);
    }

    /// Remove the rightmost span (if more than one exists)
    pub fn remove_last_span(&mut self) -> Option<SpanSegment> {
        if self.spans.len() > 1 {
            self.supports.pop(); // Remove the rightmost support
            self.spans.pop()
        } else {
            None
        }
    }
}

impl Default for ContinuousBeamInput {
    fn default() -> Self {
        Self {
            label: String::new(),
            spans: vec![SpanSegment::default()],
            supports: vec![SupportType::Pinned, SupportType::Pinned],
            load_case: EnhancedLoadCase::default(),
            adjustment_factors: AdjustmentFactors::default(),
            section_deductions: SectionDeductions::default(),
        }
    }
}

// =============================================================================
// CONTINUOUS BEAM RESULT
// =============================================================================

/// Results for a single span within a continuous beam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult {
    /// Span index (0-based)
    pub span_index: usize,

    /// Left-end moment (ft-lb) - positive causes tension on bottom
    pub moment_left_ftlb: f64,

    /// Right-end moment (ft-lb)
    pub moment_right_ftlb: f64,

    /// Left-end shear (lb) - positive upward
    pub shear_left_lb: f64,

    /// Right-end shear (lb)
    pub shear_right_lb: f64,

    /// Maximum positive moment in span (ft-lb)
    pub max_positive_moment_ftlb: f64,

    /// Position of max positive moment from left end of span (ft)
    pub max_positive_moment_pos_ft: f64,

    /// Maximum negative moment in span (ft-lb) - at supports
    pub max_negative_moment_ftlb: f64,

    /// Maximum shear magnitude in span (lb)
    pub max_shear_lb: f64,

    /// Maximum deflection in span (in)
    pub max_deflection_in: f64,

    /// Position of max deflection from left end of span (ft)
    pub max_deflection_pos_ft: f64,

    /// Actual bending stress at max moment (psi)
    pub actual_fb_psi: f64,

    /// Allowable bending stress (psi)
    pub allowable_fb_psi: f64,

    /// Bending unity ratio
    pub bending_unity: f64,

    /// Actual shear stress (psi)
    pub actual_fv_psi: f64,

    /// Allowable shear stress (psi)
    pub allowable_fv_psi: f64,

    /// Shear unity ratio
    pub shear_unity: f64,

    /// Deflection unity ratio
    pub deflection_unity: f64,
}

impl SpanResult {
    /// Check if this span passes all checks
    pub fn passes(&self) -> bool {
        self.bending_unity <= 1.0 && self.shear_unity <= 1.0 && self.deflection_unity <= 1.0
    }

    /// Get governing unity ratio for this span
    pub fn governing_unity(&self) -> f64 {
        self.bending_unity
            .max(self.shear_unity)
            .max(self.deflection_unity)
    }
}

/// Results from continuous beam analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousBeamResult {
    // === Per-Span Results ===
    /// Results for each span
    pub span_results: Vec<SpanResult>,

    // === Per-Node Results ===
    /// Reaction force at each node (lb) - positive upward
    pub reactions: Vec<f64>,

    /// Support moment at each node (ft-lb)
    ///
    /// Non-zero at fixed supports and interior supports of continuous beams.
    pub support_moments: Vec<f64>,

    /// Rotation at each node (radians)
    pub rotations: Vec<f64>,

    // === Global Extrema ===
    /// Maximum positive moment across all spans (ft-lb)
    pub max_positive_moment_ftlb: f64,

    /// Location: (span_index, position_within_span_ft)
    pub max_positive_moment_location: (usize, f64),

    /// Maximum negative moment magnitude (ft-lb) - typically at supports
    pub max_negative_moment_ftlb: f64,

    /// Node index of max negative moment
    pub max_negative_moment_node: usize,

    /// Maximum shear across all spans (lb)
    pub max_shear_lb: f64,

    /// Location: (span_index, position_within_span_ft)
    pub max_shear_location: (usize, f64),

    /// Maximum deflection across all spans (in)
    pub max_deflection_in: f64,

    /// Location: (span_index, position_within_span_ft)
    pub max_deflection_location: (usize, f64),

    // === Global Design Checks ===
    /// Governing unity ratio across all spans
    pub governing_unity: f64,

    /// Which span governs (index)
    pub governing_span: usize,

    /// What condition governs
    pub governing_condition: String,

    // === Diagram Data ===
    /// Shear diagram: (position_from_left_ft, shear_lb)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shear_diagram: Vec<(f64, f64)>,

    /// Moment diagram: (position_from_left_ft, moment_ftlb)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moment_diagram: Vec<(f64, f64)>,

    /// Deflection diagram: (position_from_left_ft, deflection_in)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deflection_diagram: Vec<(f64, f64)>,

    // === Load Info ===
    /// Governing load combination name
    pub governing_combination: String,

    /// Minimum reaction combination (for uplift)
    pub min_reaction_combination: String,

    /// Minimum reactions at each node (for uplift design)
    pub min_reactions: Vec<f64>,
}

impl ContinuousBeamResult {
    /// Check if all spans pass all checks
    pub fn passes(&self) -> bool {
        self.governing_unity <= 1.0
    }

    /// Get overall pass/fail status description
    pub fn status(&self) -> &'static str {
        if self.passes() {
            "PASS"
        } else {
            "FAIL"
        }
    }
}

impl Default for ContinuousBeamResult {
    fn default() -> Self {
        Self {
            span_results: Vec::new(),
            reactions: Vec::new(),
            support_moments: Vec::new(),
            rotations: Vec::new(),
            max_positive_moment_ftlb: 0.0,
            max_positive_moment_location: (0, 0.0),
            max_negative_moment_ftlb: 0.0,
            max_negative_moment_node: 0,
            max_shear_lb: 0.0,
            max_shear_location: (0, 0.0),
            max_deflection_in: 0.0,
            max_deflection_location: (0, 0.0),
            governing_unity: 0.0,
            governing_span: 0,
            governing_condition: String::new(),
            shear_diagram: Vec::new(),
            moment_diagram: Vec::new(),
            deflection_diagram: Vec::new(),
            governing_combination: String::new(),
            min_reaction_combination: String::new(),
            min_reactions: Vec::new(),
        }
    }
}

// =============================================================================
// CALCULATION FUNCTION
// =============================================================================

use crate::loads::DesignMethod;

/// Calculate continuous beam results
///
/// Handles all beam configurations:
/// - Single-span simply-supported (pin-roller)
/// - Single-span with fixed ends (fixed-fixed, fixed-pin, cantilever)
/// - Multi-span continuous beams
///
/// # Arguments
///
/// * `input` - Continuous beam parameters
/// * `method` - Design method (ASD or LRFD)
///
/// # Returns
///
/// * `Ok(ContinuousBeamResult)` - Calculation results
/// * `Err(CalcError)` - If inputs are invalid
pub fn calculate_continuous(
    input: &ContinuousBeamInput,
    method: DesignMethod,
) -> CalcResult<ContinuousBeamResult> {
    use crate::calculations::moment_distribution::analyze_moment_distribution;
    use crate::loads::LoadType;

    input.validate()?;

    let combinations = method.combinations();
    let _n_spans = input.span_count();
    let n_nodes = input.node_count();
    let _node_positions = input.node_positions();

    // Track governing results
    let mut governing_result: Option<ContinuousBeamResult> = None;
    let mut max_moment = 0.0f64;
    let mut min_reaction_total = f64::MAX;
    let mut min_reaction_combo_name = String::new();
    let mut min_reactions: Vec<f64> = vec![0.0; n_nodes];

    for combo in &combinations {
        // Build load factors for this combination
        let load_factors: Vec<(LoadType, f64)> = LoadType::ALL
            .iter()
            .map(|lt| (*lt, combo.get_factor(*lt)))
            .collect();

        // Run moment distribution analysis
        let dist_result = analyze_moment_distribution(input, &load_factors)?;

        // Build result from moment distribution output
        let mut result = build_result_from_distribution(
            input,
            &dist_result,
            &combo.name,
            method,
            &load_factors,
        )?;

        // Check if this combination governs for max moment
        if result.max_positive_moment_ftlb > max_moment {
            max_moment = result.max_positive_moment_ftlb;
            result.governing_combination = combo.name.clone();
            governing_result = Some(result.clone());
        }

        // Check for minimum reactions (uplift)
        let reaction_sum: f64 = result.reactions.iter().sum();
        if reaction_sum < min_reaction_total {
            min_reaction_total = reaction_sum;
            min_reaction_combo_name = combo.name.clone();
            min_reactions = result.reactions.clone();
        }
    }

    let mut final_result = governing_result.unwrap_or_else(|| {
        // Fallback for empty load case
        ContinuousBeamResult::default()
    });

    final_result.min_reaction_combination = min_reaction_combo_name;
    final_result.min_reactions = min_reactions;

    Ok(final_result)
}

/// Build a ContinuousBeamResult from moment distribution output
fn build_result_from_distribution(
    input: &ContinuousBeamInput,
    dist_result: &crate::calculations::moment_distribution::DistributionResult,
    combo_name: &str,
    _method: DesignMethod,
    load_factors: &[(LoadType, f64)],
) -> CalcResult<ContinuousBeamResult> {
    use crate::equations::beam::{
        partial_uniform_reactions, point_load_deflection, point_load_reactions,
        uniform_load_deflection, uniform_load_reactions,
    };
    use crate::nds_factors::{BeamStability, SizeFactor};

    // Helper to get load factor for a given load type
    let get_factor = |lt: LoadType| -> f64 {
        load_factors
            .iter()
            .find(|(t, _)| *t == lt)
            .map(|(_, f)| *f)
            .unwrap_or(1.0)
    };

    let n_spans = input.span_count();
    let n_nodes = input.node_count();
    let node_positions = input.node_positions();

    let mut span_results = Vec::with_capacity(n_spans);
    let mut reactions = vec![0.0; n_nodes];
    let mut shear_diagram: Vec<(f64, f64)> = Vec::new();
    let mut moment_diagram: Vec<(f64, f64)> = Vec::new();
    let mut deflection_diagram: Vec<(f64, f64)> = Vec::new();

    // Track global extrema
    let mut max_positive_moment = 0.0f64;
    let mut max_positive_moment_loc = (0, 0.0);
    let mut max_negative_moment = 0.0f64;
    let mut max_negative_node = 0;
    let mut max_shear = 0.0f64;
    let mut max_shear_loc = (0, 0.0);
    let mut max_deflection = 0.0f64;
    let mut max_deflection_loc = (0, 0.0);
    let mut governing_unity = 0.0f64;
    let mut governing_span = 0;
    let mut governing_condition = String::from("Bending");

    // Process each span
    for (i, span) in input.spans.iter().enumerate() {
        let m_left = dist_result.span_moments_left[i];
        let m_right = dist_result.span_moments_right[i];
        let span_start = node_positions[i];
        let l = span.length_ft;
        let l_in = l * 12.0;

        // 1. Calculate Simple Span Reactions from ALL loads (with factors)
        let mut simple_r1 = 0.0;
        let mut simple_r2 = 0.0;

        for load in &input.load_case.loads {
            let factor = get_factor(load.load_type);
            if factor.abs() < 1e-10 {
                continue;
            }
            let magnitude = load.effective_magnitude() * factor;
            match &load.distribution {
                LoadDistribution::UniformFull => {
                    let (r1, r2) = uniform_load_reactions(magnitude, l);
                    simple_r1 += r1;
                    simple_r2 += r2;
                }
                LoadDistribution::Point { position_ft } => {
                    if *position_ft >= span_start && *position_ft <= span_start + l {
                        let local_a = position_ft - span_start;
                        let (r1, r2) = point_load_reactions(magnitude, local_a, l);
                        simple_r1 += r1;
                        simple_r2 += r2;
                    }
                }
                LoadDistribution::UniformPartial { start_ft, end_ft } => {
                    let span_end = span_start + l;
                    if *start_ft < span_end && *end_ft > span_start {
                        let local_start = (*start_ft - span_start).max(0.0);
                        let local_end = (*end_ft - span_start).min(l);
                        if local_end > local_start {
                            let (r1, r2) =
                                partial_uniform_reactions(magnitude, local_start, local_end, l);
                            simple_r1 += r1;
                            simple_r2 += r2;
                        }
                    }
                }
                _ => {}
            }
        }

        // Add self-weight if enabled (as dead load with factor)
        if input.load_case.include_self_weight {
            let dead_factor = get_factor(LoadType::Dead);
            let sw = span.self_weight_plf() * dead_factor;
            let (r1, r2) = uniform_load_reactions(sw, l);
            simple_r1 += r1;
            simple_r2 += r2;
        }

        // 2. Adjust reactions for end moments
        let delta_r = (m_right - m_left) / l;
        let r_left = simple_r1 - delta_r;
        let r_right = simple_r2 + delta_r;

        reactions[i] += r_left;
        reactions[i + 1] += r_right;

        // 3. Generate diagram points
        let v_left = r_left;
        let v_right = -r_right;
        let mut span_max_shear = 0.0f64;
        let mut span_max_pos_moment = 0.0f64;
        let mut span_max_pos_moment_x = 0.0;
        let mut max_defl = 0.0f64;

        let e = span.e_psi()?;
        let i_val = span.moment_of_inertia_in4();
        let _s = span.section_modulus_in3();
        let _area = span.area_in2();
        let ei = span.ei()?;

        let num_points = 51;
        for p in 0..num_points {
            let x = l * p as f64 / (num_points - 1) as f64;
            let x_in = x * 12.0;
            let m_left_in = m_left * 12.0;
            let m_right_in = m_right * 12.0;

            // Start with reaction contributions
            let mut v = r_left;
            let mut m = m_left + r_left * x;
            let mut defl = 0.0;

            // Add end moment deflection contribution
            let term_moments = x_in
                * (l_in - x_in)
                * (m_left_in * (2.0 * l_in - x_in) + m_right_in * (l_in + x_in))
                / (6.0 * ei * l_in);
            defl += term_moments;

            // Superimpose all loads
            for load in &input.load_case.loads {
                let factor = get_factor(load.load_type);
                if factor.abs() < 1e-10 {
                    continue;
                }
                let magnitude = load.effective_magnitude() * factor;
                match &load.distribution {
                    LoadDistribution::UniformFull => {
                        v -= magnitude * x;
                        m -= magnitude * x * x / 2.0;
                        defl += uniform_load_deflection(magnitude / 12.0, l_in, x_in, e, i_val);
                    }
                    LoadDistribution::Point { position_ft } => {
                        if *position_ft >= span_start && *position_ft <= span_start + l {
                            let local_a = *position_ft - span_start;
                            if x > local_a {
                                v -= magnitude;
                                m -= magnitude * (x - local_a);
                            }
                            defl += point_load_deflection(
                                magnitude,
                                local_a * 12.0,
                                l_in,
                                x_in,
                                e,
                                i_val,
                            );
                        }
                    }
                    LoadDistribution::UniformPartial { start_ft, end_ft } => {
                        let span_end = span_start + l;
                        if *start_ft < span_end && *end_ft > span_start {
                            let local_start = (*start_ft - span_start).max(0.0);
                            let local_end = (*end_ft - span_start).min(l);
                            if local_end > local_start && x > local_start {
                                let active_end = x.min(local_end);
                                let active_len = active_end - local_start;
                                v -= magnitude * active_len;
                                let load_force = magnitude * active_len;
                                let centroid = local_start + active_len / 2.0;
                                m -= load_force * (x - centroid);
                            }
                            // Add deflection using numerical integration
                            // Treat partial load as multiple point loads
                            let num_segments = 20;
                            let seg_len = (local_end - local_start) / num_segments as f64;
                            let seg_load = magnitude * seg_len; // lbs per segment
                            for seg in 0..num_segments {
                                let seg_pos = local_start + seg_len * (seg as f64 + 0.5);
                                let a_in = seg_pos * 12.0;
                                let b_in = l_in - a_in;
                                if x_in <= a_in {
                                    // x before load point
                                    defl += seg_load
                                        * b_in
                                        * x_in
                                        * (l_in * l_in - b_in * b_in - x_in * x_in)
                                        / (6.0 * ei * l_in);
                                } else {
                                    // x after load point
                                    defl += seg_load
                                        * a_in
                                        * (l_in - x_in)
                                        * (2.0 * l_in * x_in - x_in * x_in - a_in * a_in)
                                        / (6.0 * ei * l_in);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Add self-weight contribution
            if input.load_case.include_self_weight {
                let dead_factor = get_factor(LoadType::Dead);
                let sw = span.self_weight_plf() * dead_factor;
                v -= sw * x;
                m -= sw * x * x / 2.0;
                defl += uniform_load_deflection(sw / 12.0, l_in, x_in, e, i_val);
            }

            shear_diagram.push((span_start + x, v));
            moment_diagram.push((span_start + x, m));
            deflection_diagram.push((span_start + x, defl));

            span_max_shear = span_max_shear.max(v.abs());
            if m > span_max_pos_moment {
                span_max_pos_moment = m;
                span_max_pos_moment_x = x;
            }
            // Track maximum absolute deflection (handles negative from uplift)
            if defl.abs() > max_defl.abs() {
                max_defl = defl;
            }
        }

        // Track global extrema
        if span_max_pos_moment > max_positive_moment {
            max_positive_moment = span_max_pos_moment;
            max_positive_moment_loc = (i, span_max_pos_moment_x);
        }
        if m_left.abs() > max_negative_moment {
            max_negative_moment = m_left.abs();
            max_negative_node = i;
        }
        if m_right.abs() > max_negative_moment {
            max_negative_moment = m_right.abs();
            max_negative_node = i + 1;
        }
        // Track global max deflection by absolute value
        if max_defl.abs() > max_deflection.abs() {
            max_deflection = max_defl;
            max_deflection_loc = (i, l / 2.0);
        }
        if span_max_shear > max_shear {
            max_shear = span_max_shear;
            max_shear_loc = (i, 0.0);
        }

        // Calculate stresses and unity checks
        let s = span.section_modulus_in3();
        let area = span.area_in2();
        let props = span.material.base_properties()?;
        let factors = &input.adjustment_factors;

        let design_moment = span_max_pos_moment.max(m_left.abs()).max(m_right.abs());
        let max_moment_inlb = design_moment * 12.0;
        let actual_fb = max_moment_inlb / s;

        // Calculate size factor
        let c_f = if !span.material.is_engineered() {
            SizeFactor::new(span.depth_in, span.width_in).factor_fb()
        } else {
            1.0
        };

        // Calculate beam stability factor
        let c_l = if factors.compression_edge_braced {
            1.0
        } else {
            let le = factors.unbraced_length_in.unwrap_or(l_in);
            let stability = BeamStability::new(le, span.width_in, span.depth_in);
            if stability.is_fully_braced() {
                1.0
            } else {
                let fb_depth = span.material.fb_for_depth(span.depth_in)?;
                let fb_star = fb_depth
                    * factors.c_d()
                    * factors.c_m_fb()
                    * factors.c_t()
                    * c_f
                    * factors.c_fu(span.width_in)
                    * factors.c_i_strength()
                    * factors.c_r();
                let e_min_prime = factors.adjusted_e_min(props.e_min_psi);
                stability.factor(fb_star, e_min_prime)
            }
        };

        let fb_depth = span.material.fb_for_depth(span.depth_in)?;
        let allowable_fb = factors.adjusted_fb(fb_depth, c_f, c_l, span.width_in);
        let bending_unity = actual_fb / allowable_fb;

        // Shear stress
        let actual_fv = 3.0 * span_max_shear / (2.0 * area);
        let allowable_fv = factors.adjusted_fv(props.fv_psi);
        let shear_unity = actual_fv / allowable_fv;

        // Deflection check (use absolute value for serviceability check)
        let deflection_limit = l_in / 240.0;
        let deflection_unity = max_defl.abs() / deflection_limit;

        // Track governing condition
        let span_governing = bending_unity.max(shear_unity).max(deflection_unity);
        if span_governing > governing_unity {
            governing_unity = span_governing;
            governing_span = i;
            governing_condition =
                if bending_unity >= shear_unity && bending_unity >= deflection_unity {
                    "Bending".to_string()
                } else if shear_unity >= deflection_unity {
                    "Shear".to_string()
                } else {
                    "Deflection".to_string()
                };
        }

        span_results.push(SpanResult {
            span_index: i,
            moment_left_ftlb: m_left,
            moment_right_ftlb: m_right,
            shear_left_lb: v_left,
            shear_right_lb: v_right,
            max_positive_moment_ftlb: span_max_pos_moment,
            max_positive_moment_pos_ft: span_max_pos_moment_x,
            max_negative_moment_ftlb: m_left.abs().max(m_right.abs()),
            max_shear_lb: span_max_shear,
            max_deflection_in: max_defl,
            max_deflection_pos_ft: l / 2.0,
            actual_fb_psi: actual_fb,
            allowable_fb_psi: allowable_fb,
            bending_unity,
            actual_fv_psi: actual_fv,
            allowable_fv_psi: allowable_fv,
            shear_unity,
            deflection_unity,
        });
    }

    Ok(ContinuousBeamResult {
        span_results,
        reactions,
        support_moments: dist_result.support_moments.clone(),
        rotations: vec![0.0; n_nodes], // Placeholder - could compute from moment distribution
        max_positive_moment_ftlb: max_positive_moment,
        max_positive_moment_location: max_positive_moment_loc,
        max_negative_moment_ftlb: max_negative_moment,
        max_negative_moment_node: max_negative_node,
        max_shear_lb: max_shear,
        max_shear_location: max_shear_loc,
        max_deflection_in: max_deflection,
        max_deflection_location: max_deflection_loc,
        governing_unity,
        governing_span,
        governing_condition,
        shear_diagram,
        moment_diagram,
        deflection_diagram,
        governing_combination: combo_name.to_string(),
        min_reaction_combination: String::new(),
        min_reactions: vec![],
    })
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loads::{DiscreteLoad, LoadType};
    use crate::materials::{WoodGrade, WoodMaterial, WoodSpecies};

    fn test_material() -> Material {
        Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ))
    }

    #[test]
    fn test_support_type_display() {
        assert_eq!(SupportType::Pinned.display_name(), "Pinned");
        assert_eq!(SupportType::Fixed.display_name(), "Fixed");
        assert_eq!(SupportType::Free.display_name(), "Free");
        assert_eq!(SupportType::Roller.display_name(), "Roller");
    }

    #[test]
    fn test_support_type_restraints() {
        assert!(SupportType::Pinned.restrains_vertical());
        assert!(!SupportType::Pinned.restrains_rotation());

        assert!(SupportType::Fixed.restrains_vertical());
        assert!(SupportType::Fixed.restrains_rotation());

        assert!(!SupportType::Free.restrains_vertical());
        assert!(!SupportType::Free.restrains_rotation());
    }

    #[test]
    fn test_span_segment_properties() {
        let span = SpanSegment::new(12.0, 1.5, 9.25, test_material());

        // I = bd³/12 = 1.5 * 9.25³ / 12 ≈ 98.93
        assert!((span.moment_of_inertia_in4() - 98.93).abs() < 0.1);

        // S = bd²/6 = 1.5 * 9.25² / 6 ≈ 21.39
        assert!((span.section_modulus_in3() - 21.39).abs() < 0.1);

        // A = bd = 1.5 * 9.25 = 13.875
        assert!((span.area_in2() - 13.875).abs() < 0.01);
    }

    #[test]
    fn test_continuous_beam_simple_span() {
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let beam =
            ContinuousBeamInput::simple_span("B-1", 12.0, 1.5, 9.25, test_material(), load_case);

        assert_eq!(beam.span_count(), 1);
        assert_eq!(beam.node_count(), 2);
        assert!(beam.is_single_span());
        assert!(beam.is_simply_supported());
        assert!(!beam.is_indeterminate());
        assert!(beam.validate().is_ok());
    }

    #[test]
    fn test_continuous_beam_cantilever() {
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let beam =
            ContinuousBeamInput::cantilever("CB-1", 8.0, 1.5, 9.25, test_material(), load_case);

        assert!(beam.is_cantilever());
        assert!(beam.is_indeterminate()); // Fixed support makes it indeterminate
        assert!(beam.validate().is_ok());
    }

    #[test]
    fn test_continuous_beam_fixed_fixed() {
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let beam =
            ContinuousBeamInput::fixed_fixed("FB-1", 10.0, 1.5, 9.25, test_material(), load_case);

        assert!(beam.is_fixed_fixed());
        assert!(beam.is_indeterminate());
        assert!(beam.validate().is_ok());
    }

    #[test]
    fn test_continuous_beam_two_span() {
        let beam = ContinuousBeamInput {
            label: "Two-Span".to_string(),
            spans: vec![
                SpanSegment::new(12.0, 1.5, 9.25, test_material()),
                SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![
                SupportType::Pinned,
                SupportType::Pinned,
                SupportType::Roller,
            ],
            ..Default::default()
        };

        assert_eq!(beam.span_count(), 2);
        assert_eq!(beam.node_count(), 3);
        assert_eq!(beam.total_length_ft(), 22.0);
        assert!(beam.is_indeterminate());
        assert!(beam.validate().is_ok());

        let positions = beam.node_positions();
        assert_eq!(positions, vec![0.0, 12.0, 22.0]);
    }

    #[test]
    fn test_validation_no_spans() {
        let beam = ContinuousBeamInput {
            spans: vec![],
            supports: vec![SupportType::Pinned],
            ..Default::default()
        };

        assert!(beam.validate().is_err());
    }

    #[test]
    fn test_validation_wrong_support_count() {
        let beam = ContinuousBeamInput {
            spans: vec![SpanSegment::default()],
            supports: vec![SupportType::Pinned], // Should be 2
            ..Default::default()
        };

        assert!(beam.validate().is_err());
    }

    #[test]
    fn test_validation_no_vertical_support() {
        let beam = ContinuousBeamInput {
            spans: vec![SpanSegment::default()],
            supports: vec![SupportType::Free, SupportType::Free],
            ..Default::default()
        };

        assert!(beam.validate().is_err());
    }

    #[test]
    fn test_add_remove_span() {
        let mut beam = ContinuousBeamInput::default();
        assert_eq!(beam.span_count(), 1);

        beam.add_span(SpanSegment::new(10.0, 1.5, 9.25, test_material()));
        assert_eq!(beam.span_count(), 2);
        assert_eq!(beam.supports.len(), 3);

        let removed = beam.remove_last_span();
        assert!(removed.is_some());
        assert_eq!(beam.span_count(), 1);
        assert_eq!(beam.supports.len(), 2);

        // Can't remove the only span
        let removed = beam.remove_last_span();
        assert!(removed.is_none());
        assert_eq!(beam.span_count(), 1);
    }

    #[test]
    fn test_serialization() {
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0));

        let beam = ContinuousBeamInput::simple_span(
            "Test Beam",
            12.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let json = serde_json::to_string_pretty(&beam).unwrap();
        let parsed: ContinuousBeamInput = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.label, beam.label);
        assert_eq!(parsed.span_count(), beam.span_count());
        assert_eq!(parsed.supports.len(), beam.supports.len());
    }

    #[test]
    fn test_wind_point_load_on_span2() {
        // Bug repro: Wind point load on span 2+ should produce valid diagrams
        // Two-span beam: 10' + 10', wind point load at 15' (middle of span 2)
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("Wind Point on Span 2")
            .with_load(DiscreteLoad::point(LoadType::Wind, 1000.0, 15.0));

        let input = ContinuousBeamInput {
            label: "Two-span wind test".to_string(),
            spans: vec![
                SpanSegment::new(10.0, 1.5, 9.25, test_material()),
                SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![
                SupportType::Pinned,
                SupportType::Pinned,
                SupportType::Pinned,
            ],
            load_case,
            ..Default::default()
        };

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        // Check reactions are reasonable
        println!("Governing combo: {}", result.governing_combination);
        println!("Reactions: {:?}", result.reactions);
        println!("Max shear: {}", result.max_shear_lb);
        println!("Max moment: {}", result.max_positive_moment_ftlb);
        println!("Max deflection: {}", result.max_deflection_in);

        // The total reactions should equal the applied load
        let total_reaction: f64 = result.reactions.iter().sum();
        println!(
            "Total reaction: {} (expected ~1000 lb with factors)",
            total_reaction
        );

        // Check deflection diagram values
        println!("\nSpan 1 deflections (first 5):");
        for (pos, d) in result.deflection_diagram.iter().take(5) {
            println!("  pos={:.2}, defl={:.6}", pos, d);
        }

        println!("\nSpan 2 deflections (positions >= 10):");
        let span2_deflections: Vec<_> = result
            .deflection_diagram
            .iter()
            .filter(|(pos, _)| *pos >= 10.0)
            .take(10)
            .collect();
        for (pos, d) in &span2_deflections {
            println!("  pos={:.2}, defl={:.6}", pos, d);
        }

        // Check for max absolute deflection
        let max_abs_defl = result
            .deflection_diagram
            .iter()
            .map(|(_, d)| d.abs())
            .fold(0.0f64, |a, b| a.max(b));
        println!("\nMax absolute deflection: {}", max_abs_defl);

        // Check shear diagram has reasonable values (not NaN, not extreme)
        for (pos, v) in &result.shear_diagram {
            assert!(!v.is_nan(), "Shear diagram has NaN at position {}", pos);
            assert!(
                v.abs() < 10000.0,
                "Shear {} at pos {} seems too large",
                v,
                pos
            );
        }

        // Check moment diagram has reasonable values
        for (pos, m) in &result.moment_diagram {
            assert!(!m.is_nan(), "Moment diagram has NaN at position {}", pos);
            assert!(
                m.abs() < 100000.0,
                "Moment {} at pos {} seems too large",
                m,
                pos
            );
        }

        // Check deflection diagram has reasonable values (not NaN, not extreme)
        for (pos, d) in &result.deflection_diagram {
            assert!(
                !d.is_nan(),
                "Deflection diagram has NaN at position {}",
                pos
            );
            assert!(
                d.abs() < 100.0,
                "Deflection {} at pos {} seems too large",
                d,
                pos
            );
        }

        // Maximum absolute deflection should be non-zero
        assert!(
            max_abs_defl > 0.0,
            "Should have non-zero deflection magnitude"
        );
    }

    #[test]
    fn test_dead_plus_wind_point_load_on_span2() {
        // More realistic case: dead load + wind point load on span 2
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("D + W Point on Span 2")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0)) // 50 plf dead
            .with_load(DiscreteLoad::point(LoadType::Wind, 1000.0, 15.0)); // 1000 lb wind at 15'

        let input = ContinuousBeamInput {
            label: "Two-span D+W test".to_string(),
            spans: vec![
                SpanSegment::new(10.0, 1.5, 9.25, test_material()),
                SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![
                SupportType::Pinned,
                SupportType::Pinned,
                SupportType::Pinned,
            ],
            load_case,
            ..Default::default()
        };

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        println!("Governing combo: {}", result.governing_combination);
        println!("Reactions: {:?}", result.reactions);
        println!("Max shear: {}", result.max_shear_lb);
        println!("Max moment: {}", result.max_positive_moment_ftlb);
        println!("Max deflection: {}", result.max_deflection_in);

        // Print span 2 deflection sample
        println!("\nSpan 2 deflections (positions >= 10):");
        let span2_deflections: Vec<_> = result
            .deflection_diagram
            .iter()
            .filter(|(pos, _)| *pos >= 10.0)
            .take(10)
            .collect();
        for (pos, d) in &span2_deflections {
            println!("  pos={:.2}, defl={:.6}", pos, d);
        }

        // Total reactions should be positive (dead load > wind uplift in most combos)
        let total_reaction: f64 = result.reactions.iter().sum();
        println!("\nTotal reaction: {}", total_reaction);

        // Check for max absolute deflection
        let max_abs_defl = result
            .deflection_diagram
            .iter()
            .map(|(_, d)| d.abs())
            .fold(0.0f64, |a, b| a.max(b));
        println!("Max absolute deflection: {}", max_abs_defl);

        // Deflections should be reasonable
        for (pos, d) in &result.deflection_diagram {
            assert!(
                !d.is_nan(),
                "Deflection diagram has NaN at position {}",
                pos
            );
        }

        assert!(max_abs_defl > 0.0, "Should have non-zero deflection");
    }

    #[test]
    fn test_partial_uniform_load_alone() {
        // Verify partial uniform load calculates correctly
        // 12 ft beam with 100 plf load from 3 ft to 9 ft (6 ft loaded length)
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("Partial Dead")
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Dead,
                100.0,
                3.0,
                9.0,
            ))
            .without_self_weight();

        let input = ContinuousBeamInput::simple_span(
            "Partial Uniform Test",
            12.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        // Total load = 100 plf * 6 ft = 600 lb
        // Centroid at 6 ft (symmetric about midspan)
        // R1 = R2 = 300 lb each
        let total_reaction: f64 = result.reactions.iter().sum();
        assert!(
            (total_reaction - 600.0).abs() < 1.0,
            "Total reaction {} should be ~600 lb",
            total_reaction
        );

        // Reactions should be equal for symmetric load
        assert!(
            (result.reactions[0] - result.reactions[1]).abs() < 1.0,
            "Reactions should be equal for symmetric partial load"
        );

        // Check diagrams have valid values
        for (pos, v) in &result.shear_diagram {
            assert!(!v.is_nan(), "Shear NaN at pos {}", pos);
        }
        for (pos, m) in &result.moment_diagram {
            assert!(!m.is_nan(), "Moment NaN at pos {}", pos);
        }
        for (pos, d) in &result.deflection_diagram {
            assert!(!d.is_nan(), "Deflection NaN at pos {}", pos);
        }

        // Max moment should be positive (load applied)
        assert!(
            result.max_positive_moment_ftlb > 0.0,
            "Should have positive moment"
        );

        // Check deflection diagram has non-zero values
        let max_abs_defl = result
            .deflection_diagram
            .iter()
            .map(|(_, d)| d.abs())
            .fold(0.0f64, f64::max);
        assert!(max_abs_defl > 0.0, "Should have non-zero deflection");
    }

    #[test]
    fn test_partial_uniform_plus_full_uniform() {
        // Partial uniform (dead) + full uniform (live)
        // 12 ft beam with:
        // - 50 plf dead from 0-6 ft (partial)
        // - 40 plf live over full span
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("D partial + L full")
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Dead,
                50.0,
                0.0,
                6.0,
            ))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0))
            .without_self_weight();

        let input = ContinuousBeamInput::simple_span(
            "Partial + Full Test",
            12.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        // Partial dead: 50 plf * 6 ft = 300 lb
        // Full live: 40 plf * 12 ft = 480 lb
        // Total = 780 lb (for D+L combination)
        let total_reaction: f64 = result.reactions.iter().sum();
        assert!(
            (total_reaction - 780.0).abs() < 1.0,
            "Total reaction {} should be ~780 lb for D+L",
            total_reaction
        );

        // Reactions should NOT be equal (asymmetric loading)
        assert!(
            (result.reactions[0] - result.reactions[1]).abs() > 10.0,
            "Reactions should differ for asymmetric load"
        );

        // Check diagrams are valid
        for (pos, v) in &result.shear_diagram {
            assert!(!v.is_nan(), "Shear NaN at pos {}", pos);
        }
        for (pos, m) in &result.moment_diagram {
            assert!(!m.is_nan(), "Moment NaN at pos {}", pos);
        }
    }

    #[test]
    fn test_partial_uniform_plus_point_load() {
        // Partial uniform + point load
        // 12 ft beam with:
        // - 60 plf dead from 2-8 ft (partial, centroid at 5 ft)
        // - 500 lb live point load at midspan (6 ft)
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("D partial + L point")
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Dead,
                60.0,
                2.0,
                8.0,
            ))
            .with_load(DiscreteLoad::point(LoadType::Live, 500.0, 6.0))
            .without_self_weight();

        let input = ContinuousBeamInput::simple_span(
            "Partial + Point Test",
            12.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        // Partial dead: 60 plf * 6 ft = 360 lb (centroid at 5 ft)
        // Point live: 500 lb at 6 ft
        // Total for D+L = 860 lb
        let total_reaction: f64 = result.reactions.iter().sum();
        assert!(
            (total_reaction - 860.0).abs() < 1.0,
            "Total reaction {} should be ~860 lb for D+L",
            total_reaction
        );

        // Reactions should sum correctly
        // - Partial load centroid at 5 ft: R1 gets more
        // - Point load at 6 ft (midspan): equal contribution
        // Both reactions should be positive and reasonable
        assert!(
            result.reactions[0] > 0.0,
            "Left reaction should be positive"
        );
        assert!(
            result.reactions[1] > 0.0,
            "Right reaction should be positive"
        );

        // Verify max moment occurs near midspan
        let (_, pos) = result.max_positive_moment_location;
        assert!(
            (pos - 6.0).abs() < 2.0,
            "Max moment at {} should be near midspan (6 ft)",
            pos
        );
    }

    #[test]
    fn test_multiple_partial_uniform_loads() {
        // Multiple partial uniform loads at different locations
        // 12 ft beam with:
        // - 50 plf dead from 0-4 ft
        // - 80 plf live from 4-8 ft
        // - 30 plf snow from 8-12 ft
        use super::calculate_continuous;
        use crate::loads::DesignMethod;

        let load_case = EnhancedLoadCase::new("Multiple partials")
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Dead,
                50.0,
                0.0,
                4.0,
            ))
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Live,
                80.0,
                4.0,
                8.0,
            ))
            .with_load(DiscreteLoad::partial_uniform(
                LoadType::Snow,
                30.0,
                8.0,
                12.0,
            ))
            .without_self_weight();

        let input = ContinuousBeamInput::simple_span(
            "Multiple Partials Test",
            12.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let result =
            calculate_continuous(&input, DesignMethod::Asd).expect("Calculation should succeed");

        // Dead: 50 * 4 = 200 lb
        // Live: 80 * 4 = 320 lb
        // Snow: 30 * 4 = 120 lb
        // D + L combination = 200 + 320 = 520 lb
        // D + S combination = 200 + 120 = 320 lb
        // The governing combo should have the highest factored load
        let total_reaction: f64 = result.reactions.iter().sum();

        // Should be D + L = 520 lb (higher than D + S)
        assert!(
            total_reaction > 300.0 && total_reaction < 700.0,
            "Total reaction {} should be reasonable",
            total_reaction
        );

        // All diagrams should be valid
        for (pos, v) in &result.shear_diagram {
            assert!(!v.is_nan(), "Shear NaN at pos {}", pos);
        }
        for (pos, m) in &result.moment_diagram {
            assert!(!m.is_nan(), "Moment NaN at pos {}", pos);
            assert!(
                m.abs() < 10000.0,
                "Moment {} at pos {} seems too large",
                m,
                pos
            );
        }
        for (pos, d) in &result.deflection_diagram {
            assert!(!d.is_nan(), "Deflection NaN at pos {}", pos);
        }
    }
}
