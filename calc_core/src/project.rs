//! # Project Data Structures
//!
//! The `Project` struct is the root container for all calculation data.
//! Projects serialize to `.stf` (Stratify) files as human-readable JSON.
//!
//! ## Structure
//!
//! ```text
//! Project
//! ├── meta: ProjectMetadata (version, engineer, job info, timestamps)
//! ├── settings: GlobalSettings (code year, defaults)
//! └── items: HashMap<Uuid, CalculationItem> (all calculations)
//! ```
//!
//! ## Example
//!
//! ```rust
//! use calc_core::project::Project;
//!
//! let mut project = Project::new("Jane Engineer", "25-042", "ACME Corp");
//!
//! // Serialize to JSON
//! let json = serde_json::to_string_pretty(&project).unwrap();
//!
//! // Save to file (see file_io module for atomic saves)
//! std::fs::write("project.stf", &json).unwrap();
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::calculations::CalculationItem;
use crate::loads::DesignMethod;

/// Current schema version for .stf files
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Root project container.
///
/// This is the top-level struct that gets serialized to `.stf` files.
/// Items are stored in a flat UUID-keyed map for O(1) lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project metadata (version, engineer, job info)
    pub meta: ProjectMetadata,

    /// Global settings (code year, default materials)
    pub settings: GlobalSettings,

    /// All calculation items, keyed by UUID
    ///
    /// Using a HashMap instead of a Vec provides:
    /// - O(1) lookup for dependencies (e.g., "beam rests on column")
    /// - No duplicate ID issues
    /// - Stable references when items are reordered
    pub items: HashMap<Uuid, CalculationItem>,
}

impl Project {
    /// Create a new empty project.
    ///
    /// # Arguments
    ///
    /// * `engineer` - Name of the responsible engineer
    /// * `job_id` - Job/project number (e.g., "25-001")
    /// * `client` - Client name
    ///
    /// # Example
    ///
    /// ```rust
    /// use calc_core::project::Project;
    ///
    /// let project = Project::new("John Doe", "25-001", "Client Corp");
    /// assert_eq!(project.meta.engineer, "John Doe");
    /// ```
    pub fn new(
        engineer: impl Into<String>,
        job_id: impl Into<String>,
        client: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Project {
            meta: ProjectMetadata {
                version: SCHEMA_VERSION.to_string(),
                engineer: engineer.into(),
                job_id: job_id.into(),
                client: client.into(),
                created: now,
                modified: now,
            },
            settings: GlobalSettings::default(),
            items: HashMap::new(),
        }
    }

    /// Add a calculation item to the project.
    ///
    /// Returns the UUID assigned to the item.
    ///
    /// # Example
    ///
    /// ```rust
    /// use calc_core::project::Project;
    /// use calc_core::calculations::{CalculationItem, ContinuousBeamInput};
    /// use calc_core::materials::{Material, WoodSpecies, WoodGrade, WoodMaterial};
    /// use calc_core::loads::{EnhancedLoadCase, DiscreteLoad, LoadType};
    ///
    /// let mut project = Project::new("Engineer", "25-001", "Client");
    ///
    /// let load_case = EnhancedLoadCase::new("Floor")
    ///     .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
    ///     .with_load(DiscreteLoad::uniform(LoadType::Live, 100.0));
    ///
    /// let beam = ContinuousBeamInput::simple_span(
    ///     "B-1",
    ///     12.0,
    ///     1.5,
    ///     9.25,
    ///     Material::SawnLumber(WoodMaterial::new(WoodSpecies::DouglasFirLarch, WoodGrade::No2)),
    ///     load_case,
    /// );
    ///
    /// let id = project.add_item(CalculationItem::Beam(beam));
    /// assert!(project.items.contains_key(&id));
    /// ```
    pub fn add_item(&mut self, item: CalculationItem) -> Uuid {
        let id = Uuid::new_v4();
        self.items.insert(id, item);
        self.touch();
        id
    }

    /// Remove a calculation item by UUID.
    ///
    /// Returns the removed item if it existed.
    pub fn remove_item(&mut self, id: &Uuid) -> Option<CalculationItem> {
        let item = self.items.remove(id);
        if item.is_some() {
            self.touch();
        }
        item
    }

    /// Get a calculation item by UUID.
    pub fn get_item(&self, id: &Uuid) -> Option<&CalculationItem> {
        self.items.get(id)
    }

