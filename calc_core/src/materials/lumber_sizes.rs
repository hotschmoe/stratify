//! Standard Lumber Sizes
//!
//! Provides standard lumber size designations with nominal-to-actual conversions
//! per NDS/AWC standards. Supports multi-ply configurations for built-up beams.
//!
//! ## Nominal vs Actual Dimensions
//!
//! - 2x nominal = 1.5" actual
//! - 4x nominal = 3.5" actual
//! - 6x nominal = 5.5" actual
//! - etc.
//!
//! ## Multi-Ply Beams
//!
//! For built-up beams (e.g., 3-2x12), the total width is the ply count
//! times the single-ply width.

use serde::{Deserialize, Serialize};

/// Standard lumber size designation
///
/// Represents nominal lumber dimensions with automatic actual dimension lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LumberSize {
    /// 2x4 (1.5" x 3.5")
    L2x4,
    /// 2x6 (1.5" x 5.5")
    L2x6,
    /// 2x8 (1.5" x 7.25")
    L2x8,
    /// 2x10 (1.5" x 9.25")
    #[default]
    L2x10,
    /// 2x12 (1.5" x 11.25")
    L2x12,
    /// 2x14 (1.5" x 13.25")
    L2x14,
    /// 4x4 (3.5" x 3.5")
    L4x4,
    /// 4x6 (3.5" x 5.5")
    L4x6,
    /// 4x8 (3.5" x 7.25")
    L4x8,
    /// 4x10 (3.5" x 9.25")
    L4x10,
    /// 4x12 (3.5" x 11.25")
    L4x12,
    /// 6x6 (5.5" x 5.5")
    L6x6,
    /// 6x8 (5.5" x 7.5")
    L6x8,
    /// 6x10 (5.5" x 9.5")
    L6x10,
    /// 6x12 (5.5" x 11.5")
    L6x12,
    /// 8x8 (7.5" x 7.5")
    L8x8,
    /// 8x10 (7.5" x 9.5")
    L8x10,
    /// 8x12 (7.5" x 11.5")
    L8x12,
    /// Custom size - user specifies actual dimensions
    Custom,
}

impl LumberSize {
    /// All standard lumber sizes for UI selection (most common beam sizes first)
    pub const ALL: [LumberSize; 19] = [
        LumberSize::L2x10,
        LumberSize::L2x12,
        LumberSize::L2x8,
        LumberSize::L2x6,
        LumberSize::L2x4,
        LumberSize::L2x14,
        LumberSize::L4x10,
        LumberSize::L4x12,
        LumberSize::L4x8,
        LumberSize::L4x6,
        LumberSize::L4x4,
        LumberSize::L6x10,
        LumberSize::L6x12,
        LumberSize::L6x8,
        LumberSize::L6x6,
        LumberSize::L8x10,
        LumberSize::L8x12,
        LumberSize::L8x8,
        LumberSize::Custom,
    ];

    /// Common 2x sizes for joists and rafters
    pub const DIMENSION_2X: [LumberSize; 6] = [
        LumberSize::L2x4,
        LumberSize::L2x6,
        LumberSize::L2x8,
        LumberSize::L2x10,
        LumberSize::L2x12,
        LumberSize::L2x14,
    ];

    /// Common 4x sizes for beams
    pub const POST_4X: [LumberSize; 5] = [
        LumberSize::L4x4,
        LumberSize::L4x6,
        LumberSize::L4x8,
        LumberSize::L4x10,
        LumberSize::L4x12,
    ];

