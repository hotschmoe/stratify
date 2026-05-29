//! UI module for Stratify GUI
//!
//! This module organizes the GUI into panels and components as described in GUI_LAYOUT.md.
//!
//! # Panel Structure
//! - `toolbar` - File operations (New, Open, Save, Export PDF), Settings
//! - `items_panel` - Left sidebar: Project Info, Wood Beams list, future sections
//! - `input_panel` - Center panel: dispatches to input_* child modules
//! - `results_panel` - Right panel: dispatches to result_* child modules
//! - `status_bar` - Bottom status messages
//!
//! # Input Panel Children
//! - `input_project_info` - Engineer, Job ID, Client fields
//! - `input_wood_beam` - Beam properties, loads, material, NDS factors
//!
//! # Results Panel Children
//! - `result_project_info` - Project summary / cover sheet preview
//! - `result_wood_beam` - Pass/Fail, demand/capacity, diagrams
//!
//! # Shared Components
//! - `shared/diagrams` - Canvas drawing for shear/moment/deflection diagrams

// Top-level panels
pub mod input_panel;
pub mod items_panel;
pub mod modal;
pub mod results_panel;
pub mod status_bar;
pub mod toolbar;

// Input panel children
pub mod input_project_info;
pub mod input_wood_beam;

// Results panel children
pub mod result_project_info;
pub mod result_wood_beam;

// Shared components
pub mod shared;

// Note: Functions are accessed via module paths (e.g., ui::toolbar::view_toolbar)
// Re-exports available if needed:
// pub use toolbar::view_toolbar;
// pub use items_panel::view_items_panel;
// pub use input_panel::view_input_panel;
// pub use results_panel::view_results_panel;
// pub use status_bar::view_status_bar;
