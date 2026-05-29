//! # Materials Database
//!
//! Material definitions and property lookups for structural engineering.
//! Supports sawn lumber (NDS) and engineered wood products.
//!
//! ## Material Types
//!
//! - **Sawn Lumber**: Visually graded dimension lumber per NDS Table 4A
//! - **Glulam**: Structural glued laminated timber per NDS-S
//! - **LVL**: Laminated Veneer Lumber
//! - **PSL**: Parallel Strand Lumber
//!
//! ## Example
//!
//! ```rust
//! use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
//!
//! // Sawn lumber
//! let lumber = Material::SawnLumber(WoodMaterial::new(
//!     WoodSpecies::DouglasFirLarch,
//!     WoodGrade::No2
//! ));
//!
//! // Get unified properties (engineered-wood lookups can fail if TOML data is missing)
//! let props = lumber.base_properties().unwrap();
//! println!("Fb = {} psi, E = {} psi", props.fb_psi, props.e_psi);
//! ```

pub mod engineered_wood;
pub mod lumber_sizes;
pub mod sawn_lumber;
pub mod steel;

// Re-export sawn lumber types
pub use sawn_lumber::{WoodGrade, WoodMaterial, WoodProperties, WoodSpecies};

// Re-export lumber size types
pub use lumber_sizes::{BeamDesignation, LumberSize, PlyCount};

// Re-export engineered wood types
pub use engineered_wood::{
    GlulamLayup, GlulamMaterial, GlulamProperties, GlulamStressClass, LvlGrade, LvlMaterial,
    LvlProperties, PslGrade, PslMaterial, PslProperties,
};

// Re-export steel types
pub use steel::{builtin_common_shapes, ShapeType, SteelShape, SteelShapeDb};

use serde::{Deserialize, Serialize};

use crate::errors::CalcResult;

/// Unified material properties for all wood types
///
/// This provides a common interface for calculations that need to work
/// with any wood material type (sawn lumber, glulam, LVL, PSL).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UnifiedWoodProperties {
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

/// Unified material enum for all structural materials
///
/// This enum allows beam and column calculations to work with any
/// supported material type through a common interface.
///
/// ## JSON Serialization
///
/// Materials serialize with a "type" discriminator:
///
/// ```json
/// // Sawn lumber (legacy format also supported)
/// { "type": "SawnLumber", "species": "DF-L", "grade": "No.2" }
///
/// // Glulam
/// { "type": "Glulam", "stress_class": "24F-V4", "layup": "Unbalanced" }
///
/// // LVL
/// { "type": "Lvl", "grade": "LVL-2.0E" }
///
/// // PSL
/// { "type": "Psl", "grade": "PSL-2.0E" }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Material {
    /// Sawn dimensional lumber (NDS Table 4A)
    SawnLumber(WoodMaterial),
    /// Structural glued laminated timber (NDS-S)
    Glulam(GlulamMaterial),
    /// Laminated Veneer Lumber
    Lvl(LvlMaterial),
    /// Parallel Strand Lumber
    Psl(PslMaterial),
}