    /// Get the actual dimensions (width, depth) in inches for a single ply
    ///
    /// Returns (actual_width_in, actual_depth_in)
    pub fn actual_dimensions(&self) -> (f64, f64) {
        match self {
            // 2x series
            LumberSize::L2x4 => (1.5, 3.5),
            LumberSize::L2x6 => (1.5, 5.5),
            LumberSize::L2x8 => (1.5, 7.25),
            LumberSize::L2x10 => (1.5, 9.25),
            LumberSize::L2x12 => (1.5, 11.25),
            LumberSize::L2x14 => (1.5, 13.25),
            // 4x series
            LumberSize::L4x4 => (3.5, 3.5),
            LumberSize::L4x6 => (3.5, 5.5),
            LumberSize::L4x8 => (3.5, 7.25),
            LumberSize::L4x10 => (3.5, 9.25),
            LumberSize::L4x12 => (3.5, 11.25),
            // 6x series (note: 6x and larger have different actual dims)
            LumberSize::L6x6 => (5.5, 5.5),
            LumberSize::L6x8 => (5.5, 7.5),
            LumberSize::L6x10 => (5.5, 9.5),
            LumberSize::L6x12 => (5.5, 11.5),
            // 8x series
            LumberSize::L8x8 => (7.5, 7.5),
            LumberSize::L8x10 => (7.5, 9.5),
            LumberSize::L8x12 => (7.5, 11.5),
            // Custom - return zeros, caller must provide actual dims
            LumberSize::Custom => (0.0, 0.0),
        }
    }

    /// Get the nominal dimensions (width, depth) in inches
    pub fn nominal_dimensions(&self) -> (u8, u8) {
        match self {
            LumberSize::L2x4 => (2, 4),
            LumberSize::L2x6 => (2, 6),
            LumberSize::L2x8 => (2, 8),
            LumberSize::L2x10 => (2, 10),
            LumberSize::L2x12 => (2, 12),
            LumberSize::L2x14 => (2, 14),
            LumberSize::L4x4 => (4, 4),
            LumberSize::L4x6 => (4, 6),
            LumberSize::L4x8 => (4, 8),
            LumberSize::L4x10 => (4, 10),
            LumberSize::L4x12 => (4, 12),
            LumberSize::L6x6 => (6, 6),
            LumberSize::L6x8 => (6, 8),
            LumberSize::L6x10 => (6, 10),
            LumberSize::L6x12 => (6, 12),
            LumberSize::L8x8 => (8, 8),
            LumberSize::L8x10 => (8, 10),
            LumberSize::L8x12 => (8, 12),
            LumberSize::Custom => (0, 0),
        }
    }

    /// Get display name (e.g., "2x10")
    pub fn display_name(&self) -> &'static str {
        match self {
            LumberSize::L2x4 => "2x4",
            LumberSize::L2x6 => "2x6",
            LumberSize::L2x8 => "2x8",
            LumberSize::L2x10 => "2x10",
            LumberSize::L2x12 => "2x12",
            LumberSize::L2x14 => "2x14",
            LumberSize::L4x4 => "4x4",
            LumberSize::L4x6 => "4x6",
            LumberSize::L4x8 => "4x8",
            LumberSize::L4x10 => "4x10",
            LumberSize::L4x12 => "4x12",
            LumberSize::L6x6 => "6x6",
            LumberSize::L6x8 => "6x8",
            LumberSize::L6x10 => "6x10",
            LumberSize::L6x12 => "6x12",
            LumberSize::L8x8 => "8x8",
            LumberSize::L8x10 => "8x10",
            LumberSize::L8x12 => "8x12",
            LumberSize::Custom => "Custom",
        }
    }

    /// Get actual width for single ply in inches
    pub fn width_in(&self) -> f64 {
        self.actual_dimensions().0
    }

    /// Get actual depth in inches
    pub fn depth_in(&self) -> f64 {
        self.actual_dimensions().1
    }

    /// Get area for single ply in square inches
    pub fn area_in2(&self) -> f64 {
        let (w, d) = self.actual_dimensions();
        w * d
    }

    /// Get section modulus for single ply (bd²/6) in in³
    pub fn section_modulus_in3(&self) -> f64 {
        let (w, d) = self.actual_dimensions();
        w * d * d / 6.0
    }

    /// Get moment of inertia for single ply (bd³/12) in in⁴
    pub fn moment_of_inertia_in4(&self) -> f64 {
        let (w, d) = self.actual_dimensions();
        w * d.powi(3) / 12.0
    }

    /// Check if this is a custom size
    pub fn is_custom(&self) -> bool {
        matches!(self, LumberSize::Custom)
    }

    /// Try to match actual dimensions to a standard size
    pub fn from_actual_dimensions(width_in: f64, depth_in: f64) -> Self {
        for size in Self::ALL.iter() {
            if *size == LumberSize::Custom {
                continue;
            }
            let (w, d) = size.actual_dimensions();
            if (w - width_in).abs() < 0.01 && (d - depth_in).abs() < 0.01 {
                return *size;
            }
        }
        LumberSize::Custom
    }
}