    /// Get a mutable reference to a calculation item by UUID.
    ///
    /// Note: This method updates the modified timestamp when an item is found.
    /// The caller should be aware that getting a mutable reference marks
    /// the project as modified.
    pub fn get_item_mut(&mut self, id: &Uuid) -> Option<&mut CalculationItem> {
        if self.items.contains_key(id) {
            self.meta.modified = Utc::now();
            self.items.get_mut(id)
        } else {
            None
        }
    }

    /// Update the modified timestamp.
    pub fn touch(&mut self) {
        self.meta.modified = Utc::now();
    }

    /// Get all items of a specific type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use calc_core::project::Project;
    /// use calc_core::calculations::CalculationItem;
    ///
    /// let project = Project::new("Engineer", "25-001", "Client");
    /// let beams: Vec<_> = project.items.values()
    ///     .filter_map(|item| match item {
    ///         CalculationItem::Beam(b) => Some(b),
    ///         _ => None,
    ///     })
    ///     .collect();
    /// ```
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

impl Default for Project {
    fn default() -> Self {
        Project::new("", "", "")
    }
}

/// Project metadata stored in the file header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Schema version (for migration compatibility)
    pub version: String,

    /// Name of the responsible engineer
    pub engineer: String,

    /// Job/project number
    pub job_id: String,

    /// Client name
    pub client: String,

    /// When the project was created
    pub created: DateTime<Utc>,

    /// When the project was last modified
    pub modified: DateTime<Utc>,
}

/// Global project settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Building code year (e.g., "IBC2024")
    pub code: String,

    /// Seismic design category (A through F)
    pub seismic_design_category: Option<String>,

    /// Risk category (I through IV)
    pub risk_category: RiskCategory,

    /// Default materials for new calculations
    pub default_materials: DefaultMaterials,

    /// Design method (ASD or LRFD) for load combinations
    pub design_method: DesignMethod,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        GlobalSettings {
            code: "IBC2024".to_string(),
            seismic_design_category: None,
            risk_category: RiskCategory::II,
            default_materials: DefaultMaterials::default(),
            design_method: DesignMethod::Asd,
        }
    }
}

/// Risk category per ASCE 7
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskCategory {
    I,
    #[default]
    II,
    III,
    IV,
}

/// Default materials for new calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultMaterials {
    /// Default wood species/grade string (e.g., "DF-L No.2")
    pub wood: String,

    /// Default steel grade (e.g., "A992")
    pub steel: String,

    /// Default concrete f'c in psi (e.g., 3000)
    pub concrete_fc_psi: u32,
}

impl Default for DefaultMaterials {
    fn default() -> Self {
        DefaultMaterials {
            wood: "DF-L No.2".to_string(),
            steel: "A992".to_string(),
            concrete_fc_psi: 3000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_creation() {
        let project = Project::new("John Doe", "25-001", "Acme Corp");
        assert_eq!(project.meta.engineer, "John Doe");
        assert_eq!(project.meta.job_id, "25-001");
        assert_eq!(project.meta.client, "Acme Corp");
        assert_eq!(project.meta.version, SCHEMA_VERSION);
    }

    #[test]
    fn test_project_serialization() {
        let project = Project::new("Jane Engineer", "25-042", "Test Client");
        let json = serde_json::to_string_pretty(&project).unwrap();

        // Should contain key fields
        assert!(json.contains("Jane Engineer"));
        assert!(json.contains("25-042"));
        assert!(json.contains("IBC2024"));

        // Roundtrip
        let roundtrip: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.meta.engineer, "Jane Engineer");
    }

    #[test]
    fn test_add_remove_item() {
        use crate::calculations::{CalculationItem, ContinuousBeamInput};
        use crate::loads::{DiscreteLoad, EnhancedLoadCase, LoadType};
        use crate::materials::{Material, WoodGrade, WoodMaterial, WoodSpecies};

        let mut project = Project::new("Engineer", "25-001", "Client");

        let load_case = EnhancedLoadCase::new("Test Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 50.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 100.0));

        let beam = ContinuousBeamInput::simple_span(
            "B-1",
            12.0,
            1.5,
            9.25,
            Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            load_case,
        );

        let id = project.add_item(CalculationItem::Beam(beam));
        assert_eq!(project.item_count(), 1);
        assert!(project.get_item(&id).is_some());

        let removed = project.remove_item(&id);
        assert!(removed.is_some());
        assert_eq!(project.item_count(), 0);
    }

    #[test]
    fn test_risk_category_serialization() {
        let cat = RiskCategory::III;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"III\"");

        let roundtrip: RiskCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, RiskCategory::III);
    }
}
