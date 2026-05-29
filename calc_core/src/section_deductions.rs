//! Section Deductions for Wood Beams
//!
//! This module handles notches and holes that reduce the effective cross-section
//! of wood beams per NDS requirements.
//!
//! ## Notches (NDS 3.2.3, 4.4.3)
//!
//! - Notches at supports: max depth = d/4 for sawn lumber
//! - Notches at tension face: not permitted except at supports
//! - Notch reduces shear capacity via reduced effective depth
//!
//! ## Holes/Borings (NDS 3.2.4)
//!
//! - Should be located in middle third of beam depth when possible
//! - Max diameter typically d/3
//! - Reduces net section for bending and shear checks

use serde::{Deserialize, Serialize};

/// Notch location along the beam
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NotchLocation {
    /// No notch
    #[default]
    None,
    /// Notch at left support
    LeftSupport,
    /// Notch at right support
    RightSupport,
    /// Notch at both supports
    BothSupports,
}

impl NotchLocation {
    pub const ALL: [NotchLocation; 4] = [
        NotchLocation::None,
        NotchLocation::LeftSupport,
        NotchLocation::RightSupport,
        NotchLocation::BothSupports,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            NotchLocation::None => "None",
            NotchLocation::LeftSupport => "Left support",
            NotchLocation::RightSupport => "Right support",
            NotchLocation::BothSupports => "Both supports",
        }
    }

    pub fn has_notch_at_left(&self) -> bool {
        matches!(
            self,
            NotchLocation::LeftSupport | NotchLocation::BothSupports
        )
    }

    pub fn has_notch_at_right(&self) -> bool {
        matches!(
            self,
            NotchLocation::RightSupport | NotchLocation::BothSupports
        )
    }

    pub fn has_any_notch(&self) -> bool {
        !matches!(self, NotchLocation::None)
    }
}

impl std::fmt::Display for NotchLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Section deductions for holes, notches, etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SectionDeductions {
    /// Notch locations
    pub notch_location: NotchLocation,

    /// Notch depth at left support (inches)
    /// Per NDS, max = d/4 for sawn lumber
    #[serde(default)]
    pub notch_depth_left_in: f64,

    /// Notch depth at right support (inches)
    #[serde(default)]
    pub notch_depth_right_in: f64,

    /// Hole diameter (inches) - largest hole
    /// Per NDS, holes should be in middle 1/3 of depth, max diameter d/3
    #[serde(default)]
    pub hole_diameter_in: f64,

    /// Number of holes in beam
    #[serde(default)]
    pub hole_count: u8,
}

impl SectionDeductions {
    /// Create new section deductions with no reductions
    pub fn none() -> Self {
        Self::default()
    }

    /// Add a notch at the left support
    pub fn with_left_notch(mut self, depth_in: f64) -> Self {
        self.notch_depth_left_in = depth_in;
        self.notch_location = match self.notch_location {
            NotchLocation::None => NotchLocation::LeftSupport,
            NotchLocation::RightSupport => NotchLocation::BothSupports,
            other => other,
        };
        self
    }

    /// Add a notch at the right support
    pub fn with_right_notch(mut self, depth_in: f64) -> Self {
        self.notch_depth_right_in = depth_in;
        self.notch_location = match self.notch_location {
            NotchLocation::None => NotchLocation::RightSupport,
            NotchLocation::LeftSupport => NotchLocation::BothSupports,
            other => other,
        };
        self
    }

    /// Add holes
    pub fn with_holes(mut self, diameter_in: f64, count: u8) -> Self {
        self.hole_diameter_in = diameter_in;
        self.hole_count = count;
        self
    }

    /// Check if there are any deductions
    pub fn has_deductions(&self) -> bool {
        self.notch_location.has_any_notch() || (self.hole_diameter_in > 0.0 && self.hole_count > 0)
    }

    /// Calculate effective depth at left support (after notch)
    pub fn effective_depth_left_in(&self, full_depth_in: f64) -> f64 {
        if self.notch_location.has_notch_at_left() {
            (full_depth_in - self.notch_depth_left_in).max(0.0)
        } else {
            full_depth_in
        }
    }

    /// Calculate effective depth at right support (after notch)
    pub fn effective_depth_right_in(&self, full_depth_in: f64) -> f64 {
        if self.notch_location.has_notch_at_right() {
            (full_depth_in - self.notch_depth_right_in).max(0.0)
        } else {
            full_depth_in
        }
    }

    /// Calculate minimum effective depth (considering notches at both ends)
    pub fn min_effective_depth_in(&self, full_depth_in: f64) -> f64 {
        self.effective_depth_left_in(full_depth_in)
            .min(self.effective_depth_right_in(full_depth_in))
    }

