//! Discrete Load System
//!
//! Provides types for representing multiple discrete loads on structural members.
//! Supports various load distribution patterns (point, uniform, partial, trapezoidal, moment)
//! with load type classification (D, L, Lr, S, W, E, H).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::load_types::LoadType;
use super::{DesignMethod, LoadCase};

// ============================================================================
// Load Distribution Types
// ============================================================================

/// How a load is distributed along a member
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LoadDistribution {
    /// Point load at a specific position
    Point {
        /// Distance from left support (ft)
        position_ft: f64,
    },

    /// Uniform load over the full span
    #[default]
    UniformFull,

    /// Uniform load over a partial span
    UniformPartial {
        /// Start position from left support (ft)
        start_ft: f64,
        /// End position from left support (ft)
        end_ft: f64,
    },

    /// Linearly varying (trapezoidal) load
    Trapezoidal {
        /// Start position from left support (ft)
        start_ft: f64,
        /// End position from left support (ft)
        end_ft: f64,
        /// Magnitude at start (plf)
        start_magnitude: f64,
        /// Magnitude at end (plf)
        end_magnitude: f64,
    },

    /// Applied moment at a specific position
    Moment {
        /// Distance from left support (ft)
        position_ft: f64,
    },
}

impl LoadDistribution {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            LoadDistribution::Point { .. } => "Point",
            LoadDistribution::UniformFull => "Uniform",
            LoadDistribution::UniformPartial { .. } => "Partial Uniform",
            LoadDistribution::Trapezoidal { .. } => "Trapezoidal",
            LoadDistribution::Moment { .. } => "Moment",
        }
    }

    /// Check if this distribution requires position input
    pub fn requires_position(&self) -> bool {
        matches!(
            self,
            LoadDistribution::Point { .. }
                | LoadDistribution::UniformPartial { .. }
                | LoadDistribution::Trapezoidal { .. }
                | LoadDistribution::Moment { .. }
        )
    }
}

// ============================================================================
// Discrete Load
// ============================================================================

/// A single discrete load entry
///
/// Represents one load applied to a structural member, with its type,
/// distribution pattern, magnitude, and optional tributary width.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteLoad {
    /// Unique identifier for this load (for UI row management).
    ///
    /// Optional on JSON input - a fresh v4 UUID is generated when absent. The
    /// GUI uses it as a stable row key; the CLI and library consumers can
    /// ignore it entirely.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// Load type (D, L, Lr, S, W, E, H)
    pub load_type: LoadType,

    /// How the load is distributed
    pub distribution: LoadDistribution,

    /// Load magnitude
    /// - For line loads (uniform, partial): plf (pounds per linear foot)
    /// - For point loads: lbs (pounds)
    /// - For moments: ft-lbs (foot-pounds)
    pub magnitude: f64,

    /// Tributary width for converting area load to line load (ft)
    /// When set, the effective magnitude = magnitude * tributary_width
    /// (input is psf, output is plf)
    #[serde(default)]
    pub tributary_width_ft: Option<f64>,

    /// User note/description for this load
    #[serde(default)]
    pub note: String,
}

impl DiscreteLoad {
    /// Create a new uniform full-span load
    pub fn uniform(load_type: LoadType, magnitude_plf: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            load_type,
            distribution: LoadDistribution::UniformFull,
            magnitude: magnitude_plf,
            tributary_width_ft: None,
            note: String::new(),
        }
    }

    /// Create a new point load
    pub fn point(load_type: LoadType, magnitude_lbs: f64, position_ft: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            load_type,
            distribution: LoadDistribution::Point { position_ft },
            magnitude: magnitude_lbs,
            tributary_width_ft: None,
            note: String::new(),
        }
    }

    /// Create a new partial uniform load
    pub fn partial_uniform(
        load_type: LoadType,
        magnitude_plf: f64,
        start_ft: f64,
        end_ft: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            load_type,
            distribution: LoadDistribution::UniformPartial { start_ft, end_ft },
            magnitude: magnitude_plf,
            tributary_width_ft: None,
            note: String::new(),
        }
    }

    /// Create a new applied moment
    pub fn moment(load_type: LoadType, magnitude_ftlbs: f64, position_ft: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            load_type,
            distribution: LoadDistribution::Moment { position_ft },
            magnitude: magnitude_ftlbs,
            tributary_width_ft: None,
            note: String::new(),
        }
    }

    /// Set tributary width and return self (builder pattern)
    pub fn with_tributary_width(mut self, width_ft: f64) -> Self {
        self.tributary_width_ft = Some(width_ft);
        self
    }

    /// Set note and return self (builder pattern)
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }

    /// Get effective magnitude accounting for tributary width
    ///
    /// For area loads (psf) with tributary width, returns line load (plf).
    /// For loads without tributary width, returns the raw magnitude.
    pub fn effective_magnitude(&self) -> f64 {
        match self.tributary_width_ft {
            Some(tw) => self.magnitude * tw,
            None => self.magnitude,
        }
    }
}

