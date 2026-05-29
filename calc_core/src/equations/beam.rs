//! # Simply-Supported Beam Formulas
//!
//! Fundamental equations for simply-supported beams under various loading conditions.
//! All formulas assume a beam with pin support at left (x=0) and roller at right (x=L).
//!
//! ## Notation
//!
//! - `L` = Span length
//! - `x` = Position along beam from left support
//! - `a` = Load position from left support
//! - `P` = Point load magnitude
//! - `w` = Uniform load intensity (force per unit length)
//! - `M` = Bending moment
//! - `V` = Shear force
//! - `δ` = Deflection
//! - `E` = Modulus of elasticity
//! - `I` = Moment of inertia
//! - `R1` = Left reaction, `R2` = Right reaction
//!
//! ## Sign Conventions
//!
//! - Loads: Positive downward
//! - Moment: Positive causes tension on bottom (sagging)
//! - Shear: Positive when left side up relative to right
//! - Deflection: Positive downward
//! - Reactions: Positive upward
//!
//! ## References
//!
//! - Roark's Formulas for Stress and Strain, 8th Edition, Table 8.1
//! - Structural Analysis by R.C. Hibbeler

// =============================================================================
// POINT LOAD FORMULAS
// Simply-supported beam with concentrated load P at distance 'a' from left
// =============================================================================

/// Calculate reactions for point load P at position a on span L
///
/// ```text
///        P
///        ↓
///    ────┬────────────
///    △   a            △
///   R1  ←───────L────→ R2
/// ```
///
/// # Formulas (Roark's Table 8.1, Case 1a)
/// - R1 = P(L-a)/L
/// - R2 = Pa/L
///
/// # Arguments
/// * `p` - Point load magnitude (positive downward)
/// * `a` - Distance from left support to load
/// * `l` - Span length
///
/// # Returns
/// (R1, R2) - Left and right reactions (positive upward)
#[inline]
pub fn point_load_reactions(p: f64, a: f64, l: f64) -> (f64, f64) {
    let r1 = p * (l - a) / l;
    let r2 = p * a / l;
    (r1, r2)
}

/// Calculate shear at position x for point load P at position a
///
/// # Formulas
/// - V(x) = R1           for x < a
/// - V(x) = R1 - P       for x ≥ a
///
/// where R1 = P(L-a)/L
#[inline]
pub fn point_load_shear(p: f64, a: f64, l: f64, x: f64) -> f64 {
    let (r1, _) = point_load_reactions(p, a, l);
    if x < a {
        r1
    } else {
        r1 - p
    }
}

/// Calculate moment at position x for point load P at position a
///
/// # Formulas (Roark's Table 8.1, Case 1a)
/// - M(x) = R1·x           for x ≤ a
/// - M(x) = R1·x - P(x-a)  for x > a
///
/// Maximum moment occurs at the load point:
/// - M_max = Pa(L-a)/L = R1·a = R2·(L-a)
#[inline]
pub fn point_load_moment(p: f64, a: f64, l: f64, x: f64) -> f64 {
    let (r1, _) = point_load_reactions(p, a, l);
    if x <= a {
        r1 * x
    } else {
        r1 * x - p * (x - a)
    }
}

/// Calculate deflection at position x for point load P at position a
///
/// # Formulas (Roark's Table 8.1, Case 1a)
///
/// For x ≤ a:
/// ```text
/// δ(x) = Pbx(L² - b² - x²) / (6EIL)
/// ```
///
/// For x > a:
/// ```text
/// δ(x) = Pa(L-x)(2Lx - x² - a²) / (6EIL)
/// ```
///
/// where b = L - a
///
/// # Arguments
/// * `p` - Point load (positive downward)
/// * `a` - Distance from left support to load
/// * `l` - Span length
/// * `x` - Position to calculate deflection
/// * `e` - Modulus of elasticity
/// * `i` - Moment of inertia
///
/// # Returns
/// Deflection (positive downward)
#[inline]
pub fn point_load_deflection(p: f64, a: f64, l: f64, x: f64, e: f64, i: f64) -> f64 {
    let b = l - a;
    let ei = e * i;

    if x <= a {
        // δ = Pbx(L² - b² - x²) / (6EIL)
        p * b * x * (l * l - b * b - x * x) / (6.0 * ei * l)
    } else {
        // δ = Pa(L-x)(2Lx - x² - a²) / (6EIL)
        p * a * (l - x) * (2.0 * l * x - x * x - a * a) / (6.0 * ei * l)
    }
}

