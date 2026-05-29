//! # Cross-Section Property Formulas
//!
//! Formulas for calculating geometric properties of structural cross-sections.
//! These properties are used in stress and deflection calculations.
//!
//! ## Notation
//!
//! - `A` = Cross-sectional area
//! - `I` = Moment of inertia (second moment of area)
//! - `S` = Section modulus (I/c, where c = distance to extreme fiber)
//! - `r` = Radius of gyration (√(I/A))
//! - `b` = Width of section
//! - `d` = Depth (height) of section
//!
//! ## References
//!
//! - NDS 2018 Section 3.3: Bending Members - Section Properties
//! - AISC Steel Construction Manual, Part 1: Dimensions and Properties
//! - Roark's Formulas for Stress and Strain, 8th Edition, Chapter 3

// =============================================================================
// RECTANGULAR SECTION PROPERTIES
// Solid rectangular cross-section (common for sawn lumber)
// =============================================================================

/// Calculate cross-sectional area for rectangular section
///
/// ```text
///     ┌─────────┐
///     │         │
///   d │         │
///     │         │
///     └─────────┘
///          b
/// ```
///
/// # Formula
/// A = b × d
///
/// # Arguments
/// * `b` - Width of section
/// * `d` - Depth (height) of section
///
/// # Returns
/// Cross-sectional area in square units of input
///
/// # Example
/// ```rust
/// use calc_core::equations::section::rectangular_area;
///
/// // 2x10 nominal (1.5" x 9.25" actual)
/// let area = rectangular_area(1.5, 9.25);
/// assert!((area - 13.875).abs() < 0.001);
/// ```
#[inline]
pub fn rectangular_area(b: f64, d: f64) -> f64 {
    b * d
}

/// Calculate moment of inertia for rectangular section about centroidal axis
///
/// The moment of inertia (second moment of area) measures the section's
/// resistance to bending. For a rectangle bending about its strong axis:
///
/// ```text
///     ┌─────────┐
///     │         │
///   d │ ════════│ ← neutral axis at d/2
///     │         │
///     └─────────┘
///          b
/// ```
///
/// # Formula (Strong Axis Bending)
/// I = bd³/12
///
/// For weak axis bending (section rotated 90°):
/// I = db³/12
///
/// # Arguments
/// * `b` - Width of section (perpendicular to bending)
/// * `d` - Depth of section (parallel to bending)
///
/// # Returns
/// Moment of inertia in fourth power of input units (e.g., in⁴)
///
/// # Example
/// ```rust
/// use calc_core::equations::section::rectangular_moment_of_inertia;
///
/// // 2x10 nominal (1.5" x 9.25" actual)
/// let i = rectangular_moment_of_inertia(1.5, 9.25);
/// // I = 1.5 × 9.25³ / 12 = 98.93 in⁴
/// assert!((i - 98.93).abs() < 0.01);
/// ```
///
/// # Reference
/// - Roark's Formulas, Table 3.1
/// - Any structural mechanics textbook
#[inline]
pub fn rectangular_moment_of_inertia(b: f64, d: f64) -> f64 {
    b * d.powi(3) / 12.0
}

/// Calculate section modulus for rectangular section
///
/// The section modulus relates bending moment to maximum bending stress:
/// σ = M/S
///
/// ```text
///     ┌─────────┐
///     │ tension │ ← distance c = d/2 from neutral axis
///   d │ ════════│ ← neutral axis
///     │ compression │
///     └─────────┘
///          b
/// ```
///
/// # Formula
/// S = I/c = bd³/12 ÷ d/2 = bd²/6
///
/// For rectangular sections, S_top = S_bottom (symmetric section).
///
/// # Arguments
/// * `b` - Width of section
/// * `d` - Depth of section
///
/// # Returns
/// Section modulus in cubic units of input (e.g., in³)
///
/// # Example
/// ```rust
/// use calc_core::equations::section::rectangular_section_modulus;
///
/// // 2x10 nominal (1.5" x 9.25" actual)
/// let s = rectangular_section_modulus(1.5, 9.25);
/// // S = 1.5 × 9.25² / 6 = 21.39 in³
/// assert!((s - 21.39).abs() < 0.01);
/// ```
///
/// # Reference
/// - NDS 2018 Section 3.3.3
/// - σ_b = M/S_x (bending stress equation)
#[inline]
pub fn rectangular_section_modulus(b: f64, d: f64) -> f64 {
    b * d.powi(2) / 6.0
}