impl Material {
    /// Get the base reference design values for this material
    ///
    /// For glulam with unbalanced layup, this returns Fb+ (positive bending).
    /// Use `fb_for_depth` to get depth-adjusted values for engineered lumber.
    ///
    /// Returns `Err(CalcError::MaterialNotFound)` if a referenced engineered-wood
    /// variant has no TOML data emitted by `build.rs`.
    pub fn base_properties(&self) -> CalcResult<UnifiedWoodProperties> {
        Ok(match self {
            Material::SawnLumber(mat) => {
                let props = mat.properties();
                UnifiedWoodProperties {
                    fb_psi: props.fb_psi,
                    ft_psi: props.ft_psi,
                    fv_psi: props.fv_psi,
                    fc_perp_psi: props.fc_perp_psi,
                    fc_psi: props.fc_psi,
                    e_psi: props.e_psi,
                    e_min_psi: props.e_min_psi,
                    specific_gravity: props.specific_gravity,
                }
            }
            Material::Glulam(mat) => {
                let props = mat.properties()?;
                UnifiedWoodProperties {
                    fb_psi: props.fb_pos_psi, // Use positive bending as base
                    ft_psi: props.ft_psi,
                    fv_psi: props.fv_psi,
                    fc_perp_psi: props.fc_perp_psi,
                    fc_psi: props.fc_psi,
                    e_psi: props.e_psi,
                    e_min_psi: props.e_min_psi,
                    specific_gravity: props.specific_gravity,
                }
            }
            Material::Lvl(mat) => {
                let props = mat.properties()?;
                UnifiedWoodProperties {
                    fb_psi: props.fb_psi,
                    ft_psi: props.ft_psi,
                    fv_psi: props.fv_psi,
                    fc_perp_psi: props.fc_perp_psi,
                    fc_psi: props.fc_psi,
                    e_psi: props.e_psi,
                    e_min_psi: props.e_min_psi,
                    specific_gravity: props.specific_gravity,
                }
            }
            Material::Psl(mat) => {
                let props = mat.properties()?;
                UnifiedWoodProperties {
                    fb_psi: props.fb_psi,
                    ft_psi: props.ft_psi,
                    fv_psi: props.fv_psi,
                    fc_perp_psi: props.fc_perp_psi,
                    fc_psi: props.fc_psi,
                    e_psi: props.e_psi,
                    e_min_psi: props.e_min_psi,
                    specific_gravity: props.specific_gravity,
                }
            }
        })
    }

    /// Get Fb adjusted for member depth
    ///
    /// For LVL and PSL, applies the depth adjustment factor.
    /// For sawn lumber and glulam, returns the base Fb (size factor applied separately).
    pub fn fb_for_depth(&self, depth_in: f64) -> CalcResult<f64> {
        Ok(match self {
            Material::SawnLumber(mat) => mat.properties().fb_psi,
            Material::Glulam(mat) => mat.properties()?.fb_pos_psi,
            Material::Lvl(mat) => mat.properties()?.adjusted_fb(depth_in),
            Material::Psl(mat) => mat.properties()?.adjusted_fb(depth_in),
        })
    }

    /// Get display name for this material
    pub fn display_name(&self) -> String {
        match self {
            Material::SawnLumber(mat) => mat.display_name(),
            Material::Glulam(mat) => mat.display_name(),
            Material::Lvl(mat) => mat.display_name(),
            Material::Psl(mat) => mat.display_name(),
        }
    }

    /// Get material type as a string
    pub fn material_type(&self) -> &'static str {
        match self {
            Material::SawnLumber(_) => "Sawn Lumber",
            Material::Glulam(_) => "Glulam",
            Material::Lvl(_) => "LVL",
            Material::Psl(_) => "PSL",
        }
    }

    /// Check if this is an engineered wood product
    pub fn is_engineered(&self) -> bool {
        !matches!(self, Material::SawnLumber(_))
    }
}

impl Default for Material {
    fn default() -> Self {
        Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ))
    }
}

impl std::fmt::Display for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// Convenience conversions
impl From<WoodMaterial> for Material {
    fn from(mat: WoodMaterial) -> Self {
        Material::SawnLumber(mat)
    }
}

impl From<GlulamMaterial> for Material {
    fn from(mat: GlulamMaterial) -> Self {
        Material::Glulam(mat)
    }
}

impl From<LvlMaterial> for Material {
    fn from(mat: LvlMaterial) -> Self {
        Material::Lvl(mat)
    }
}

impl From<PslMaterial> for Material {
    fn from(mat: PslMaterial) -> Self {
        Material::Psl(mat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_sawn_lumber() {
        let mat = Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ));

        let props = mat.base_properties().unwrap();
        assert_eq!(props.fb_psi, 900.0);
        assert_eq!(props.e_psi, 1_600_000.0);
    }

