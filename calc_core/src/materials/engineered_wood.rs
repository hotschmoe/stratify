//! Engineered Wood Products
//!
//! Reference design values for structural composite lumber products:
//! - Glulam (Structural Glued Laminated Timber) - NDS-S
//! - LVL (Laminated Veneer Lumber)
//! - PSL (Parallel Strand Lumber)
//!
//! Base design values are loaded from TOML at compile time.

use serde::{Deserialize, Serialize};

use crate::errors::{CalcError, CalcResult};
use crate::generated::engineered_wood_data;

// ============================================================================
// Glulam (Structural Glued Laminated Timber)
// ============================================================================

/// Glulam stress class per ANSI/APA PRG 320 and NDS-S
///
/// The naming convention: FbValue-EValue (e.g., 24F-1.8E means Fb=2400 psi, E=1.8 million psi)
/// V-grades (e.g., 24F-V4) are for visually graded tension laminations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)] // Industry standard naming
pub enum GlulamStressClass {
    /// 16F-1.3E - Economy grade
    #[serde(rename = "16F-1.3E")]
    F16_E1_3,
    /// 20F-1.5E - Standard grade
    #[serde(rename = "20F-1.5E")]
    F20_E1_5,
    /// 24F-1.7E
    #[serde(rename = "24F-1.7E")]
    F24_E1_7,
    /// 24F-1.8E - Common structural grade
    #[serde(rename = "24F-1.8E")]
    F24_E1_8,
    /// 26F-1.9E - High strength
    #[serde(rename = "26F-1.9E")]
    F26_E1_9,
    /// 24F-V4 - Visually graded, common for beams
    #[serde(rename = "24F-V4")]
    F24_V4,
    /// 24F-V8 - Visually graded, balanced layup
    #[serde(rename = "24F-V8")]
    F24_V8,
}

impl GlulamStressClass {
    /// All stress classes for UI selection
    pub const ALL: [GlulamStressClass; 7] = [
        GlulamStressClass::F16_E1_3,
        GlulamStressClass::F20_E1_5,
        GlulamStressClass::F24_E1_7,
        GlulamStressClass::F24_E1_8,
        GlulamStressClass::F26_E1_9,
        GlulamStressClass::F24_V4,
        GlulamStressClass::F24_V8,
    ];

    /// Get the code string for TOML lookup (e.g., "24F-V4")
    pub fn code(&self) -> &'static str {
        match self {
            GlulamStressClass::F16_E1_3 => "16F-1.3E",
            GlulamStressClass::F20_E1_5 => "20F-1.5E",
            GlulamStressClass::F24_E1_7 => "24F-1.7E",
            GlulamStressClass::F24_E1_8 => "24F-1.8E",
            GlulamStressClass::F26_E1_9 => "26F-1.9E",
            GlulamStressClass::F24_V4 => "24F-V4",
            GlulamStressClass::F24_V8 => "24F-V8",
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        self.code()
    }
}

impl std::fmt::Display for GlulamStressClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Glulam layup orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GlulamLayup {
    /// Balanced - tension and compression lams are equal (same Fb+ and Fb-)
    #[default]
    Balanced,
    /// Unbalanced - optimized for positive bending (tension face has higher grade lams)
    /// Fb+ > Fb- (use Fb- for negative moment regions)
    Unbalanced,
}

impl GlulamLayup {
    pub const ALL: [GlulamLayup; 2] = [GlulamLayup::Balanced, GlulamLayup::Unbalanced];

    pub fn display_name(&self) -> &'static str {
        match self {
            GlulamLayup::Balanced => "Balanced",
            GlulamLayup::Unbalanced => "Unbalanced",
        }
    }
}