/// Maximum deflection for point load at any position
///
/// For load at midspan (a = L/2):
/// ```text
/// δ_max = PL³ / (48EI)   at x = L/2
/// ```
///
/// For load not at midspan, max deflection is NOT at the load point.
/// Location of max deflection (Roark's):
/// ```text
/// x_max = √((L² - a²)/3)   for a < L/2
/// ```
#[inline]
pub fn point_load_max_deflection_midspan(p: f64, l: f64, e: f64, i: f64) -> f64 {
    p * l.powi(3) / (48.0 * e * i)
}

// =============================================================================
// UNIFORM LOAD FORMULAS
// Simply-supported beam with uniform load w over entire span
// =============================================================================

/// Calculate reactions for uniform load w over full span L
///
/// ```text
///    ↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓ w
///    ═════════════════
///    △                △
///   R1  ←─────L─────→ R2
/// ```
///
/// # Formula
/// R1 = R2 = wL/2
#[inline]
pub fn uniform_load_reactions(w: f64, l: f64) -> (f64, f64) {
    let r = w * l / 2.0;
    (r, r)
}

/// Calculate shear at position x for uniform load w over full span
///
/// # Formula
/// V(x) = wL/2 - wx = w(L/2 - x)
///
/// - At x=0: V = +wL/2 (positive, left side up)
/// - At x=L/2: V = 0
/// - At x=L: V = -wL/2 (negative, right side up)
#[inline]
pub fn uniform_load_shear(w: f64, l: f64, x: f64) -> f64 {
    w * (l / 2.0 - x)
}

/// Calculate moment at position x for uniform load w over full span
///
/// # Formula (Roark's Table 8.1, Case 2a)
/// M(x) = wx(L-x)/2
///
/// Maximum moment at midspan:
/// M_max = wL²/8  at x = L/2
#[inline]
pub fn uniform_load_moment(w: f64, l: f64, x: f64) -> f64 {
    w * x * (l - x) / 2.0
}

/// Maximum moment for uniform load
///
/// # Formula
/// M_max = wL²/8
#[inline]
pub fn uniform_load_max_moment(w: f64, l: f64) -> f64 {
    w * l * l / 8.0
}

/// Calculate deflection at position x for uniform load w
///
/// # Formula (Roark's Table 8.1, Case 2a)
/// δ(x) = wx(L³ - 2Lx² + x³) / (24EI)
///
/// # Note
/// Maximum deflection at midspan:
/// δ_max = 5wL⁴ / (384EI)
#[inline]
pub fn uniform_load_deflection(w: f64, l: f64, x: f64, e: f64, i: f64) -> f64 {
    w * x * (l.powi(3) - 2.0 * l * x * x + x.powi(3)) / (24.0 * e * i)
}

/// Maximum deflection for uniform load (at midspan)
///
/// # Formula
/// δ_max = 5wL⁴ / (384EI)
#[inline]
pub fn uniform_load_max_deflection(w: f64, l: f64, e: f64, i: f64) -> f64 {
    5.0 * w * l.powi(4) / (384.0 * e * i)
}

// =============================================================================
// PARTIAL UNIFORM LOAD FORMULAS
// Simply-supported beam with uniform load w from position a to b
// =============================================================================

/// Calculate reactions for partial uniform load w from a to b
///
/// ```text
///          ↓↓↓↓↓↓↓↓↓ w
///    ══════════════════
///    △     a     b     △
///   R1  ←─────L─────→ R2
/// ```
///
/// # Formulas
/// Total load W = w(b-a)
/// Centroid at c = (a+b)/2
/// - R1 = W(L-c)/L
/// - R2 = Wc/L
#[inline]
pub fn partial_uniform_reactions(w: f64, a: f64, b: f64, l: f64) -> (f64, f64) {
    let total_load = w * (b - a);
    let centroid = (a + b) / 2.0;
    let r1 = total_load * (l - centroid) / l;
    let r2 = total_load * centroid / l;
    (r1, r2)
}

/// Calculate shear at position x for partial uniform load
///
/// # Formulas
/// - V(x) = R1                     for x ≤ a
/// - V(x) = R1 - w(x-a)            for a < x < b
/// - V(x) = R1 - w(b-a) = -R2      for x ≥ b
#[inline]
pub fn partial_uniform_shear(w: f64, a: f64, b: f64, l: f64, x: f64) -> f64 {
    let (r1, _) = partial_uniform_reactions(w, a, b, l);

    if x <= a {
        r1
    } else if x >= b {
        r1 - w * (b - a)
    } else {
        r1 - w * (x - a)
    }
}

