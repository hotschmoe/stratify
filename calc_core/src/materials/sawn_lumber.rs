//! Sawn Lumber Materials (NDS 2018 Table 4A)
//!
//! Reference design values for visually graded dimension lumber.
//! All values are for 2"-4" thick members (2x10 and wider).
//!
//! Base grade values (SS, No1, No2, No3) are loaded from TOML at compile time.
//! Derived grades (Stud, Construction, Standard, Utility) are computed from base grades.

use serde::{Deserialize, Serialize};

use crate::errors::{CalcError, CalcResult};
use crate::generated::sawn_lumber_data;
use crate::units::Psi;

/// Wood species groups per NDS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum WoodSpecies {
    /// Douglas Fir-Larch
    #[serde(rename = "DF-L")]
    DouglasFirLarch,
    /// Southern Pine
    #[serde(rename = "SP")]
    SouthernPine,
    /// Hem-Fir
    #[serde(rename = "HF")]
    HemFir,
    /// Spruce-Pine-Fir
    #[serde(rename = "SPF")]
    SprucePineFir,
    /// Douglas Fir-South
    #[serde(rename = "DF-S")]
    DouglasFirSouth,
}

impl WoodSpecies {
    /// All wood species variants for UI selection
    pub const ALL: [WoodSpecies; 5] = [
        WoodSpecies::DouglasFirLarch,
        WoodSpecies::SouthernPine,
        WoodSpecies::HemFir,
        WoodSpecies::SprucePineFir,
        WoodSpecies::DouglasFirSouth,
    ];

    /// Get the code string for TOML lookup (e.g., "DF-L", "SP")
    pub fn code(&self) -> &'static str {
        match self {
            WoodSpecies::DouglasFirLarch => "DF-L",
            WoodSpecies::SouthernPine => "SP",
            WoodSpecies::HemFir => "HF",
            WoodSpecies::SprucePineFir => "SPF",
            WoodSpecies::DouglasFirSouth => "DF-S",
        }
    }

    /// Parse from common string representations
    pub fn from_str_flexible(s: &str) -> CalcResult<Self> {
        match s.to_uppercase().replace([' ', '_'], "-").as_str() {
            "DF-L" | "DOUGLAS-FIR-LARCH" | "DFL" => Ok(WoodSpecies::DouglasFirLarch),
            "SP" | "SOUTHERN-PINE" => Ok(WoodSpecies::SouthernPine),
            "HF" | "HEM-FIR" => Ok(WoodSpecies::HemFir),
            "SPF" | "SPRUCE-PINE-FIR" => Ok(WoodSpecies::SprucePineFir),
            "DF-S" | "DOUGLAS-FIR-SOUTH" | "DFS" => Ok(WoodSpecies::DouglasFirSouth),
            _ => Err(CalcError::material_not_found(s)),
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            WoodSpecies::DouglasFirLarch => "Douglas Fir-Larch",
            WoodSpecies::SouthernPine => "Southern Pine",
            WoodSpecies::HemFir => "Hem-Fir",
            WoodSpecies::SprucePineFir => "Spruce-Pine-Fir",
            WoodSpecies::DouglasFirSouth => "Douglas Fir-South",
        }
    }
}

impl std::fmt::Display for WoodSpecies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Wood grades per NDS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WoodGrade {
    /// Select Structural
    #[serde(rename = "SS")]
    SelectStructural,
    /// No. 1
    #[serde(rename = "No.1")]
    No1,
    /// No. 2
    #[serde(rename = "No.2")]
    No2,
    /// No. 3
    #[serde(rename = "No.3")]
    No3,
    /// Stud
    Stud,
    /// Construction
    Construction,
    /// Standard
    Standard,
    /// Utility
    Utility,
}

impl WoodGrade {
    /// All wood grade variants for UI selection
    pub const ALL: [WoodGrade; 8] = [
        WoodGrade::SelectStructural,
        WoodGrade::No1,
        WoodGrade::No2,
        WoodGrade::No3,
        WoodGrade::Stud,
        WoodGrade::Construction,
        WoodGrade::Standard,
        WoodGrade::Utility,
    ];