impl std::fmt::Display for GlulamLayup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Reference design values for Glulam per NDS-S
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlulamProperties {
    /// Stress class
    pub stress_class: GlulamStressClass,
    /// Bending stress Fb+ for positive bending (tension on bottom, psi)
    pub fb_pos_psi: f64,
    /// Bending stress Fb- for negative bending (tension on top, psi)
    /// For balanced layups, fb_neg_psi == fb_pos_psi
    pub fb_neg_psi: f64,
    /// Tension parallel Ft (psi)
    pub ft_psi: f64,
    /// Shear Fv (psi)
    pub fv_psi: f64,
    /// Compression perpendicular Fc_perp (psi)
    pub fc_perp_psi: f64,
    /// Compression parallel Fc (psi)
    pub fc_psi: f64,
    /// Modulus of elasticity E (psi)
    pub e_psi: f64,
    /// Minimum E for stability Emin (psi)
    pub e_min_psi: f64,
    /// Specific gravity
    pub specific_gravity: f64,
}

impl GlulamProperties {
    /// Look up glulam properties by stress class
    ///
    /// Design values are loaded from TOML at compile time.
    /// For unbalanced layups, fb_neg_psi is lower than fb_pos_psi.
    ///
    /// Returns `Err(CalcError::MaterialNotFound)` if the build script did not
    /// emit data for this variant; the exhaustiveness test in `tests` catches
    /// this in CI.
    pub fn lookup(stress_class: GlulamStressClass) -> CalcResult<Self> {
        let props = engineered_wood_data::lookup_glulam(stress_class.code()).ok_or_else(|| {
            CalcError::material_not_found(format!("glulam {}", stress_class.code()))
        })?;

        Ok(GlulamProperties {
            stress_class,
            fb_pos_psi: props.fb_pos_psi,
            fb_neg_psi: props.fb_neg_psi,
            ft_psi: props.ft_psi,
            fv_psi: props.fv_psi,
            fc_perp_psi: props.fc_perp_psi,
            fc_psi: props.fc_psi,
            e_psi: props.e_psi,
            e_min_psi: props.e_min_psi,
            specific_gravity: props.specific_gravity,
        })
    }

    /// Get Fb to use based on moment direction and layup
    pub fn fb_for_moment(&self, is_positive_moment: bool, layup: GlulamLayup) -> f64 {
        match (is_positive_moment, layup) {
            (true, _) => self.fb_pos_psi,
            (false, GlulamLayup::Balanced) => self.fb_pos_psi,
            (false, GlulamLayup::Unbalanced) => self.fb_neg_psi,
        }
    }
}

/// Glulam material specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlulamMaterial {
    /// Stress class (e.g., 24F-V4)
    pub stress_class: GlulamStressClass,
    /// Layup type
    pub layup: GlulamLayup,
}

impl GlulamMaterial {
    pub fn new(stress_class: GlulamStressClass, layup: GlulamLayup) -> Self {
        Self {
            stress_class,
            layup,
        }
    }

    /// Get properties for this material
    pub fn properties(&self) -> CalcResult<GlulamProperties> {
        GlulamProperties::lookup(self.stress_class)
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        format!("Glulam {} ({})", self.stress_class, self.layup)
    }
}

impl Default for GlulamMaterial {
    fn default() -> Self {
        Self {
            stress_class: GlulamStressClass::F24_V4,
            layup: GlulamLayup::Unbalanced,
        }
    }
}

impl std::fmt::Display for GlulamMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// LVL (Laminated Veneer Lumber)
// ============================================================================

/// LVL grade designations (manufacturer-independent approach)
///
/// LVL properties vary by manufacturer. These grades represent common
/// property ranges available in the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LvlGrade {
    /// Standard structural LVL (E ~2.0 million psi, Fb ~2600 psi)
    #[default]
    #[serde(rename = "LVL-2.0E")]
    Standard,
    /// High-strength LVL (E ~2.2 million psi, Fb ~2900 psi)
    #[serde(rename = "LVL-2.2E")]
    HighStrength,
}

impl LvlGrade {
    pub const ALL: [LvlGrade; 2] = [LvlGrade::Standard, LvlGrade::HighStrength];

    /// Get the code string for TOML lookup (e.g., "LVL-2.0E")
    pub fn code(&self) -> &'static str {
        match self {
            LvlGrade::Standard => "LVL-2.0E",
            LvlGrade::HighStrength => "LVL-2.2E",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LvlGrade::Standard => "LVL 2.0E",
            LvlGrade::HighStrength => "LVL 2.2E",
        }
    }
}

