//! Load type definitions per ASCE 7-22
//!
//! This module defines the standard load types used in structural engineering
//! load combinations per ASCE 7-22.

use serde::{Deserialize, Serialize};

/// Load types per ASCE 7-22 Section 2
///
/// These represent the standard load categories used in structural design.
/// Each load type has a standard abbreviation used in load combination equations.
///
/// # Example
/// ```
/// use calc_core::loads::LoadType;
///
/// let dead = LoadType::Dead;
/// assert_eq!(dead.code(), "D");
/// assert_eq!(dead.description(), "Dead load");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadType {
    /// D - Dead load (self-weight of structure and permanent attachments)
    Dead,
    /// L - Live load (floor live load, occupancy)
    Live,
    /// Lr - Roof live load (maintenance, workers)
    LiveRoof,
    /// S - Snow load
    Snow,
    /// R - Rain load
    Rain,
    /// W - Wind load
    Wind,
    /// E - Seismic (earthquake) load
    Seismic,
    /// H - Lateral earth pressure, groundwater pressure
    SoilLateral,
    /// F - Fluid pressure
    Fluid,
    /// T - Self-straining forces (temperature, shrinkage, creep, differential settlement)
    SelfStraining,
}

impl LoadType {
    /// All load types in standard order
    pub const ALL: [LoadType; 10] = [
        LoadType::Dead,
        LoadType::Live,
        LoadType::LiveRoof,
        LoadType::Snow,
        LoadType::Rain,
        LoadType::Wind,
        LoadType::Seismic,
        LoadType::SoilLateral,
        LoadType::Fluid,
        LoadType::SelfStraining,
    ];

    /// Standard abbreviation code (D, L, Lr, S, R, W, E, H, F, T)
    ///
    /// # Example
    /// ```
    /// use calc_core::loads::LoadType;
    /// assert_eq!(LoadType::LiveRoof.code(), "Lr");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            LoadType::Dead => "D",
            LoadType::Live => "L",
            LoadType::LiveRoof => "Lr",
            LoadType::Snow => "S",
            LoadType::Rain => "R",
            LoadType::Wind => "W",
            LoadType::Seismic => "E",
            LoadType::SoilLateral => "H",
            LoadType::Fluid => "F",
            LoadType::SelfStraining => "T",
        }
    }

    /// Human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            LoadType::Dead => "Dead load",
            LoadType::Live => "Live load",
            LoadType::LiveRoof => "Roof live load",
            LoadType::Snow => "Snow load",
            LoadType::Rain => "Rain load",
            LoadType::Wind => "Wind load",
            LoadType::Seismic => "Seismic load",
            LoadType::SoilLateral => "Lateral earth pressure",
            LoadType::Fluid => "Fluid pressure",
            LoadType::SelfStraining => "Self-straining forces",
        }
    }

    /// Whether this load type can act in multiple directions (requires +/- consideration)
    ///
    /// Wind and seismic loads are directional and may need to be considered
    /// in both positive and negative directions.
    pub fn is_directional(&self) -> bool {
        matches!(self, LoadType::Wind | LoadType::Seismic)
    }

    /// Whether this load type is a gravity load (acts downward)
    pub fn is_gravity(&self) -> bool {
        matches!(
            self,
            LoadType::Dead | LoadType::Live | LoadType::LiveRoof | LoadType::Snow | LoadType::Rain
        )
    }

    /// Whether this load type is a lateral/environmental load
    pub fn is_environmental(&self) -> bool {
        matches!(
            self,
            LoadType::Wind | LoadType::Seismic | LoadType::Snow | LoadType::Rain
        )
    }
}

impl std::fmt::Display for LoadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_type_codes() {
        assert_eq!(LoadType::Dead.code(), "D");
        assert_eq!(LoadType::Live.code(), "L");
        assert_eq!(LoadType::LiveRoof.code(), "Lr");
        assert_eq!(LoadType::Snow.code(), "S");
        assert_eq!(LoadType::Rain.code(), "R");
        assert_eq!(LoadType::Wind.code(), "W");
        assert_eq!(LoadType::Seismic.code(), "E");
        assert_eq!(LoadType::SoilLateral.code(), "H");
        assert_eq!(LoadType::Fluid.code(), "F");
        assert_eq!(LoadType::SelfStraining.code(), "T");
    }

    #[test]
    fn test_directional_loads() {
        assert!(LoadType::Wind.is_directional());
        assert!(LoadType::Seismic.is_directional());
        assert!(!LoadType::Dead.is_directional());
        assert!(!LoadType::Live.is_directional());
    }

    #[test]
    fn test_gravity_loads() {
        assert!(LoadType::Dead.is_gravity());
        assert!(LoadType::Live.is_gravity());
        assert!(LoadType::Snow.is_gravity());
        assert!(!LoadType::Wind.is_gravity());
        assert!(!LoadType::Seismic.is_gravity());
    }

    #[test]
    fn test_serialization() {
        let load = LoadType::LiveRoof;
        let json = serde_json::to_string(&load).unwrap();
        assert_eq!(json, "\"LiveRoof\"");

        let parsed: LoadType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, LoadType::LiveRoof);
    }

    #[test]
    fn test_all_contains_all_variants() {
        assert_eq!(LoadType::ALL.len(), 10);
    }
}