/// Calculate moment at position x for partial uniform load
///
/// # Formulas
/// - M(x) = R1·x                           for x ≤ a
/// - M(x) = R1·x - w(x-a)²/2               for a < x < b
/// - M(x) = R1·x - W(x-c)                  for x ≥ b
///   where W = w(b-a), c = (a+b)/2
#[inline]
pub fn partial_uniform_moment(w: f64, a: f64, b: f64, l: f64, x: f64) -> f64 {
    let (r1, _) = partial_uniform_reactions(w, a, b, l);

    if x <= a {
        r1 * x
    } else if x >= b {
        let total_load = w * (b - a);
        let centroid = (a + b) / 2.0;
        r1 * x - total_load * (x - centroid)
    } else {
        r1 * x - w * (x - a).powi(2) / 2.0
    }
}

// =============================================================================
// APPLIED MOMENT FORMULAS
// Simply-supported beam with applied moment M0 at position a
// =============================================================================

/// Calculate reactions for applied moment M0 at position a
///
/// # Formulas
/// - R1 = -M0/L (downward if M0 is counterclockwise)
/// - R2 = +M0/L (upward if M0 is counterclockwise)
///
/// Note: Moment creates a couple, no net vertical force
#[inline]
pub fn applied_moment_reactions(m0: f64, l: f64) -> (f64, f64) {
    let r1 = -m0 / l;
    let r2 = m0 / l;
    (r1, r2)
}

// =============================================================================
// FIXED-END MOMENT (FEM) FORMULAS
// Used for moment distribution in indeterminate beam analysis
// =============================================================================

/// Fixed-end moments for uniform load w over entire span L
///
/// ```text
///     ▣═══════════════════▣
///       ↓↓↓↓↓↓↓↓↓↓↓↓↓ w
///    -M_A ←─────L─────→ M_B
/// ```
///
/// # Formulas (Roark's Table 8.1, Case 2e)
/// - FEM_A = -wL²/12 (negative = counterclockwise at left)
/// - FEM_B = +wL²/12 (positive = clockwise at right)
///
/// # Returns
/// (M_A, M_B) in same units as w×L² (e.g., ft-lb if w is plf and L is ft)
#[inline]
pub fn fem_uniform_full(w: f64, l: f64) -> (f64, f64) {
    let fem = w * l * l / 12.0;
    (-fem, fem)
}

/// Fixed-end moments for point load P at distance 'a' from left
///
/// ```text
///     ▣════════P═════════▣
///              ↓
///    -M_A ←──a──┼──b──→ M_B
///         ←────────L──────→
/// ```
///
/// # Formulas (Roark's Table 8.1, Case 1e)
/// - FEM_A = -Pab²/L²
/// - FEM_B = +Pa²b/L²
///
/// where b = L - a
#[inline]
pub fn fem_point_load(p: f64, a: f64, l: f64) -> (f64, f64) {
    let b = l - a;
    let l2 = l * l;
    let fem_a = -p * a * b * b / l2;
    let fem_b = p * a * a * b / l2;
    (fem_a, fem_b)
}

/// Fixed-end moments for partial uniform load w from 'a' to 'b'
///
/// Uses numerical integration by dividing load into point loads
#[inline]
pub fn fem_partial_uniform(w: f64, start: f64, end: f64, l: f64) -> (f64, f64) {
    // Divide into 20 segments for accuracy
    let num_segments = 20;
    let segment_length = (end - start) / num_segments as f64;
    let segment_load = w * segment_length;

    let mut fem_a = 0.0;
    let mut fem_b = 0.0;

    for i in 0..num_segments {
        let pos = start + (i as f64 + 0.5) * segment_length;
        let (fa, fb) = fem_point_load(segment_load, pos, l);
        fem_a += fa;
        fem_b += fb;
    }

    (fem_a, fem_b)
}

// =============================================================================
// FIXED-FIXED BEAM FORMULAS
// Beam fixed at both ends
// =============================================================================