/// Calculate radius of gyration for rectangular section
///
/// The radius of gyration is used in column buckling calculations
/// (slenderness ratio = L/r).
///
/// # Formula
/// r = √(I/A) = √(bd³/12 / bd) = √(d²/12) = d/√12 ≈ 0.289d
///
/// # Arguments
/// * `d` - Depth of section (about bending axis)
///
/// # Returns
/// Radius of gyration in same units as input
///
/// # Example
/// ```rust
/// use calc_core::equations::section::rectangular_radius_of_gyration;
///
/// // 2x10 nominal (9.25" depth)
/// let r = rectangular_radius_of_gyration(9.25);
/// // r = 9.25 / √12 = 2.67 in
/// assert!((r - 2.67).abs() < 0.01);
/// ```
///
/// # Reference
/// - NDS 2018 Section 3.7.1: Column Stability Factor
#[inline]
pub fn rectangular_radius_of_gyration(d: f64) -> f64 {
    d / (12.0_f64).sqrt()
}

// =============================================================================
// SHEAR AREA CALCULATIONS
// =============================================================================

/// Calculate effective shear area for rectangular section
///
/// For a rectangular section, the maximum shear stress occurs at the
/// neutral axis and is 1.5× the average shear stress:
/// τ_max = 1.5 × V/A = V/A_shear
///
/// Therefore, the effective shear area is:
/// A_shear = 2A/3
///
/// # Formula
/// A_shear = 2bd/3
///
/// # Arguments
/// * `b` - Width of section
/// * `d` - Depth of section
///
/// # Returns
/// Effective shear area in square units of input
///
/// # Reference
/// - NDS 2018 Section 3.4.2: f_v = 3V/(2bd)
#[inline]
pub fn rectangular_shear_area(b: f64, d: f64) -> f64 {
    2.0 * b * d / 3.0
}

// =============================================================================
// NDS DIMENSIONAL LUMBER PROPERTIES
// =============================================================================