    #[test]
    fn test_material_glulam() {
        let mat = Material::Glulam(GlulamMaterial::new(
            GlulamStressClass::F24_V4,
            GlulamLayup::Unbalanced,
        ));

        let props = mat.base_properties().unwrap();
        assert_eq!(props.fb_psi, 2400.0);
        assert_eq!(props.e_psi, 1_800_000.0);
    }

    #[test]
    fn test_material_lvl() {
        let mat = Material::Lvl(LvlMaterial::new(LvlGrade::Standard));

        let props = mat.base_properties().unwrap();
        assert_eq!(props.fb_psi, 2600.0);
        assert_eq!(props.e_psi, 2_000_000.0);
    }

    #[test]
    fn test_material_psl() {
        let mat = Material::Psl(PslMaterial::new(PslGrade::Standard));

        let props = mat.base_properties().unwrap();
        assert_eq!(props.fb_psi, 2900.0);
    }

    #[test]
    fn test_fb_for_depth_lvl() {
        let mat = Material::Lvl(LvlMaterial::new(LvlGrade::Standard));

        let fb_12 = mat.fb_for_depth(12.0).unwrap();
        let fb_18 = mat.fb_for_depth(18.0).unwrap();

        assert_eq!(fb_12, 2600.0);
        assert!(fb_18 < fb_12);
    }

    #[test]
    fn test_material_display_names() {
        let lumber = Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ));
        assert_eq!(lumber.display_name(), "Douglas Fir-Larch No. 2");

        let glulam = Material::Glulam(GlulamMaterial::new(
            GlulamStressClass::F24_V4,
            GlulamLayup::Unbalanced,
        ));
        assert_eq!(glulam.display_name(), "Glulam 24F-V4 (Unbalanced)");

        let lvl = Material::Lvl(LvlMaterial::default());
        assert_eq!(lvl.display_name(), "LVL 2.0E");
    }

    #[test]
    fn test_material_type() {
        let lumber = Material::default();
        assert_eq!(lumber.material_type(), "Sawn Lumber");
        assert!(!lumber.is_engineered());

        let glulam = Material::Glulam(GlulamMaterial::default());
        assert_eq!(glulam.material_type(), "Glulam");
        assert!(glulam.is_engineered());
    }

    #[test]
    fn test_material_serialization() {
        // Test sawn lumber
        let lumber = Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ));
        let json = serde_json::to_string(&lumber).unwrap();
        assert!(json.contains("\"type\":\"SawnLumber\""));
        let parsed: Material = serde_json::from_str(&json).unwrap();
        assert_eq!(lumber, parsed);

        // Test glulam
        let glulam = Material::Glulam(GlulamMaterial::new(
            GlulamStressClass::F24_V4,
            GlulamLayup::Unbalanced,
        ));
        let json = serde_json::to_string(&glulam).unwrap();
        assert!(json.contains("\"type\":\"Glulam\""));
        let parsed: Material = serde_json::from_str(&json).unwrap();
        assert_eq!(glulam, parsed);

        // Test LVL
        let lvl = Material::Lvl(LvlMaterial::new(LvlGrade::HighStrength));
        let json = serde_json::to_string(&lvl).unwrap();
        assert!(json.contains("\"type\":\"Lvl\""));
        let parsed: Material = serde_json::from_str(&json).unwrap();
        assert_eq!(lvl, parsed);
    }

    #[test]
    fn test_from_conversions() {
        let wood = WoodMaterial::new(WoodSpecies::SouthernPine, WoodGrade::No1);
        let mat: Material = wood.into();
        assert!(matches!(mat, Material::SawnLumber(_)));

        let glulam = GlulamMaterial::default();
        let mat: Material = glulam.into();
        assert!(matches!(mat, Material::Glulam(_)));
    }

    #[test]
    fn test_default() {
        let mat = Material::default();
        assert!(matches!(mat, Material::SawnLumber(_)));
        if let Material::SawnLumber(wood) = mat {
            assert_eq!(wood.species, WoodSpecies::DouglasFirLarch);
            assert_eq!(wood.grade, WoodGrade::No2);
        }
    }
}