/// Reactions for fixed-fixed beam with uniform load
///
/// # Formula
/// R_A = R_B = wL/2 (same as simply-supported for symmetric load)
#[inline]
pub fn fixed_fixed_uniform_reactions(w: f64, l: f64) -> (f64, f64) {
    let r = w * l / 2.0;
    (r, r)
}

/// End moments for fixed-fixed beam with uniform load
///
/// # Formulas (Roark's Table 8.1, Case 2e)
/// - M_A = M_B = wL²/12 (magnitude, signs are negative = hogging)
///
/// # Returns
/// (M_A, M_B) both positive values representing hogging moment magnitude
#[inline]
pub fn fixed_fixed_uniform_end_moments(w: f64, l: f64) -> (f64, f64) {
    let m = w * l * l / 12.0;
    (m, m)
}

/// Maximum positive moment for fixed-fixed beam with uniform load
///
/// # Formula
/// M_max = wL²/24 at midspan (sagging)
#[inline]
pub fn fixed_fixed_uniform_max_positive_moment(w: f64, l: f64) -> f64 {
    w * l * l / 24.0
}

/// Moment at position x for fixed-fixed beam with uniform load
///
/// # Formula
/// M(x) = -wL²/12 + wLx/2 - wx²/2
///      = w(6Lx - L² - 6x²)/12
#[inline]
pub fn fixed_fixed_uniform_moment(w: f64, l: f64, x: f64) -> f64 {
    w * (6.0 * l * x - l * l - 6.0 * x * x) / 12.0
}

/// Shear at position x for fixed-fixed beam with uniform load
///
/// # Formula
/// V(x) = wL/2 - wx (same as simply-supported for symmetric load)
#[inline]
pub fn fixed_fixed_uniform_shear(w: f64, l: f64, x: f64) -> f64 {
    w * (l / 2.0 - x)
}

/// Deflection at position x for fixed-fixed beam with uniform load
///
/// # Formula (Roark's Table 8.1, Case 2e)
/// δ(x) = wx²(L-x)² / (24EI)
#[inline]
pub fn fixed_fixed_uniform_deflection(w: f64, l: f64, x: f64, e: f64, i: f64) -> f64 {
    w * x * x * (l - x) * (l - x) / (24.0 * e * i)
}

/// Maximum deflection for fixed-fixed beam with uniform load
///
/// # Formula
/// δ_max = wL⁴/(384EI) at midspan
#[inline]
pub fn fixed_fixed_uniform_max_deflection(w: f64, l: f64, e: f64, i: f64) -> f64 {
    w * l.powi(4) / (384.0 * e * i)
}

/// Reactions for fixed-fixed beam with point load at position a
///
/// # Formulas (Roark's Table 8.1, Case 1e)
/// R_A = Pb²(3a + b)/L³
/// R_B = Pa²(a + 3b)/L³
/// where b = L - a
#[inline]
pub fn fixed_fixed_point_reactions(p: f64, a: f64, l: f64) -> (f64, f64) {
    let b = l - a;
    let l3 = l * l * l;
    let r_a = p * b * b * (3.0 * a + b) / l3;
    let r_b = p * a * a * (a + 3.0 * b) / l3;
    (r_a, r_b)
}

/// End moments for fixed-fixed beam with point load at position a
///
/// # Formulas (Roark's Table 8.1, Case 1e)
/// M_A = Pab²/L² (hogging at left support)
/// M_B = Pa²b/L² (hogging at right support)
#[inline]
pub fn fixed_fixed_point_end_moments(p: f64, a: f64, l: f64) -> (f64, f64) {
    let b = l - a;
    let l2 = l * l;
    let m_a = p * a * b * b / l2;
    let m_b = p * a * a * b / l2;
    (m_a, m_b)
}

// =============================================================================
// CANTILEVER BEAM FORMULAS
// Fixed at one end (A), free at other (B)
// =============================================================================

/// Reaction and moment at fixed end for cantilever with uniform load
///
/// # Formulas
/// R_A = wL (total load)
/// M_A = wL²/2 (hogging moment at support)
///
/// # Returns
/// (R_A, M_A)
#[inline]
pub fn cantilever_uniform_reactions(w: f64, l: f64) -> (f64, f64) {
    let r = w * l;
    let m = w * l * l / 2.0;
    (r, m)
}

/// Shear at position x for cantilever with uniform load (fixed at left)
///
/// # Formula
/// V(x) = w(L - x)
#[inline]
pub fn cantilever_uniform_shear(w: f64, l: f64, x: f64) -> f64 {
    w * (l - x)
}