    /// Get the code string for TOML lookup (e.g., "SS", "No1")
    /// Returns None for derived grades (Stud, Construction, Standard, Utility)
    pub fn code(&self) -> Option<&'static str> {
        match self {
            WoodGrade::SelectStructural => Some("SS"),
            WoodGrade::No1 => Some("No1"),
            WoodGrade::No2 => Some("No2"),
            WoodGrade::No3 => Some("No3"),
            // Derived grades are computed from base grades, not stored in TOML
            WoodGrade::Stud
            | WoodGrade::Construction
            | WoodGrade::Standard
            | WoodGrade::Utility => None,
        }
    }

    /// Parse from common string representations
    pub fn from_str_flexible(s: &str) -> CalcResult<Self> {
        match s.to_uppercase().replace([' ', '.', '#'], "").as_str() {
            "SS" | "SELECTSTRUCTURAL" | "SELSTR" => Ok(WoodGrade::SelectStructural),
            "NO1" | "1" | "N1" => Ok(WoodGrade::No1),
            "NO2" | "2" | "N2" => Ok(WoodGrade::No2),
            "NO3" | "3" | "N3" => Ok(WoodGrade::No3),
            "STUD" => Ok(WoodGrade::Stud),
            "CONST" | "CONSTRUCTION" => Ok(WoodGrade::Construction),
            "STANDARD" | "STD" => Ok(WoodGrade::Standard),
            "UTILITY" | "UTIL" => Ok(WoodGrade::Utility),
            _ => Err(CalcError::material_not_found(s)),
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            WoodGrade::SelectStructural => "Select Structural",
            WoodGrade::No1 => "No. 1",
            WoodGrade::No2 => "No. 2",
            WoodGrade::No3 => "No. 3",
            WoodGrade::Stud => "Stud",
            WoodGrade::Construction => "Construction",
            WoodGrade::Standard => "Standard",
            WoodGrade::Utility => "Utility",
        }
    }
}

impl std::fmt::Display for WoodGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Reference design values for wood (NDS Table 4A)
///
/// All values are in psi unless otherwise noted.
/// These are base values before adjustment factors.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WoodProperties {
    /// Species
    pub species: WoodSpecies,
    /// Grade
    pub grade: WoodGrade,
    /// Bending stress Fb (psi)
    pub fb_psi: f64,
    /// Tension parallel to grain Ft (psi)
    pub ft_psi: f64,
    /// Shear parallel to grain Fv (psi)
    pub fv_psi: f64,
    /// Compression perpendicular to grain Fc_perp (psi)
    pub fc_perp_psi: f64,
    /// Compression parallel to grain Fc (psi)
    pub fc_psi: f64,
    /// Modulus of elasticity E (psi)
    pub e_psi: f64,
    /// Minimum modulus of elasticity Emin (psi)
    pub e_min_psi: f64,
    /// Specific gravity G
    pub specific_gravity: f64,
}

impl WoodProperties {
    /// Look up wood properties by species and grade.
    ///
    /// Base grades (SS, No1, No2, No3) are loaded from TOML data compiled at build time.
    /// Derived grades (Stud, Construction, Standard, Utility) are computed from base grades.
    ///
    /// # Example
    ///
    /// ```rust
    /// use calc_core::materials::{WoodSpecies, WoodGrade, WoodProperties};
    ///
    /// let props = WoodProperties::lookup(WoodSpecies::DouglasFirLarch, WoodGrade::No2);
    /// assert!(props.fb_psi > 0.0);
    /// ```
    pub fn lookup(species: WoodSpecies, grade: WoodGrade) -> Self {
        // Check if this is a base grade with TOML data
        if let Some(grade_code) = grade.code() {
            // Look up from generated TOML data
            if let Some(props) = sawn_lumber_data::lookup(species.code(), grade_code) {
                return WoodProperties {
                    species,
                    grade,
                    fb_psi: props.fb_psi,
                    ft_psi: props.ft_psi,
                    fv_psi: props.fv_psi,
                    fc_perp_psi: props.fc_perp_psi,
                    fc_psi: props.fc_psi,
                    e_psi: props.e_psi,
                    e_min_psi: props.e_min_psi,
                    specific_gravity: props.specific_gravity,
                };
            }
        }

        // Derived grades are computed from base grades
        match grade {
            // Stud grade - use similar to No.3 for structural calcs
            WoodGrade::Stud => {
                let base = Self::lookup(species, WoodGrade::No3);
                WoodProperties {
                    grade: WoodGrade::Stud,
                    ..base
                }
            }

            // Construction - approximately 15% higher Fb than No.2
            WoodGrade::Construction => {
                let base = Self::lookup(species, WoodGrade::No2);
                WoodProperties {
                    grade: WoodGrade::Construction,
                    fb_psi: base.fb_psi * 1.15,
                    ..base
                }
            }

            // Standard - same as No.3
            WoodGrade::Standard => {
                let base = Self::lookup(species, WoodGrade::No3);
                WoodProperties {
                    grade: WoodGrade::Standard,
                    ..base
                }
            }

            // Utility - reduced strength from No.3
            WoodGrade::Utility => {
                let base = Self::lookup(species, WoodGrade::No3);
                WoodProperties {
                    grade: WoodGrade::Utility,
                    fb_psi: base.fb_psi * 0.6,
                    fc_psi: base.fc_psi * 0.6,
                    ..base
                }
            }

            // Base grades should have been handled above
            _ => unreachable!("Base grade {} should have TOML data", grade),
        }
    }