impl std::fmt::Display for LvlGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Reference design values for LVL
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LvlProperties {
    /// Grade
    pub grade: LvlGrade,
    /// Bending stress Fb (psi) - base value at 12" depth
    pub fb_psi: f64,
    /// Tension parallel Ft (psi)
    pub ft_psi: f64,
    /// Shear Fv (psi)
    pub fv_psi: f64,
    /// Compression perpendicular Fc_perp (psi)
    pub fc_perp_psi: f64,
    /// Compression parallel Fc (psi)
    pub fc_psi: f64,
    /// Modulus of elasticity E (psi)
    pub e_psi: f64,
    /// Minimum E for stability Emin (psi)
    pub e_min_psi: f64,
    /// Specific gravity
    pub specific_gravity: f64,
}

impl LvlProperties {
    /// Look up LVL properties by grade
    ///
    /// Design values are loaded from TOML at compile time.
    ///
    /// Returns `Err(CalcError::MaterialNotFound)` if the build script did not
    /// emit data for this variant.
    pub fn lookup(grade: LvlGrade) -> CalcResult<Self> {
        let props = engineered_wood_data::lookup_lvl(grade.code())
            .ok_or_else(|| CalcError::material_not_found(format!("LVL {}", grade.code())))?;

        Ok(LvlProperties {
            grade,
            fb_psi: props.fb_psi,
            ft_psi: props.ft_psi,
            fv_psi: props.fv_psi,
            fc_perp_psi: props.fc_perp_psi,
            fc_psi: props.fc_psi,
            e_psi: props.e_psi,
            e_min_psi: props.e_min_psi,
            specific_gravity: props.specific_gravity,
        })
    }

    /// Apply depth adjustment for LVL bending
    ///
    /// LVL has a depth factor similar to sawn lumber. Common approach:
    /// For d > 12": Fb' = Fb * (12/d)^0.111
    pub fn adjusted_fb(&self, depth_in: f64) -> f64 {
        if depth_in > 12.0 {
            self.fb_psi * (12.0 / depth_in).powf(0.111)
        } else {
            self.fb_psi
        }
    }
}

/// LVL material specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LvlMaterial {
    /// Grade designation
    pub grade: LvlGrade,
}

impl LvlMaterial {
    pub fn new(grade: LvlGrade) -> Self {
        Self { grade }
    }

    /// Get properties for this material
    pub fn properties(&self) -> CalcResult<LvlProperties> {
        LvlProperties::lookup(self.grade)
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        self.grade.display_name().to_string()
    }
}

impl Default for LvlMaterial {
    fn default() -> Self {
        Self {
            grade: LvlGrade::Standard,
        }
    }
}

impl std::fmt::Display for LvlMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// PSL (Parallel Strand Lumber)
// ============================================================================

/// PSL grade designations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PslGrade {
    /// Standard PSL (E ~2.0 million psi)
    #[default]
    #[serde(rename = "PSL-2.0E")]
    Standard,
}

impl PslGrade {
    pub const ALL: [PslGrade; 1] = [PslGrade::Standard];

    /// Get the code string for TOML lookup (e.g., "PSL-2.0E")
    pub fn code(&self) -> &'static str {
        match self {
            PslGrade::Standard => "PSL-2.0E",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PslGrade::Standard => "PSL 2.0E",
        }
    }
}

impl std::fmt::Display for PslGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Reference design values for PSL
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PslProperties {
    /// Grade
    pub grade: PslGrade,
    /// Bending stress Fb (psi)
    pub fb_psi: f64,
    /// Tension parallel Ft (psi)
    pub ft_psi: f64,
    /// Shear Fv (psi)
    pub fv_psi: f64,
    /// Compression perpendicular Fc_perp (psi)
    pub fc_perp_psi: f64,
    /// Compression parallel Fc (psi)
    pub fc_psi: f64,
    /// Modulus of elasticity E (psi)
    pub e_psi: f64,
    /// Minimum E for stability Emin (psi)
    pub e_min_psi: f64,
    /// Specific gravity
    pub specific_gravity: f64,
}