impl std::fmt::Display for LumberSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// PLY COUNT
// ============================================================================

/// Number of plies for built-up beams
///
/// Standard configurations for multi-ply beams (e.g., 2-2x10, 3-2x12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlyCount {
    /// Single member (1 ply)
    #[default]
    Single,
    /// Double (2 plies)
    Double,
    /// Triple (3 plies)
    Triple,
    /// Quadruple (4 plies)
    Quad,
}

impl PlyCount {
    /// All ply count options for UI selection
    pub const ALL: [PlyCount; 4] = [
        PlyCount::Single,
        PlyCount::Double,
        PlyCount::Triple,
        PlyCount::Quad,
    ];

    /// Get the numeric ply count
    pub fn count(&self) -> u8 {
        match self {
            PlyCount::Single => 1,
            PlyCount::Double => 2,
            PlyCount::Triple => 3,
            PlyCount::Quad => 4,
        }
    }

    /// Get display label
    pub fn display_name(&self) -> &'static str {
        match self {
            PlyCount::Single => "Single",
            PlyCount::Double => "Double (2-ply)",
            PlyCount::Triple => "Triple (3-ply)",
            PlyCount::Quad => "Quad (4-ply)",
        }
    }

    /// Get short label for beam designation (e.g., "2-", "3-", "")
    pub fn prefix(&self) -> &'static str {
        match self {
            PlyCount::Single => "",
            PlyCount::Double => "2-",
            PlyCount::Triple => "3-",
            PlyCount::Quad => "4-",
        }
    }

    /// Create from numeric count
    pub fn from_count(count: u8) -> Self {
        match count {
            1 => PlyCount::Single,
            2 => PlyCount::Double,
            3 => PlyCount::Triple,
            4 => PlyCount::Quad,
            _ => PlyCount::Single, // Default to single for invalid counts
        }
    }
}

impl std::fmt::Display for PlyCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// BUILT-UP BEAM DESIGNATION
// ============================================================================

/// Complete beam designation with size and ply count
///
/// Represents a built-up beam like "2-2x10" or "3-2x12".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeamDesignation {
    /// The lumber size for each ply
    pub size: LumberSize,
    /// Number of plies
    pub plies: PlyCount,
}

impl BeamDesignation {
    /// Create a new beam designation
    pub fn new(size: LumberSize, plies: PlyCount) -> Self {
        Self { size, plies }
    }

    /// Create a single-ply beam
    pub fn single(size: LumberSize) -> Self {
        Self {
            size,
            plies: PlyCount::Single,
        }
    }

    /// Get the total actual width (all plies combined) in inches
    pub fn total_width_in(&self) -> f64 {
        self.size.width_in() * self.plies.count() as f64
    }

    /// Get the depth in inches (same for all plies)
    pub fn depth_in(&self) -> f64 {
        self.size.depth_in()
    }

    /// Get the total cross-sectional area in square inches
    pub fn total_area_in2(&self) -> f64 {
        self.size.area_in2() * self.plies.count() as f64
    }

    /// Get the total section modulus in in³
    pub fn total_section_modulus_in3(&self) -> f64 {
        let b = self.total_width_in();
        let d = self.depth_in();
        b * d * d / 6.0
    }

    /// Get the total moment of inertia in in⁴
    pub fn total_moment_of_inertia_in4(&self) -> f64 {
        let b = self.total_width_in();
        let d = self.depth_in();
        b * d.powi(3) / 12.0
    }

    /// Get display name (e.g., "2-2x10", "3-2x12", "4x10")
    pub fn display_name(&self) -> String {
        format!("{}{}", self.plies.prefix(), self.size.display_name())
    }

    /// Self-weight per linear foot (plf), assuming 35 pcf wood density
    pub fn self_weight_plf(&self) -> f64 {
        const WOOD_DENSITY_PCF: f64 = 35.0;
        self.total_area_in2() * WOOD_DENSITY_PCF / 144.0
    }
}