// ============================================================================
// Enhanced Load Case
// ============================================================================

/// Enhanced load configuration supporting multiple discrete loads
///
/// This replaces the simple `uniform_load_plf` field in BeamInput,
/// allowing for multiple loads of different types and distributions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnhancedLoadCase {
    /// Collection of discrete loads
    pub loads: Vec<DiscreteLoad>,

    /// Auto-calculate and include member self-weight as dead load
    pub include_self_weight: bool,

    /// User label for this load case
    pub label: String,
}

impl EnhancedLoadCase {
    /// Create a new empty load case (self-weight included by default)
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            loads: Vec::new(),
            include_self_weight: true,
            label: label.into(),
        }
    }

    /// Add a load and return self (builder pattern)
    pub fn with_load(mut self, load: DiscreteLoad) -> Self {
        self.loads.push(load);
        self
    }

    /// Enable self-weight inclusion and return self (builder pattern)
    pub fn with_self_weight(mut self) -> Self {
        self.include_self_weight = true;
        self
    }

    /// Disable self-weight inclusion and return self (builder pattern)
    pub fn without_self_weight(mut self) -> Self {
        self.include_self_weight = false;
        self
    }

    /// Add a load to this case
    pub fn add_load(&mut self, load: DiscreteLoad) {
        self.loads.push(load);
    }

    /// Remove a load by ID
    pub fn remove_load(&mut self, id: Uuid) -> Option<DiscreteLoad> {
        if let Some(pos) = self.loads.iter().position(|l| l.id == id) {
            Some(self.loads.remove(pos))
        } else {
            None
        }
    }

    /// Get a load by ID
    pub fn get_load(&self, id: Uuid) -> Option<&DiscreteLoad> {
        self.loads.iter().find(|l| l.id == id)
    }

    /// Get mutable reference to a load by ID
    pub fn get_load_mut(&mut self, id: Uuid) -> Option<&mut DiscreteLoad> {
        self.loads.iter_mut().find(|l| l.id == id)
    }

    /// Sum all uniform full-span loads of a specific type (plf)
    ///
    /// Only includes UniformFull distributions. For point loads and partial
    /// loads, use the detailed calculation methods.
    pub fn total_uniform_by_type(&self, load_type: LoadType) -> f64 {
        self.loads
            .iter()
            .filter(|l| l.load_type == load_type)
            .filter(|l| matches!(l.distribution, LoadDistribution::UniformFull))
            .map(|l| l.effective_magnitude())
            .sum()
    }

    /// Get total uniform load across all types (for simplified calculation)
    ///
    /// This sums all UniformFull loads regardless of type.
    /// For proper factored design, use `to_load_case()` with load combinations.
    pub fn total_uniform_plf(&self) -> f64 {
        self.loads
            .iter()
            .filter(|l| matches!(l.distribution, LoadDistribution::UniformFull))
            .map(|l| l.effective_magnitude())
            .sum()
    }

    /// Convert to simple LoadCase for combination calculations
    ///
    /// This aggregates all loads by type into a HashMap, suitable for
    /// applying ASCE 7 load combinations. Only includes uniform loads;
    /// point loads and moments require separate handling.
    pub fn to_load_case(&self) -> LoadCase {
        let mut lc = LoadCase::new(&self.label);
        for load_type in LoadType::ALL.iter() {
            let total = self.total_uniform_by_type(*load_type);
            if total != 0.0 {
                lc = lc.with_load(*load_type, total);
            }
        }
        lc
    }

    /// Get governing factored uniform load (plf)
    ///
    /// Applies ASCE 7 load combinations and returns the maximum factored load.
    /// Does not include self-weight - caller should add that separately.
    pub fn governing_uniform_plf(&self, method: DesignMethod) -> f64 {
        let load_case = self.to_load_case();
        let (governing, _name) = load_case.governing_load(method);
        governing
    }

    /// Check if there are any loads defined
    pub fn is_empty(&self) -> bool {
        self.loads.is_empty()
    }

    /// Get count of loads
    pub fn load_count(&self) -> usize {
        self.loads.len()
    }

    /// Get all point loads
    pub fn point_loads(&self) -> impl Iterator<Item = &DiscreteLoad> {
        self.loads
            .iter()
            .filter(|l| matches!(l.distribution, LoadDistribution::Point { .. }))
    }

    /// Get all moment loads
    pub fn moment_loads(&self) -> impl Iterator<Item = &DiscreteLoad> {
        self.loads
            .iter()
            .filter(|l| matches!(l.distribution, LoadDistribution::Moment { .. }))
    }

    /// Get all uniform loads (full and partial)
    pub fn uniform_loads(&self) -> impl Iterator<Item = &DiscreteLoad> {
        self.loads.iter().filter(|l| {
            matches!(
                l.distribution,
                LoadDistribution::UniformFull | LoadDistribution::UniformPartial { .. }
            )
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discrete_load_creation() {
        let load = DiscreteLoad::uniform(LoadType::Dead, 50.0);
        assert_eq!(load.load_type, LoadType::Dead);
        assert_eq!(load.magnitude, 50.0);
        assert!(matches!(load.distribution, LoadDistribution::UniformFull));
    }

    #[test]
    fn test_discrete_load_with_tributary() {
        let load = DiscreteLoad::uniform(LoadType::Live, 40.0).with_tributary_width(4.0);
        assert_eq!(load.magnitude, 40.0);
        assert_eq!(load.tributary_width_ft, Some(4.0));
        assert_eq!(load.effective_magnitude(), 160.0); // 40 psf * 4 ft = 160 plf
    }

    #[test]
    fn test_point_load() {
        let load = DiscreteLoad::point(LoadType::Live, 2500.0, 6.0);
        assert_eq!(load.magnitude, 2500.0);
        assert!(matches!(
            load.distribution,
            LoadDistribution::Point { position_ft } if position_ft == 6.0
        ));
    }

    #[test]
    fn test_enhanced_load_case_builder() {
        let case = EnhancedLoadCase::new("Floor Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0))
            .with_self_weight();

        assert_eq!(case.load_count(), 2);
        assert!(case.include_self_weight);
        assert_eq!(case.total_uniform_by_type(LoadType::Dead), 15.0);
        assert_eq!(case.total_uniform_by_type(LoadType::Live), 40.0);
    }

    #[test]
    fn test_to_load_case() {
        let case = EnhancedLoadCase::new("Test")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 20.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 50.0))
            .with_load(DiscreteLoad::uniform(LoadType::Snow, 30.0));

        let lc = case.to_load_case();
        assert_eq!(lc.get(LoadType::Dead), 20.0);
        assert_eq!(lc.get(LoadType::Live), 50.0);
        assert_eq!(lc.get(LoadType::Snow), 30.0);
    }

    #[test]
    fn test_governing_load_asd() {
        let case = EnhancedLoadCase::new("Test")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 20.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 50.0));

        let governing = case.governing_uniform_plf(DesignMethod::Asd);
        // ASD combo D + L = 20 + 50 = 70 plf
        assert_eq!(governing, 70.0);
    }

    #[test]
    fn test_load_distribution_serialization() {
        let dist = LoadDistribution::Point { position_ft: 6.0 };
        let json = serde_json::to_string(&dist).unwrap();
        assert!(json.contains("Point"));
        assert!(json.contains("6.0"));

        let roundtrip: LoadDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, dist);
    }

    #[test]
    fn test_enhanced_load_case_serialization() {
        let case = EnhancedLoadCase::new("Test Case")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0).with_note("Self-weight"))
            .with_load(DiscreteLoad::point(LoadType::Live, 2500.0, 6.0))
            .with_self_weight();

        let json = serde_json::to_string_pretty(&case).unwrap();
        let roundtrip: EnhancedLoadCase = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip.load_count(), 2);
        assert!(roundtrip.include_self_weight);
        assert_eq!(roundtrip.label, "Test Case");
    }

    #[test]
    fn test_remove_load() {
        let mut case = EnhancedLoadCase::new("Test")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0));

        let id = case.loads[0].id;
        let removed = case.remove_load(id);
        assert!(removed.is_some());
        assert_eq!(case.load_count(), 1);
    }

    #[test]
    fn test_point_loads_iterator() {
        let case = EnhancedLoadCase::new("Test")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
            .with_load(DiscreteLoad::point(LoadType::Live, 2500.0, 6.0))
            .with_load(DiscreteLoad::point(LoadType::Live, 1000.0, 3.0));

        let points: Vec<_> = case.point_loads().collect();
        assert_eq!(points.len(), 2);
    }
}