/// Moment at position x for cantilever with uniform load (fixed at left)
///
/// # Formula
/// M(x) = -w(L-x)²/2
///
/// Negative because causes tension on top (hogging).
/// At x=0: M = -wL²/2
/// At x=L: M = 0
#[inline]
pub fn cantilever_uniform_moment(w: f64, l: f64, x: f64) -> f64 {
    -w * (l - x) * (l - x) / 2.0
}

/// Deflection at position x for cantilever with uniform load (fixed at left)
///
/// # Formula (Roark's Table 8.1, Case 2b)
/// δ(x) = wx²(6L² - 4Lx + x²) / (24EI)
#[inline]
pub fn cantilever_uniform_deflection(w: f64, l: f64, x: f64, e: f64, i: f64) -> f64 {
    w * x * x * (6.0 * l * l - 4.0 * l * x + x * x) / (24.0 * e * i)
}

/// Maximum deflection for cantilever with uniform load (at free end)
///
/// # Formula
/// δ_max = wL⁴/(8EI)
#[inline]
pub fn cantilever_uniform_max_deflection(w: f64, l: f64, e: f64, i: f64) -> f64 {
    w * l.powi(4) / (8.0 * e * i)
}

/// Reaction and moment at fixed end for cantilever with point load at distance a
///
/// # Formulas
/// R_A = P
/// M_A = Pa (moment = force × distance from support)
///
/// # Returns
/// (R_A, M_A)
#[inline]
pub fn cantilever_point_reactions(p: f64, a: f64) -> (f64, f64) {
    (p, p * a)
}

/// Moment at position x for cantilever with point load at distance a (fixed at left)
///
/// # Formula
/// M(x) = -P(a-x) for x < a
/// M(x) = 0       for x >= a
#[inline]
pub fn cantilever_point_moment(p: f64, a: f64, x: f64) -> f64 {
    if x < a {
        -p * (a - x)
    } else {
        0.0
    }
}

/// Deflection at position x for cantilever with point load at distance a
///
/// # Formulas (Roark's Table 8.1, Case 1b)
/// For x ≤ a: δ(x) = Px²(3a - x) / (6EI)
/// For x > a: δ(x) = Pa²(3x - a) / (6EI)
#[inline]
pub fn cantilever_point_deflection(p: f64, a: f64, x: f64, e: f64, i: f64) -> f64 {
    if x <= a {
        p * x * x * (3.0 * a - x) / (6.0 * e * i)
    } else {
        p * a * a * (3.0 * x - a) / (6.0 * e * i)
    }
}

// =============================================================================
// PROPPED CANTILEVER (FIXED-PINNED) FORMULAS
// Fixed at left (A), pinned at right (B)
// =============================================================================

/// Reactions for propped cantilever with uniform load
///
/// # Formulas (Roark's Table 8.1)
/// R_A = 5wL/8
/// R_B = 3wL/8
/// M_A = wL²/8 (fixed end moment)
#[inline]
pub fn fixed_pinned_uniform_reactions(w: f64, l: f64) -> (f64, f64, f64) {
    let r_a = 5.0 * w * l / 8.0;
    let r_b = 3.0 * w * l / 8.0;
    let m_a = w * l * l / 8.0;
    (r_a, r_b, m_a)
}

/// Moment at position x for propped cantilever with uniform load
///
/// # Formula
/// M(x) = R_A·x - M_A - wx²/2
///      = 5wLx/8 - wL²/8 - wx²/2
#[inline]
pub fn fixed_pinned_uniform_moment(w: f64, l: f64, x: f64) -> f64 {
    5.0 * w * l * x / 8.0 - w * l * l / 8.0 - w * x * x / 2.0
}

/// Maximum positive moment for propped cantilever with uniform load
///
/// Occurs at x = 5L/8 - sqrt(x²) where V=0
/// x_max = 5L/8
/// M_max = 9wL²/128
#[inline]
pub fn fixed_pinned_uniform_max_positive_moment(w: f64, l: f64) -> f64 {
    9.0 * w * l * l / 128.0
}