impl PslProperties {
    /// Look up PSL properties by grade
    ///
    /// Design values are loaded from TOML at compile time.
    ///
    /// Returns `Err(CalcError::MaterialNotFound)` if the build script did not
    /// emit data for this variant.
    pub fn lookup(grade: PslGrade) -> CalcResult<Self> {
        let props = engineered_wood_data::lookup_psl(grade.code())
            .ok_or_else(|| CalcError::material_not_found(format!("PSL {}", grade.code())))?;

        Ok(PslProperties {
            grade,
            fb_psi: props.fb_psi,
            ft_psi: props.ft_psi,
            fv_psi: props.fv_psi,
            fc_perp_psi: props.fc_perp_psi,
            fc_psi: props.fc_psi,
            e_psi: props.e_psi,
            e_min_psi: props.e_min_psi,
            specific_gravity: props.specific_gravity,
        })
    }

    /// Apply depth adjustment for PSL bending (similar to LVL)
    pub fn adjusted_fb(&self, depth_in: f64) -> f64 {
        if depth_in > 12.0 {
            self.fb_psi * (12.0 / depth_in).powf(0.111)
        } else {
            self.fb_psi
        }
    }
}

/// PSL material specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PslMaterial {
    /// Grade designation
    pub grade: PslGrade,
}

impl PslMaterial {
    pub fn new(grade: PslGrade) -> Self {
        Self { grade }
    }

    /// Get properties for this material
    pub fn properties(&self) -> CalcResult<PslProperties> {
        PslProperties::lookup(self.grade)
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        self.grade.display_name().to_string()
    }
}

impl Default for PslMaterial {
    fn default() -> Self {
        Self {
            grade: PslGrade::Standard,
        }
    }
}