/// Get actual dimensions for nominal lumber size (inches)
///
/// Per NDS/industry standards, dressed lumber dimensions are smaller
/// than nominal dimensions.
///
/// # Returns
/// (actual_width, actual_depth) in inches, or None if not a standard size
///
/// # Standard Sizes (NDS Table 1B)
/// - 2x nominal: 1.5" actual width
/// - 4x nominal: 3.5" actual width
/// - 6x nominal: 5.5" actual width
/// - 8x nominal: 7.25" actual depth (for 2x8)
/// - 10x nominal: 9.25" actual depth (for 2x10)
/// - 12x nominal: 11.25" actual depth (for 2x12)
///
/// # Example
/// ```rust
/// use calc_core::equations::section::nominal_to_actual_dimensions;
///
/// let (w, d) = nominal_to_actual_dimensions(2, 10).unwrap();
/// assert!((w - 1.5).abs() < 0.001);
/// assert!((d - 9.25).abs() < 0.001);
/// ```
pub fn nominal_to_actual_dimensions(nominal_width: u8, nominal_depth: u8) -> Option<(f64, f64)> {
    // Width conversion
    let actual_width = match nominal_width {
        2 => 1.5,
        3 => 2.5,
        4 => 3.5,
        6 => 5.5,
        8 => 7.25,
        10 => 9.25,
        12 => 11.25,
        _ => return None,
    };

    // Depth conversion
    let actual_depth = match nominal_depth {
        2 => 1.5,
        3 => 2.5,
        4 => 3.5,
        6 => 5.5,
        8 => 7.25,
        10 => 9.25,
        12 => 11.25,
        14 => 13.25,
        16 => 15.25,
        _ => return None,
    };

    Some((actual_width, actual_depth))
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 0.01;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON || (a - b).abs() / b.abs().max(1.0) < 0.001
    }

    #[test]
    fn test_rectangular_area() {
        // 2x10 (1.5" x 9.25")
        let a = rectangular_area(1.5, 9.25);
        assert!(approx_eq(a, 13.875), "A = {} (expected 13.875)", a);
    }

    #[test]
    fn test_rectangular_moment_of_inertia() {
        // 2x10 (1.5" x 9.25")
        // I = 1.5 * 9.25^3 / 12 = 98.932
        let i = rectangular_moment_of_inertia(1.5, 9.25);
        assert!(approx_eq(i, 98.93), "I = {} (expected 98.93)", i);
    }

    #[test]
    fn test_rectangular_section_modulus() {
        // 2x10 (1.5" x 9.25")
        // S = 1.5 * 9.25^2 / 6 = 21.391
        let s = rectangular_section_modulus(1.5, 9.25);
        assert!(approx_eq(s, 21.39), "S = {} (expected 21.39)", s);

        // Verify relationship: S = I / (d/2)
        let i = rectangular_moment_of_inertia(1.5, 9.25);
        let s_from_i = i / (9.25 / 2.0);
        assert!(approx_eq(s, s_from_i), "S = {}, I/(d/2) = {}", s, s_from_i);
    }

    #[test]
    fn test_rectangular_radius_of_gyration() {
        // r = d / sqrt(12)
        let r = rectangular_radius_of_gyration(9.25);
        let expected = 9.25 / (12.0_f64).sqrt(); // 2.67
        assert!(approx_eq(r, expected), "r = {} (expected {})", r, expected);
    }

    #[test]
    fn test_rectangular_shear_area() {
        // A_shear = 2bd/3
        let a_shear = rectangular_shear_area(1.5, 9.25);
        let expected = 2.0 * 1.5 * 9.25 / 3.0; // 9.25
        assert!(
            approx_eq(a_shear, expected),
            "A_shear = {} (expected {})",
            a_shear,
            expected
        );
    }

    #[test]
    fn test_nominal_to_actual_2x10() {
        let (w, d) = nominal_to_actual_dimensions(2, 10).unwrap();
        assert!(approx_eq(w, 1.5), "width = {} (expected 1.5)", w);
        assert!(approx_eq(d, 9.25), "depth = {} (expected 9.25)", d);
    }

    #[test]
    fn test_nominal_to_actual_4x12() {
        let (w, d) = nominal_to_actual_dimensions(4, 12).unwrap();
        assert!(approx_eq(w, 3.5), "width = {} (expected 3.5)", w);
        assert!(approx_eq(d, 11.25), "depth = {} (expected 11.25)", d);
    }

    #[test]
    fn test_nominal_to_actual_invalid() {
        assert!(nominal_to_actual_dimensions(5, 10).is_none());
        assert!(nominal_to_actual_dimensions(2, 7).is_none());
    }

    #[test]
    fn test_section_properties_consistency() {
        // For any rectangle: I = A * r^2
        // Let's verify: I = bd³/12, A = bd, r = d/√12
        // A * r² = bd * (d²/12) = bd³/12 = I ✓
        let b = 3.5;
        let d = 11.25;

        let i = rectangular_moment_of_inertia(b, d);
        let a = rectangular_area(b, d);
        let r = rectangular_radius_of_gyration(d);

        let i_from_ar = a * r * r;
        assert!(approx_eq(i, i_from_ar), "I = {}, A*r² = {}", i, i_from_ar);
    }
}