    /// Calculate shear stress adjustment factor at notched section per NDS Eq. 3.4-3
    ///
    /// fv_adj = fv * (d/d_e)^2 for tension-side notches
    ///
    /// This returns the multiplier to apply to actual shear stress,
    /// or equivalently the reduction to allowable shear stress.
    pub fn notch_shear_factor(&self, full_depth_in: f64) -> f64 {
        let d_e_left = self.effective_depth_left_in(full_depth_in);
        let d_e_right = self.effective_depth_right_in(full_depth_in);
        let min_d_e = d_e_left.min(d_e_right);

        if min_d_e >= full_depth_in || min_d_e <= 0.0 {
            1.0
        } else {
            // NDS 3.4.3.2: fv ≤ Fv' * (2/3) * (d_e/d)^2
            // This means actual fv is multiplied by (d/d_e)^2
            (full_depth_in / min_d_e).powi(2)
        }
    }

    /// Check if notch exceeds NDS limits (d/4 for sawn lumber)
    pub fn notch_exceeds_limit(&self, full_depth_in: f64) -> bool {
        let max_notch = full_depth_in / 4.0;
        self.notch_depth_left_in > max_notch || self.notch_depth_right_in > max_notch
    }

    /// Check if hole exceeds NDS recommendations (d/3)
    pub fn hole_exceeds_recommendation(&self, full_depth_in: f64) -> bool {
        self.hole_diameter_in > full_depth_in / 3.0
    }

    /// Calculate net width reduction from holes (for bending check)
    ///
    /// Returns the reduction in effective width due to holes.
    /// For a hole at mid-depth (worst case for bending):
    /// Net section modulus S_net = S * (d - d_hole) / d
    pub fn net_section_factor(&self, full_depth_in: f64) -> f64 {
        if self.hole_diameter_in > 0.0 && self.hole_count > 0 {
            // Conservative: assume hole at worst location for bending
            let net_depth = full_depth_in - self.hole_diameter_in;
            if net_depth <= 0.0 {
                0.0
            } else {
                // Approximate net section modulus factor
                // For hole at neutral axis: S_net ≈ S * (1 - (d_hole/d)^3)
                let ratio = self.hole_diameter_in / full_depth_in;
                (1.0 - ratio.powi(3)).max(0.1)
            }
        } else {
            1.0
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_deductions() {
        let ded = SectionDeductions::none();
        assert!(!ded.has_deductions());
        assert_eq!(ded.effective_depth_left_in(10.0), 10.0);
        assert_eq!(ded.effective_depth_right_in(10.0), 10.0);
        assert_eq!(ded.notch_shear_factor(10.0), 1.0);
    }

    #[test]
    fn test_left_notch() {
        let ded = SectionDeductions::none().with_left_notch(2.0);
        assert!(ded.has_deductions());
        assert!(ded.notch_location.has_notch_at_left());
        assert!(!ded.notch_location.has_notch_at_right());
        assert_eq!(ded.effective_depth_left_in(10.0), 8.0);
        assert_eq!(ded.effective_depth_right_in(10.0), 10.0);
    }

    #[test]
    fn test_both_notches() {
        let ded = SectionDeductions::none()
            .with_left_notch(2.0)
            .with_right_notch(3.0);
        assert!(ded.notch_location.has_notch_at_left());
        assert!(ded.notch_location.has_notch_at_right());
        assert_eq!(ded.effective_depth_left_in(10.0), 8.0);
        assert_eq!(ded.effective_depth_right_in(10.0), 7.0);
        assert_eq!(ded.min_effective_depth_in(10.0), 7.0);
    }

    #[test]
    fn test_notch_shear_factor() {
        let ded = SectionDeductions::none().with_left_notch(2.5);
        // Full depth = 10", effective = 7.5"
        // Factor = (10/7.5)^2 = 1.78
        let factor = ded.notch_shear_factor(10.0);
        assert!((factor - 1.78).abs() < 0.01);
    }

    #[test]
    fn test_notch_exceeds_limit() {
        let ded = SectionDeductions::none().with_left_notch(3.0);
        // 3.0 > 10.0/4 = 2.5, so exceeds limit
        assert!(ded.notch_exceeds_limit(10.0));

        let ded2 = SectionDeductions::none().with_left_notch(2.0);
        // 2.0 < 2.5, so OK
        assert!(!ded2.notch_exceeds_limit(10.0));
    }

    #[test]
    fn test_holes() {
        let ded = SectionDeductions::none().with_holes(1.5, 2);
        assert!(ded.has_deductions());
        assert!(!ded.hole_exceeds_recommendation(10.0)); // 1.5 < 10/3 = 3.33

        let ded2 = SectionDeductions::none().with_holes(4.0, 1);
        assert!(ded2.hole_exceeds_recommendation(10.0)); // 4.0 > 3.33
    }

    #[test]
    fn test_net_section_factor() {
        let ded = SectionDeductions::none().with_holes(2.0, 1);
        // For 10" depth with 2" hole: factor ≈ 1 - (0.2)^3 = 0.992
        let factor = ded.net_section_factor(10.0);
        assert!((factor - 0.992).abs() < 0.01);
    }

    #[test]
    fn test_serialization() {
        let ded = SectionDeductions::none()
            .with_left_notch(2.0)
            .with_holes(1.5, 2);
        let json = serde_json::to_string(&ded).unwrap();
        let parsed: SectionDeductions = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.notch_depth_left_in, 2.0);
        assert_eq!(parsed.hole_diameter_in, 1.5);
        assert_eq!(parsed.hole_count, 2);
    }
}