/// Position of max positive moment for propped cantilever
#[inline]
pub fn fixed_pinned_uniform_max_moment_position(l: f64) -> f64 {
    3.0 * l / 8.0 // Where shear = 0
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON || (a - b).abs() / b.abs().max(1.0) < 0.001
    }

    fn approx_eq_tolerance(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol || (a - b).abs() / b.abs().max(1.0) < tol
    }

    // Point load tests
    #[test]
    fn test_point_load_midspan_reactions() {
        // 10 ft beam, 1000 lb at midspan
        let (r1, r2) = point_load_reactions(1000.0, 5.0, 10.0);
        assert!(approx_eq(r1, 500.0), "R1 = {}", r1);
        assert!(approx_eq(r2, 500.0), "R2 = {}", r2);
    }

    #[test]
    fn test_point_load_asymmetric_reactions() {
        // 10 ft beam, 1000 lb at 3 ft from left
        let (r1, r2) = point_load_reactions(1000.0, 3.0, 10.0);
        assert!(approx_eq(r1, 700.0), "R1 = {} (expected 700)", r1);
        assert!(approx_eq(r2, 300.0), "R2 = {} (expected 300)", r2);
    }

    #[test]
    fn test_point_load_moment_max() {
        // 10 ft beam, 1000 lb at midspan
        // M_max = PL/4 = 1000 * 10 / 4 = 2500 ft-lb
        let m = point_load_moment(1000.0, 5.0, 10.0, 5.0);
        assert!(approx_eq(m, 2500.0), "M = {} (expected 2500)", m);
    }

    #[test]
    fn test_point_load_moment_zero_at_supports() {
        let m0 = point_load_moment(1000.0, 5.0, 10.0, 0.0);
        let ml = point_load_moment(1000.0, 5.0, 10.0, 10.0);
        assert!(approx_eq(m0, 0.0), "M(0) = {} (expected 0)", m0);
        assert!(approx_eq(ml, 0.0), "M(L) = {} (expected 0)", ml);
    }

    // Uniform load tests
    #[test]
    fn test_uniform_load_reactions() {
        // 10 ft beam, 100 plf
        let (r1, r2) = uniform_load_reactions(100.0, 10.0);
        assert!(approx_eq(r1, 500.0), "R1 = {}", r1);
        assert!(approx_eq(r2, 500.0), "R2 = {}", r2);
    }

    #[test]
    fn test_uniform_load_max_moment() {
        // 10 ft beam, 100 plf
        // M_max = wL²/8 = 100 * 100 / 8 = 1250 ft-lb
        let m = uniform_load_max_moment(100.0, 10.0);
        assert!(approx_eq(m, 1250.0), "M_max = {} (expected 1250)", m);
    }

    #[test]
    fn test_uniform_load_shear_at_supports() {
        // V at x=0 should be +wL/2
        // V at x=L should be -wL/2
        let v0 = uniform_load_shear(100.0, 10.0, 0.0);
        let vl = uniform_load_shear(100.0, 10.0, 10.0);
        assert!(approx_eq(v0, 500.0), "V(0) = {}", v0);
        assert!(approx_eq(vl, -500.0), "V(L) = {}", vl);
    }

    #[test]
    fn test_uniform_load_shear_at_midspan() {
        // V at midspan should be 0
        let v = uniform_load_shear(100.0, 10.0, 5.0);
        assert!(approx_eq(v, 0.0), "V(L/2) = {}", v);
    }

    // Partial uniform load tests
    #[test]
    fn test_partial_uniform_symmetric() {
        // 10 ft beam, 100 plf from 2 to 8 ft (symmetric)
        // Total load = 100 * 6 = 600 lb, centroid at 5 ft
        // R1 = R2 = 300 lb
        let (r1, r2) = partial_uniform_reactions(100.0, 2.0, 8.0, 10.0);
        assert!(approx_eq(r1, 300.0), "R1 = {} (expected 300)", r1);
        assert!(approx_eq(r2, 300.0), "R2 = {} (expected 300)", r2);
    }

    // Superposition principle test
    #[test]
    fn test_superposition() {
        // Two point loads should superimpose
        let l = 10.0;
        let p1 = 500.0;
        let a1 = 3.0;
        let p2 = 500.0;
        let a2 = 7.0;

        let (r1_1, r2_1) = point_load_reactions(p1, a1, l);
        let (r1_2, r2_2) = point_load_reactions(p2, a2, l);

        // Combined reaction
        let r1_total = r1_1 + r1_2;
        let r2_total = r2_1 + r2_2;

        // Should equal sum of individual reactions
        // P1 at 3ft: R1 = 500*7/10 = 350, R2 = 150
        // P2 at 7ft: R1 = 500*3/10 = 150, R2 = 350
        // Total: R1 = R2 = 500
        assert!(approx_eq(r1_total, 500.0), "R1_total = {}", r1_total);
        assert!(approx_eq(r2_total, 500.0), "R2_total = {}", r2_total);
    }

    // =======================================================================
    // FIXED-END MOMENT TESTS
    // =======================================================================

    #[test]
    fn test_fem_uniform_full() {
        // 10 ft beam, 100 plf
        // FEM = wL²/12 = 100 * 100 / 12 = 833.33 ft-lb
        let (fem_a, fem_b) = fem_uniform_full(100.0, 10.0);
        assert!(
            approx_eq_tolerance(fem_a.abs(), 833.33, 1.0),
            "FEM_A = {} (expected -833.33)",
            fem_a
        );
        assert!(
            approx_eq_tolerance(fem_b.abs(), 833.33, 1.0),
            "FEM_B = {} (expected +833.33)",
            fem_b
        );
        // Signs: A is negative (counterclockwise), B is positive (clockwise)
        assert!(fem_a < 0.0, "FEM_A should be negative");
        assert!(fem_b > 0.0, "FEM_B should be positive");
    }

    #[test]
    fn test_fem_point_load_midspan() {
        // 10 ft beam, 1000 lb at midspan
        // FEM_A = -Pab²/L² = -1000 * 5 * 25 / 100 = -1250 ft-lb
        // FEM_B = +Pa²b/L² = +1000 * 25 * 5 / 100 = +1250 ft-lb
        let (fem_a, fem_b) = fem_point_load(1000.0, 5.0, 10.0);
        assert!(
            approx_eq_tolerance(fem_a, -1250.0, 1.0),
            "FEM_A = {} (expected -1250)",
            fem_a
        );
        assert!(
            approx_eq_tolerance(fem_b, 1250.0, 1.0),
            "FEM_B = {} (expected +1250)",
            fem_b
        );
    }

    // =======================================================================
    // FIXED-FIXED BEAM TESTS
    // =======================================================================

    #[test]
    fn test_fixed_fixed_uniform_end_moments() {
        // 10 ft beam, 100 plf
        // M_end = wL²/12 = 100 * 100 / 12 = 833.33 ft-lb
        let (m_a, m_b) = fixed_fixed_uniform_end_moments(100.0, 10.0);
        assert!(
            approx_eq_tolerance(m_a, 833.33, 1.0),
            "M_A = {} (expected 833.33)",
            m_a
        );
        assert!(
            approx_eq_tolerance(m_b, 833.33, 1.0),
            "M_B = {} (expected 833.33)",
            m_b
        );
    }

    #[test]
    fn test_fixed_fixed_uniform_max_positive_moment() {
        // 10 ft beam, 100 plf
        // M_max = wL²/24 = 100 * 100 / 24 = 416.67 ft-lb
        let m = fixed_fixed_uniform_max_positive_moment(100.0, 10.0);
        assert!(
            approx_eq_tolerance(m, 416.67, 1.0),
            "M_max = {} (expected 416.67)",
            m
        );
    }

    #[test]
    fn test_fixed_fixed_uniform_moment_at_midspan() {
        // 10 ft beam, 100 plf
        // At midspan x=5: M = wL²/24 = 416.67 ft-lb (positive, sagging)
        let m = fixed_fixed_uniform_moment(100.0, 10.0, 5.0);
        assert!(
            approx_eq_tolerance(m, 416.67, 1.0),
            "M(L/2) = {} (expected 416.67)",
            m
        );
    }

    #[test]
    fn test_fixed_fixed_uniform_moment_at_support() {
        // At x=0: M = -wL²/12 (hogging)
        let m = fixed_fixed_uniform_moment(100.0, 10.0, 0.0);
        assert!(
            approx_eq_tolerance(m, -833.33, 1.0),
            "M(0) = {} (expected -833.33)",
            m
        );
    }

    #[test]
    fn test_fixed_fixed_uniform_max_deflection() {
        // 10 ft beam, 100 plf, E = 1.6e6 psi, I = 100 in⁴
        // δ_max = wL⁴/(384EI)
        // w = 100 plf = 8.333 lb/in
        // L = 120 in
        // δ_max = 8.333 * 120^4 / (384 * 1.6e6 * 100)
        //       = 8.333 * 207,360,000 / 61,440,000,000 = 0.0281 in
        let w = 100.0 / 12.0; // Convert plf to lb/in
        let l = 120.0; // inches
        let e = 1.6e6;
        let i = 100.0;
        let delta = fixed_fixed_uniform_max_deflection(w, l, e, i);
        assert!(
            approx_eq_tolerance(delta, 0.0281, 0.001),
            "δ_max = {} (expected ~0.0281)",
            delta
        );
    }

    #[test]
    fn test_fixed_fixed_point_end_moments() {
        // 10 ft beam, 1000 lb at midspan
        // M_A = M_B = Pab²/L² = Pa²b/L² = 1000 * 5 * 25 / 100 = 1250 ft-lb
        let (m_a, m_b) = fixed_fixed_point_end_moments(1000.0, 5.0, 10.0);
        assert!(
            approx_eq_tolerance(m_a, 1250.0, 1.0),
            "M_A = {} (expected 1250)",
            m_a
        );
        assert!(
            approx_eq_tolerance(m_b, 1250.0, 1.0),
            "M_B = {} (expected 1250)",
            m_b
        );
    }

    // =======================================================================
    // CANTILEVER BEAM TESTS
    // =======================================================================

    #[test]
    fn test_cantilever_uniform_reactions() {
        // 10 ft cantilever, 100 plf
        // R = wL = 1000 lb
        // M = wL²/2 = 5000 ft-lb
        let (r, m) = cantilever_uniform_reactions(100.0, 10.0);
        assert!(approx_eq(r, 1000.0), "R = {} (expected 1000)", r);
        assert!(approx_eq(m, 5000.0), "M = {} (expected 5000)", m);
    }

    #[test]
    fn test_cantilever_uniform_moment_at_support() {
        // At x=0: M = -wL²/2 = -5000 ft-lb (hogging)
        let m = cantilever_uniform_moment(100.0, 10.0, 0.0);
        assert!(approx_eq(m, -5000.0), "M(0) = {} (expected -5000)", m);
    }

    #[test]
    fn test_cantilever_uniform_moment_at_free_end() {
        // At x=L: M = 0
        let m = cantilever_uniform_moment(100.0, 10.0, 10.0);
        assert!(approx_eq(m, 0.0), "M(L) = {} (expected 0)", m);
    }

    #[test]
    fn test_cantilever_uniform_shear() {
        // At x=0: V = wL = 1000 lb
        // At x=L: V = 0
        let v0 = cantilever_uniform_shear(100.0, 10.0, 0.0);
        let vl = cantilever_uniform_shear(100.0, 10.0, 10.0);
        assert!(approx_eq(v0, 1000.0), "V(0) = {} (expected 1000)", v0);
        assert!(approx_eq(vl, 0.0), "V(L) = {} (expected 0)", vl);
    }

    #[test]
    fn test_cantilever_point_reactions() {
        // 8 ft cantilever, 500 lb at tip (a = 8 ft)
        // R = P = 500 lb
        // M = Pa = 4000 ft-lb
        let (r, m) = cantilever_point_reactions(500.0, 8.0);
        assert!(approx_eq(r, 500.0), "R = {} (expected 500)", r);
        assert!(approx_eq(m, 4000.0), "M = {} (expected 4000)", m);
    }

    // =======================================================================
    // PROPPED CANTILEVER (FIXED-PINNED) TESTS
    // =======================================================================

    #[test]
    fn test_fixed_pinned_uniform_reactions() {
        // 10 ft propped cantilever, 100 plf
        // R_A = 5wL/8 = 625 lb
        // R_B = 3wL/8 = 375 lb
        // M_A = wL²/8 = 1250 ft-lb
        let (r_a, r_b, m_a) = fixed_pinned_uniform_reactions(100.0, 10.0);
        assert!(approx_eq(r_a, 625.0), "R_A = {} (expected 625)", r_a);
        assert!(approx_eq(r_b, 375.0), "R_B = {} (expected 375)", r_b);
        assert!(approx_eq(m_a, 1250.0), "M_A = {} (expected 1250)", m_a);
    }

    #[test]
    fn test_fixed_pinned_uniform_max_moment() {
        // M_max = 9wL²/128 = 703.125 ft-lb
        let m = fixed_pinned_uniform_max_positive_moment(100.0, 10.0);
        assert!(
            approx_eq_tolerance(m, 703.125, 1.0),
            "M_max = {} (expected 703.125)",
            m
        );
    }
}