    /// Get Fb as a typed unit
    pub fn fb(&self) -> Psi {
        Psi(self.fb_psi)
    }

    /// Get Fv as a typed unit
    pub fn fv(&self) -> Psi {
        Psi(self.fv_psi)
    }

    /// Get E as a typed unit
    pub fn e(&self) -> Psi {
        Psi(self.e_psi)
    }
}

/// Combined wood material identifier for serialization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WoodMaterial {
    pub species: WoodSpecies,
    pub grade: WoodGrade,
}

impl WoodMaterial {
    pub fn new(species: WoodSpecies, grade: WoodGrade) -> Self {
        Self { species, grade }
    }

    /// Parse from string like "DF-L No.2"
    pub fn from_str_flexible(s: &str) -> CalcResult<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(CalcError::invalid_input(
                "material",
                s,
                "Expected format: 'SPECIES GRADE' (e.g., 'DF-L No.2')",
            ));
        }
        Ok(Self {
            species: WoodSpecies::from_str_flexible(parts[0])?,
            grade: WoodGrade::from_str_flexible(parts[1])?,
        })
    }

    /// Get properties for this material
    pub fn properties(&self) -> WoodProperties {
        WoodProperties::lookup(self.species, self.grade)
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        format!(
            "{} {}",
            self.species.display_name(),
            self.grade.display_name()
        )
    }
}

impl std::fmt::Display for WoodMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wood_lookup() {
        let props = WoodProperties::lookup(WoodSpecies::DouglasFirLarch, WoodGrade::No2);
        assert_eq!(props.fb_psi, 900.0);
        assert_eq!(props.e_psi, 1_600_000.0);
    }

    #[test]
    fn test_species_parsing() {
        assert_eq!(
            WoodSpecies::from_str_flexible("DF-L").unwrap(),
            WoodSpecies::DouglasFirLarch
        );
        assert_eq!(
            WoodSpecies::from_str_flexible("southern pine").unwrap(),
            WoodSpecies::SouthernPine
        );
    }

    #[test]
    fn test_grade_parsing() {
        assert_eq!(
            WoodGrade::from_str_flexible("No.2").unwrap(),
            WoodGrade::No2
        );
        assert_eq!(WoodGrade::from_str_flexible("#2").unwrap(), WoodGrade::No2);
        assert_eq!(
            WoodGrade::from_str_flexible("SS").unwrap(),
            WoodGrade::SelectStructural
        );
    }

    #[test]
    fn test_material_parsing() {
        let mat = WoodMaterial::from_str_flexible("DF-L No.2").unwrap();
        assert_eq!(mat.species, WoodSpecies::DouglasFirLarch);
        assert_eq!(mat.grade, WoodGrade::No2);
    }

    #[test]
    fn test_serialization() {
        let props = WoodProperties::lookup(WoodSpecies::SouthernPine, WoodGrade::No1);
        let json = serde_json::to_string(&props).unwrap();
        let roundtrip: WoodProperties = serde_json::from_str(&json).unwrap();
        assert_eq!(props.fb_psi, roundtrip.fb_psi);
    }

    #[test]
    fn test_material_display() {
        let mat = WoodMaterial::new(WoodSpecies::DouglasFirLarch, WoodGrade::No2);
        assert_eq!(mat.display_name(), "Douglas Fir-Larch No. 2");
    }
}