impl Default for BeamDesignation {
    fn default() -> Self {
        Self {
            size: LumberSize::L2x10,
            plies: PlyCount::Single,
        }
    }
}

impl std::fmt::Display for BeamDesignation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lumber_size_dimensions() {
        let size = LumberSize::L2x10;
        assert_eq!(size.actual_dimensions(), (1.5, 9.25));
        assert_eq!(size.nominal_dimensions(), (2, 10));
        assert_eq!(size.display_name(), "2x10");
    }

    #[test]
    fn test_lumber_size_section_properties() {
        let size = LumberSize::L2x10;
        // I = bd³/12 = 1.5 * 9.25³ / 12 ≈ 98.93
        assert!((size.moment_of_inertia_in4() - 98.93).abs() < 0.1);
        // S = bd²/6 = 1.5 * 9.25² / 6 ≈ 21.39
        assert!((size.section_modulus_in3() - 21.39).abs() < 0.1);
        // A = bd = 1.5 * 9.25 = 13.875
        assert_eq!(size.area_in2(), 13.875);
    }

    #[test]
    fn test_ply_count() {
        assert_eq!(PlyCount::Single.count(), 1);
        assert_eq!(PlyCount::Double.count(), 2);
        assert_eq!(PlyCount::Triple.count(), 3);
        assert_eq!(PlyCount::Quad.count(), 4);

        assert_eq!(PlyCount::Double.prefix(), "2-");
        assert_eq!(PlyCount::Single.prefix(), "");
    }

    #[test]
    fn test_beam_designation() {
        let beam = BeamDesignation::new(LumberSize::L2x10, PlyCount::Double);
        assert_eq!(beam.display_name(), "2-2x10");
        assert_eq!(beam.total_width_in(), 3.0); // 1.5 * 2
        assert_eq!(beam.depth_in(), 9.25);
    }

    #[test]
    fn test_built_up_section_properties() {
        let single = BeamDesignation::single(LumberSize::L2x10);
        let double = BeamDesignation::new(LumberSize::L2x10, PlyCount::Double);

        // Double should have 2x the area
        assert!((double.total_area_in2() - 2.0 * single.total_area_in2()).abs() < 0.001);

        // Double should have 2x the section modulus
        assert!(
            (double.total_section_modulus_in3() - 2.0 * single.total_section_modulus_in3()).abs()
                < 0.01
        );

        // Double should have 2x the moment of inertia
        assert!(
            (double.total_moment_of_inertia_in4() - 2.0 * single.total_moment_of_inertia_in4())
                .abs()
                < 0.1
        );
    }

    #[test]
    fn test_from_actual_dimensions() {
        assert_eq!(
            LumberSize::from_actual_dimensions(1.5, 9.25),
            LumberSize::L2x10
        );
        assert_eq!(
            LumberSize::from_actual_dimensions(1.5, 11.25),
            LumberSize::L2x12
        );
        assert_eq!(
            LumberSize::from_actual_dimensions(3.5, 9.25),
            LumberSize::L4x10
        );
        assert_eq!(
            LumberSize::from_actual_dimensions(2.0, 10.0),
            LumberSize::Custom
        );
    }

    #[test]
    fn test_self_weight() {
        let beam = BeamDesignation::single(LumberSize::L2x10);
        // Expected: 13.875 in² * 35 pcf / 144 ≈ 3.37 plf
        let expected = 13.875 * 35.0 / 144.0;
        assert!((beam.self_weight_plf() - expected).abs() < 0.01);
    }

    #[test]
    fn test_serialization() {
        let size = LumberSize::L2x12;
        let json = serde_json::to_string(&size).unwrap();
        let parsed: LumberSize = serde_json::from_str(&json).unwrap();
        assert_eq!(size, parsed);

        let ply = PlyCount::Triple;
        let json = serde_json::to_string(&ply).unwrap();
        let parsed: PlyCount = serde_json::from_str(&json).unwrap();
        assert_eq!(ply, parsed);

        let beam = BeamDesignation::new(LumberSize::L2x10, PlyCount::Double);
        let json = serde_json::to_string(&beam).unwrap();
        let parsed: BeamDesignation = serde_json::from_str(&json).unwrap();
        assert_eq!(beam, parsed);
    }
}