impl std::fmt::Display for PslMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Glulam tests
    #[test]
    fn test_glulam_24f_v4_properties() {
        let props = GlulamProperties::lookup(GlulamStressClass::F24_V4).unwrap();
        assert_eq!(props.fb_pos_psi, 2400.0);
        assert_eq!(props.fb_neg_psi, 1450.0); // Unbalanced
        assert_eq!(props.e_psi, 1_800_000.0);
    }

    #[test]
    fn test_glulam_balanced_fb() {
        let props = GlulamProperties::lookup(GlulamStressClass::F24_V8).unwrap();
        // V8 is balanced layup: Fb+ = Fb- = 2400 psi per NDS-S
        assert_eq!(props.fb_pos_psi, 2400.0);
        assert_eq!(props.fb_neg_psi, 2400.0);
        assert_eq!(props.fb_pos_psi, props.fb_neg_psi);
    }

    #[test]
    fn test_glulam_fb_for_moment() {
        let props = GlulamProperties::lookup(GlulamStressClass::F24_V4).unwrap();

        // Positive moment
        assert_eq!(props.fb_for_moment(true, GlulamLayup::Unbalanced), 2400.0);
        // Negative moment with unbalanced
        assert_eq!(props.fb_for_moment(false, GlulamLayup::Unbalanced), 1450.0);
        // Negative moment with balanced
        assert_eq!(props.fb_for_moment(false, GlulamLayup::Balanced), 2400.0);
    }

    #[test]
    fn test_glulam_material_display() {
        let mat = GlulamMaterial::new(GlulamStressClass::F24_V4, GlulamLayup::Unbalanced);
        assert_eq!(mat.display_name(), "Glulam 24F-V4 (Unbalanced)");
    }

    #[test]
    fn test_glulam_serialization() {
        let mat = GlulamMaterial::new(GlulamStressClass::F24_E1_8, GlulamLayup::Balanced);
        let json = serde_json::to_string(&mat).unwrap();
        let parsed: GlulamMaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(mat.stress_class, parsed.stress_class);
        assert_eq!(mat.layup, parsed.layup);
        // Verify deserialized material returns correct properties
        let props = parsed.properties().unwrap();
        assert_eq!(props.fb_pos_psi, 2400.0);
        assert_eq!(props.e_psi, 1_800_000.0);
    }

    // LVL tests
    #[test]
    fn test_lvl_standard_properties() {
        let props = LvlProperties::lookup(LvlGrade::Standard).unwrap();
        assert_eq!(props.fb_psi, 2600.0);
        assert_eq!(props.e_psi, 2_000_000.0);
    }

    #[test]
    fn test_lvl_depth_adjustment() {
        let props = LvlProperties::lookup(LvlGrade::Standard).unwrap();
        let fb_12 = props.adjusted_fb(12.0);
        let fb_18 = props.adjusted_fb(18.0);

        // Base value at 12"
        assert_eq!(fb_12, props.fb_psi);
        // Deeper member should have lower adjusted Fb
        assert!(fb_18 < fb_12);
        // Check approximate value: (12/18)^0.111 ≈ 0.956
        assert!((fb_18 / props.fb_psi - 0.956).abs() < 0.01);
    }

    #[test]
    fn test_lvl_serialization() {
        let mat = LvlMaterial::new(LvlGrade::HighStrength);
        let json = serde_json::to_string(&mat).unwrap();
        let parsed: LvlMaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(mat.grade, parsed.grade);
        // Verify deserialized material returns correct properties
        let props = parsed.properties().unwrap();
        assert_eq!(props.fb_psi, 2900.0);
        assert_eq!(props.e_psi, 2_200_000.0);
    }

    // PSL tests
    #[test]
    fn test_psl_standard_properties() {
        let props = PslProperties::lookup(PslGrade::Standard).unwrap();
        assert_eq!(props.fb_psi, 2900.0);
        assert_eq!(props.e_psi, 2_000_000.0);
    }

    #[test]
    fn test_psl_depth_adjustment() {
        let props = PslProperties::lookup(PslGrade::Standard).unwrap();
        let fb_12 = props.adjusted_fb(12.0);
        let fb_16 = props.adjusted_fb(16.0);

        assert_eq!(fb_12, props.fb_psi);
        assert!(fb_16 < fb_12);
    }

    #[test]
    fn test_psl_serialization() {
        let mat = PslMaterial::new(PslGrade::Standard);
        let json = serde_json::to_string(&mat).unwrap();
        let parsed: PslMaterial = serde_json::from_str(&json).unwrap();
        assert_eq!(mat.grade, parsed.grade);
        // Verify deserialized material returns correct properties
        let props = parsed.properties().unwrap();
        assert_eq!(props.fb_psi, 2900.0);
        assert_eq!(props.e_psi, 2_000_000.0);
    }

    #[test]
    fn test_defaults() {
        // Verify defaults have valid, usable properties
        let glulam = GlulamMaterial::default();
        assert_eq!(glulam.stress_class, GlulamStressClass::F24_V4);
        let glulam_props = glulam.properties().unwrap();
        assert!(glulam_props.fb_pos_psi > 0.0);
        assert!(glulam_props.e_psi > 1_000_000.0);

        let lvl = LvlMaterial::default();
        assert_eq!(lvl.grade, LvlGrade::Standard);
        let lvl_props = lvl.properties().unwrap();
        assert!(lvl_props.fb_psi > 0.0);
        assert!(lvl_props.e_psi > 1_000_000.0);

        let psl = PslMaterial::default();
        assert_eq!(psl.grade, PslGrade::Standard);
        let psl_props = psl.properties().unwrap();
        assert!(psl_props.fb_psi > 0.0);
        assert!(psl_props.e_psi > 1_000_000.0);
    }

    /// Exhaustiveness check: every Glulam/LVL/PSL enum variant must have
    /// corresponding TOML data emitted by `build.rs`. Catches a missing TOML
    /// entry (or a renamed code) at CI time instead of at runtime.
    #[test]
    fn test_all_engineered_wood_variants_have_data() {
        for sc in GlulamStressClass::ALL {
            GlulamProperties::lookup(sc)
                .unwrap_or_else(|e| panic!("missing glulam data for {sc:?}: {e}"));
        }
        for g in LvlGrade::ALL {
            LvlProperties::lookup(g).unwrap_or_else(|e| panic!("missing LVL data for {g:?}: {e}"));
        }
        for g in PslGrade::ALL {
            PslProperties::lookup(g).unwrap_or_else(|e| panic!("missing PSL data for {g:?}: {e}"));
        }
    }
}
