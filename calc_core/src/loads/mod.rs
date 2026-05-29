//! Load combinations and load case management per ASCE 7-22
//!
//! This module provides types and functions for working with structural loads
//! and load combinations according to ASCE 7-22.
//!
//! # Overview
//!
//! - [`LoadType`] - Enumeration of all ASCE 7 load categories (D, L, S, W, E, etc.)
//! - [`LoadCase`] - A collection of load values for a specific scenario
//! - [`LoadCombination`] - Factors to apply for code-compliant load combinations
//! - [`DesignMethod`] - ASD vs LRFD design methodology
//!
//! # Example
//!
//! ```
//! use calc_core::loads::{LoadType, LoadCase, DesignMethod, asce7_asd_combinations};
//!
//! // Define loads for a floor beam
//! let floor_loads = LoadCase::new("Second Floor")
//!     .with_load(LoadType::Dead, 15.0)   // psf self-weight
//!     .with_load(LoadType::Live, 40.0);  // psf occupancy
//!
//! // Get ASD combinations and find governing
//! let combos = asce7_asd_combinations();
//! let max_load = combos.iter()
//!     .map(|c| c.apply(&floor_loads))
//!     .fold(0.0f64, f64::max);
//!
//! println!("Governing load: {} psf", max_load);
//! ```

pub mod combinations;
pub mod discrete;
pub mod load_types;

pub use combinations::{
    asce7_asd_combinations, asce7_lrfd_combinations, find_governing_combination,
    find_governing_min_max, find_minimum_combination, GoverningResults, LoadCombination,
};
pub use discrete::{DiscreteLoad, EnhancedLoadCase, LoadDistribution};
pub use load_types::LoadType;

use crate::errors::{CalcError, CalcResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Design methodology selection
///
/// Structural design can use either Allowable Stress Design (ASD) or
/// Load and Resistance Factor Design (LRFD). The choice affects load
/// factors and capacity reduction factors used in calculations.
///
/// # Example
/// ```
/// use calc_core::loads::{DesignMethod, asce7_asd_combinations, asce7_lrfd_combinations};
///
/// let method = DesignMethod::Asd;
/// let combos = match method {
///     DesignMethod::Asd => asce7_asd_combinations(),
///     DesignMethod::Lrfd => asce7_lrfd_combinations(),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DesignMethod {
    /// Allowable Stress Design - uses service loads with safety factors on capacity
    #[default]
    Asd,
    /// Load and Resistance Factor Design - uses factored loads with phi factors on capacity
    Lrfd,
}

impl DesignMethod {
    /// Human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            DesignMethod::Asd => "ASD (Allowable Stress Design)",
            DesignMethod::Lrfd => "LRFD (Load and Resistance Factor Design)",
        }
    }

    /// Short abbreviation
    pub fn code(&self) -> &'static str {
        match self {
            DesignMethod::Asd => "ASD",
            DesignMethod::Lrfd => "LRFD",
        }
    }

    /// Get the appropriate load combinations for this design method
    pub fn combinations(&self) -> Vec<LoadCombination> {
        match self {
            DesignMethod::Asd => asce7_asd_combinations(),
            DesignMethod::Lrfd => asce7_lrfd_combinations(),
        }
    }
}

impl std::fmt::Display for DesignMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// A collection of load values by type for a specific loading scenario
///
/// LoadCase stores unfactored (service) load values. These are combined
/// using LoadCombination factors to get design loads.
///
/// # Example
/// ```
/// use calc_core::loads::{LoadCase, LoadType};
///
/// let case = LoadCase::new("Typical Floor")
///     .with_load(LoadType::Dead, 20.0)    // 20 psf dead
///     .with_load(LoadType::Live, 50.0);   // 50 psf live
///
/// assert_eq!(case.get(LoadType::Dead), 20.0);
/// assert_eq!(case.get(LoadType::Snow), 0.0);  // Not specified, defaults to 0
/// ```
///
/// # JSON Format
/// ```json
/// {
///   "label": "Second Floor",
///   "loads": {
///     "Dead": 20.0,
///     "Live": 50.0
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCase {
    /// User-provided label for this load case
    pub label: String,

    /// Load values keyed by type (units depend on context: psf, plf, kips, etc.)
    pub loads: HashMap<LoadType, f64>,
}

impl LoadCase {
    /// Create a new empty load case with a label
    pub fn new(label: impl Into<String>) -> Self {
        LoadCase {
            label: label.into(),
            loads: HashMap::new(),
        }
    }

    /// Add or update a load value (builder pattern)
    pub fn with_load(mut self, load_type: LoadType, value: f64) -> Self {
        self.loads.insert(load_type, value);
        self
    }

    /// Set a load value (mutable)
    pub fn set_load(&mut self, load_type: LoadType, value: f64) {
        self.loads.insert(load_type, value);
    }

    /// Get the load value for a type, defaulting to 0.0 if not set
    pub fn get(&self, load_type: LoadType) -> f64 {
        self.loads.get(&load_type).copied().unwrap_or(0.0)
    }

    /// Check if a load type is defined (even if zero)
    pub fn has(&self, load_type: LoadType) -> bool {
        self.loads.contains_key(&load_type)
    }

    /// Get all defined load types
    pub fn load_types(&self) -> impl Iterator<Item = &LoadType> {
        self.loads.keys()
    }

    /// Validate the load case
    ///
    /// Checks that gravity loads are non-negative.
    pub fn validate(&self) -> CalcResult<()> {
        for (load_type, value) in &self.loads {
            if load_type.is_gravity() && *value < 0.0 {
                return Err(CalcError::invalid_input(
                    format!("load_{}", load_type.code()),
                    value.to_string(),
                    format!("{} cannot be negative", load_type.description()),
                ));
            }
        }
        Ok(())
    }

    /// Calculate total unfactored gravity load
    pub fn total_gravity(&self) -> f64 {
        LoadType::ALL
            .iter()
            .filter(|lt| lt.is_gravity())
            .map(|lt| self.get(*lt))
            .sum()
    }

    /// Apply all combinations and find the governing (maximum) result
    pub fn governing_load(&self, method: DesignMethod) -> (f64, String) {
        find_governing_combination(self, &method.combinations())
    }

    /// Apply all combinations and return all results
    pub fn all_combination_results(&self, method: DesignMethod) -> Vec<(String, f64)> {
        method
            .combinations()
            .iter()
            .map(|combo| (combo.name.clone(), combo.apply(self)))
            .collect()
    }
}

impl Default for LoadCase {
    fn default() -> Self {
        LoadCase::new("Unnamed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_design_method_default() {
        let method = DesignMethod::default();
        assert_eq!(method, DesignMethod::Asd);
    }

    #[test]
    fn test_design_method_combinations() {
        let asd = DesignMethod::Asd.combinations();
        let lrfd = DesignMethod::Lrfd.combinations();

        assert!(!asd.is_empty());
        assert!(!lrfd.is_empty());
    }

    #[test]
    fn test_load_case_builder() {
        let case = LoadCase::new("Test")
            .with_load(LoadType::Dead, 10.0)
            .with_load(LoadType::Live, 20.0);

        assert_eq!(case.label, "Test");
        assert_eq!(case.get(LoadType::Dead), 10.0);
        assert_eq!(case.get(LoadType::Live), 20.0);
        assert_eq!(case.get(LoadType::Snow), 0.0);
    }

    #[test]
    fn test_load_case_has() {
        let case = LoadCase::new("Test").with_load(LoadType::Dead, 10.0);

        assert!(case.has(LoadType::Dead));
        assert!(!case.has(LoadType::Live));
    }

    #[test]
    fn test_load_case_validation_positive() {
        let case = LoadCase::new("Valid")
            .with_load(LoadType::Dead, 10.0)
            .with_load(LoadType::Live, 20.0);

        assert!(case.validate().is_ok());
    }

    #[test]
    fn test_load_case_validation_negative_gravity() {
        let case = LoadCase::new("Invalid").with_load(LoadType::Dead, -10.0);

        assert!(case.validate().is_err());
    }

    #[test]
    fn test_load_case_negative_lateral_allowed() {
        // Lateral loads can be negative (direction)
        let case = LoadCase::new("Valid")
            .with_load(LoadType::Wind, -50.0)
            .with_load(LoadType::Seismic, -30.0);

        assert!(case.validate().is_ok());
    }

    #[test]
    fn test_total_gravity() {
        let case = LoadCase::new("Floor")
            .with_load(LoadType::Dead, 20.0)
            .with_load(LoadType::Live, 50.0)
            .with_load(LoadType::Wind, 30.0); // Not gravity

        assert_eq!(case.total_gravity(), 70.0);
    }

    #[test]
    fn test_governing_load_asd() {
        let case = LoadCase::new("Test")
            .with_load(LoadType::Dead, 20.0)
            .with_load(LoadType::Live, 40.0);

        let (load, _name) = case.governing_load(DesignMethod::Asd);
        // D + L = 60 is max for ASD with these loads
        assert!((load - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_governing_load_lrfd() {
        let case = LoadCase::new("Test")
            .with_load(LoadType::Dead, 20.0)
            .with_load(LoadType::Live, 40.0);

        let (load, _name) = case.governing_load(DesignMethod::Lrfd);
        // 1.2D + 1.6L = 24 + 64 = 88 should govern
        assert!((load - 88.0).abs() < 0.001);
    }

    #[test]
    fn test_load_case_serialization() {
        let case = LoadCase::new("Floor")
            .with_load(LoadType::Dead, 20.0)
            .with_load(LoadType::Live, 50.0);

        let json = serde_json::to_string(&case).unwrap();
        let parsed: LoadCase = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.label, "Floor");
        assert_eq!(parsed.get(LoadType::Dead), 20.0);
        assert_eq!(parsed.get(LoadType::Live), 50.0);
    }

    #[test]
    fn test_all_combination_results() {
        let case = LoadCase::new("Test").with_load(LoadType::Dead, 10.0);

        let results = case.all_combination_results(DesignMethod::Asd);
        assert!(!results.is_empty());

        // ASD-1 should be just D = 10
        let asd1 = results.iter().find(|(name, _)| name == "ASD-1");
        assert!(asd1.is_some());
        assert!((asd1.unwrap().1 - 10.0).abs() < 0.001);
    }
}
