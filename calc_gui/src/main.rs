//! # Stratify GUI Application
//!
//! Full-featured graphical interface for structural engineering calculations.
//! Built with Iced framework for cross-platform support (Windows, macOS, Linux, WASM).

use std::collections::HashSet;
use std::path::PathBuf;

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::canvas;
use iced::widget::{column, container, operation, row, rule, stack, Space};
use iced::{event, Element, Event, Font, Length, Subscription, Task, Theme};
use uuid::Uuid;

use calc_core::calculations::continuous_beam::{
    calculate_continuous, ContinuousBeamInput, ContinuousBeamResult, SpanSegment, SupportType,
};
use calc_core::calculations::CalculationItem;
#[cfg(not(target_arch = "wasm32"))]
use calc_core::file_io::{save_project, FileLock};
use calc_core::loads::{DesignMethod, DiscreteLoad, EnhancedLoadCase, LoadDistribution, LoadType};
use calc_core::materials::{
    GlulamLayup, GlulamMaterial, GlulamStressClass, LumberSize, LvlGrade, LvlMaterial, Material,
    PlyCount, PslGrade, PslMaterial, WoodGrade, WoodMaterial, WoodSpecies,
};
use calc_core::nds_factors::{
    AdjustmentFactors, FlatUse, Incising, LoadDuration, RepetitiveMember, Temperature, WetService,
};
use calc_core::pdf::render_project_pdf;
use calc_core::project::Project;
use calc_core::section_deductions::{NotchLocation, SectionDeductions};

mod ui;

#[cfg(not(target_arch = "wasm32"))]
mod update;

use ui::modal::{ModalType, PendingAction};

// ============================================================================
// UI Types
// ============================================================================

/// Material type for UI selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialType {
    #[default]
    SawnLumber,
    Glulam,
    Lvl,
    Psl,
}

impl MaterialType {
    pub const ALL: [MaterialType; 4] = [
        MaterialType::SawnLumber,
        MaterialType::Glulam,
        MaterialType::Lvl,
        MaterialType::Psl,
    ];
}

impl std::fmt::Display for MaterialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialType::SawnLumber => write!(f, "Sawn Lumber"),
            MaterialType::Glulam => write!(f, "Glulam"),
            MaterialType::Lvl => write!(f, "LVL"),
            MaterialType::Psl => write!(f, "PSL"),
        }
    }
}

/// Distribution type for UI dropdown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistributionType {
    #[default]
    UniformFull,
    Point,
    UniformPartial,
}

impl DistributionType {
    pub const ALL: [DistributionType; 3] = [
        DistributionType::UniformFull,
        DistributionType::Point,
        DistributionType::UniformPartial,
    ];
}

impl std::fmt::Display for DistributionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistributionType::UniformFull => write!(f, "Uniform"),
            DistributionType::Point => write!(f, "Point"),
            DistributionType::UniformPartial => write!(f, "Partial"),
        }
    }
}

/// Left panel section identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemSection {
    ProjectInfo,
    WoodBeams,
    WoodColumns,
    // Future sections (disabled in UI)
    ContinuousFootings,
    SpreadFootings,
    CantileverWalls,
    RestrainedWalls,
}

/// Categories of items that can be added to a project via the category picker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemCategory {
    // Beams
    WoodBeams,
    // SteelBeams,      // Future
    // ContinuousBeams, // Future

    // Posts/Columns
    // WoodColumns,     // Future
    // SteelColumns,    // Future

    // Foundations
    // SpreadFootings,  // Future
    // CombinedFootings,// Future

    // Retaining
    CantileverRetainingWalls,
    // GravityWalls,    // Future

    // Misc
    // Connections,     // Future
}

impl ItemCategory {
    /// All currently implemented categories
    pub const IMPLEMENTED: &'static [ItemCategory] = &[ItemCategory::WoodBeams];

    /// All categories (including future/unimplemented)
    pub const ALL: &'static [ItemCategory] = &[
        ItemCategory::WoodBeams,
        ItemCategory::CantileverRetainingWalls,
    ];

    /// Display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            ItemCategory::WoodBeams => "Wood Beams",
            ItemCategory::CantileverRetainingWalls => "Cantilever Retaining Walls",
        }
    }

    /// Whether this category is implemented
    pub fn is_implemented(&self) -> bool {
        matches!(self, ItemCategory::WoodBeams)
    }

    /// Get the corresponding ItemSection for this category
    pub fn to_section(&self) -> ItemSection {
        match self {
            ItemCategory::WoodBeams => ItemSection::WoodBeams,
            ItemCategory::CantileverRetainingWalls => ItemSection::CantileverWalls,
        }
    }

    /// Group name for organizing in tabs
    pub fn group(&self) -> &'static str {
        match self {
            ItemCategory::WoodBeams => "Beams",
            ItemCategory::CantileverRetainingWalls => "Retaining",
        }
    }
}

/// What is currently being edited in the middle panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSelection {
    /// No selection - show welcome/instructions
    None,
    /// Editing project info
    ProjectInfo,
    /// Editing a beam (existing or new)
    Beam(Option<Uuid>),
}

/// Which panel divider is being dragged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerType {
    /// Divider between items panel and input panel
    ItemsInput,
    /// Divider between input panel and results panel
    InputResults,
}

/// Input panel tabs for organizing beam inputs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputTab {
    /// Beam description: label, span, dimensions
    #[default]
    Description,
    /// Member selection: material, NDS factors, section deductions
    MemberSelection,
    /// Loads: load table
    Loads,
}

impl InputTab {
    pub const ALL: [InputTab; 3] = [
        InputTab::Description,
        InputTab::MemberSelection,
        InputTab::Loads,
    ];
}

impl std::fmt::Display for InputTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputTab::Description => write!(f, "Description"),
            InputTab::MemberSelection => write!(f, "Member"),
            InputTab::Loads => write!(f, "Loads"),
        }
    }
}

/// Results panel tabs for organizing output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResultsTab {
    /// Numerical calculation results
    #[default]
    Results,
    /// Visual diagrams (V, M, δ)
    Diagrams,
}

impl ResultsTab {
    pub const ALL: [ResultsTab; 2] = [ResultsTab::Results, ResultsTab::Diagrams];
}

impl std::fmt::Display for ResultsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultsTab::Results => write!(f, "Results"),
            ResultsTab::Diagrams => write!(f, "Diagrams"),
        }
    }
}

/// A row in the span table (for multi-span beams)
#[derive(Debug, Clone)]
pub struct SpanTableRow {
    pub id: Uuid,
    pub length_ft: String,
    pub left_support: SupportType,
}

impl SpanTableRow {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            length_ft: "12.0".to_string(),
            left_support: SupportType::Pinned,
        }
    }

    fn from_span(span: &SpanSegment, support: SupportType) -> Self {
        Self {
            id: span.id,
            length_ft: span.length_ft.to_string(),
            left_support: support,
        }
    }
}

/// A row in the load table (editable UI state)
#[derive(Debug, Clone)]
pub struct LoadTableRow {
    pub id: Uuid,
    pub load_type: LoadType,
    pub distribution: DistributionType,
    pub magnitude: String,
    pub position: String,
    pub start_ft: String,
    pub end_ft: String,
    pub tributary_width: String,
}

impl LoadTableRow {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            load_type: LoadType::Dead,
            distribution: DistributionType::UniformFull,
            magnitude: "0.0".to_string(),
            position: String::new(),
            start_ft: "0.0".to_string(),
            end_ft: String::new(),
            tributary_width: String::new(),
        }
    }

    fn to_discrete_load(&self, span_ft: f64) -> Option<DiscreteLoad> {
        let magnitude: f64 = self.magnitude.parse().ok()?;
        let tributary: Option<f64> = if self.tributary_width.is_empty() {
            None
        } else {
            Some(self.tributary_width.parse().ok()?)
        };

        let mut load = match self.distribution {
            DistributionType::UniformFull => DiscreteLoad::uniform(self.load_type, magnitude),
            DistributionType::Point => {
                let pos: f64 = self.position.parse().unwrap_or(span_ft / 2.0);
                DiscreteLoad::point(self.load_type, magnitude, pos)
            }
            DistributionType::UniformPartial => {
                let start: f64 = self.start_ft.parse().unwrap_or(0.0);
                let end: f64 = self.end_ft.parse().unwrap_or(span_ft);
                DiscreteLoad::partial_uniform(self.load_type, magnitude, start, end)
            }
        };

        if let Some(tw) = tributary {
            load = load.with_tributary_width(tw);
        }

        Some(load)
    }

    fn from_discrete_load(load: &DiscreteLoad) -> Self {
        let (distribution, position, start_ft, end_ft) = match &load.distribution {
            LoadDistribution::UniformFull => (
                DistributionType::UniformFull,
                String::new(),
                String::new(),
                String::new(),
            ),
            LoadDistribution::Point { position_ft } => (
                DistributionType::Point,
                position_ft.to_string(),
                String::new(),
                String::new(),
            ),
            LoadDistribution::UniformPartial { start_ft, end_ft } => (
                DistributionType::UniformPartial,
                String::new(),
                start_ft.to_string(),
                end_ft.to_string(),
            ),
            _ => (
                DistributionType::UniformFull,
                String::new(),
                String::new(),
                String::new(),
            ),
        };

        let tributary_width = load
            .tributary_width_ft
            .map(|t| t.to_string())
            .unwrap_or_default();

        Self {
            id: load.id,
            load_type: load.load_type,
            distribution,
            magnitude: load.magnitude.to_string(),
            position,
            start_ft,
            end_ft,
            tributary_width,
        }
    }
}

// Embed BerkeleyMono font at compile time
const BERKELEY_MONO: &[u8] =
    include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Regular.otf");
const BERKELEY_MONO_BOLD: &[u8] =
    include_bytes!("../../assets/fonts/BerkleyMono/BerkeleyMono-Bold.otf");

/// Convert a positive f64 to String, or return empty string if zero or negative
fn positive_or_empty(value: f64) -> String {
    if value > 0.0 {
        value.to_string()
    } else {
        String::new()
    }
}

fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    iced::application(App::new, App::update, App::view)
        .title(App::window_title)
        .subscription(App::subscription)
        .theme(App::theme)
        .window_size((1200.0, 750.0))
        .font(BERKELEY_MONO)
        .font(BERKELEY_MONO_BOLD)
        .default_font(Font::with_name("Berkeley Mono"))
        .run()
}

// ============================================================================
// Application State
// ============================================================================

pub struct App {
    // Project data
    pub project: Project,

    // File management
    pub current_file: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    pub file_lock: Option<FileLock>,
    pub is_modified: bool,
    pub lock_holder: Option<String>,

    // Left panel state
    pub collapsed_sections: HashSet<ItemSection>,
    pub enabled_categories: HashSet<ItemCategory>,

    // Current editor selection
    pub selection: EditorSelection,

    // Beam input fields
    pub beam_label: String,
    pub span_ft: String,
    pub width_in: String,
    pub depth_in: String,

    // Multi-span configuration
    pub span_table: Vec<SpanTableRow>,
    pub right_end_support: SupportType,
    pub multi_span_mode: bool,

    // Load table
    pub load_table: Vec<LoadTableRow>,
    pub include_self_weight: bool,

    // Material selection
    pub selected_material_type: MaterialType,
    pub selected_species: Option<WoodSpecies>,
    pub selected_grade: Option<WoodGrade>,
    pub selected_lumber_size: LumberSize,
    pub selected_ply_count: PlyCount,
    pub selected_glulam_class: Option<GlulamStressClass>,
    pub selected_glulam_layup: Option<GlulamLayup>,
    pub selected_lvl_grade: Option<LvlGrade>,
    pub selected_psl_grade: Option<PslGrade>,

    // NDS Adjustment Factors
    pub selected_load_duration: LoadDuration,
    pub selected_wet_service: WetService,
    pub selected_temperature: Temperature,
    pub selected_incising: Incising,
    pub selected_repetitive_member: RepetitiveMember,
    pub selected_flat_use: FlatUse,
    pub compression_edge_braced: bool,

    // Section deductions
    pub selected_notch_location: NotchLocation,
    pub notch_depth_left: String,
    pub notch_depth_right: String,
    pub hole_diameter: String,
    pub hole_count: String,

    // Calculation results
    pub calc_input: Option<ContinuousBeamInput>,
    pub result: Option<ContinuousBeamResult>,
    pub error_message: Option<String>,
    pub diagram_cache: canvas::Cache,

    // Status message
    pub status: String,

    // Settings
    pub dark_mode: bool,
    pub settings_menu_open: bool,

    // Modal state
    pub active_modal: Option<ModalType>,

    // Panel resizing
    pub items_panel_width: f32,
    pub input_panel_ratio: f32, // Ratio of (input_panel / (input_panel + results_panel))
    pub dragging_divider: Option<DividerType>,
    pub drag_start_x: f32,
    pub drag_start_value: f32, // Width for items panel, ratio for input panel

    // Input panel tabs
    pub selected_input_tab: InputTab,

    // Results panel tabs
    pub selected_results_tab: ResultsTab,

    // Update checker (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub update_status: UpdateStatus,
}

/// Status of the update checker
#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    #[default]
    NotChecked,
    Checking,
    UpToDate,
    UpdateAvailable {
        version: String,
        download_url: String,
        html_url: String,
    },
    Failed(String),
}

impl Default for App {
    fn default() -> Self {
        let default_loads = vec![
            LoadTableRow {
                id: Uuid::new_v4(),
                load_type: LoadType::Dead,
                distribution: DistributionType::UniformFull,
                magnitude: "15.0".to_string(),
                position: String::new(),
                start_ft: "0.0".to_string(),
                end_ft: String::new(),
                tributary_width: String::new(),
            },
            LoadTableRow {
                id: Uuid::new_v4(),
                load_type: LoadType::Live,
                distribution: DistributionType::UniformFull,
                magnitude: "40.0".to_string(),
                position: String::new(),
                start_ft: "0.0".to_string(),
                end_ft: String::new(),
                tributary_width: String::new(),
            },
        ];

        App {
            project: Project::new("Engineer", "25-001", "Client"),
            current_file: None,
            #[cfg(not(target_arch = "wasm32"))]
            file_lock: None,
            is_modified: false,
            lock_holder: None,
            collapsed_sections: HashSet::new(),
            enabled_categories: HashSet::new(), // Start empty; user adds categories via picker
            selection: EditorSelection::ProjectInfo,
            beam_label: "B-1".to_string(),
            span_ft: "12.0".to_string(),
            width_in: "1.5".to_string(),
            depth_in: "9.25".to_string(),
            span_table: vec![SpanTableRow {
                id: Uuid::new_v4(),
                length_ft: "12.0".to_string(),
                left_support: SupportType::Pinned,
            }],
            right_end_support: SupportType::Roller,
            multi_span_mode: false,
            load_table: default_loads,
            include_self_weight: true,
            selected_material_type: MaterialType::SawnLumber,
            selected_species: Some(WoodSpecies::DouglasFirLarch),
            selected_grade: Some(WoodGrade::No2),
            selected_lumber_size: LumberSize::L2x10,
            selected_ply_count: PlyCount::Single,
            selected_glulam_class: Some(GlulamStressClass::F24_V4),
            selected_glulam_layup: Some(GlulamLayup::Unbalanced),
            selected_lvl_grade: Some(LvlGrade::Standard),
            selected_psl_grade: Some(PslGrade::Standard),
            selected_load_duration: LoadDuration::Normal,
            selected_wet_service: WetService::Dry,
            selected_temperature: Temperature::Normal,
            selected_incising: Incising::None,
            selected_repetitive_member: RepetitiveMember::Single,
            selected_flat_use: FlatUse::Normal,
            compression_edge_braced: true,
            selected_notch_location: NotchLocation::None,
            notch_depth_left: String::new(),
            notch_depth_right: String::new(),
            hole_diameter: String::new(),
            hole_count: String::new(),
            calc_input: None,
            result: None,
            error_message: None,
            diagram_cache: canvas::Cache::default(),
            status: "Ready - New Project".to_string(),
            dark_mode: false,
            settings_menu_open: false,
            active_modal: None,
            items_panel_width: 170.0,
            input_panel_ratio: 0.5,
            dragging_divider: None,
            drag_start_x: 0.0,
            drag_start_value: 0.0,
            selected_input_tab: InputTab::default(),
            selected_results_tab: ResultsTab::default(),
            #[cfg(not(target_arch = "wasm32"))]
            update_status: UpdateStatus::default(),
        }
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = Self::default();

        // On native, start background update check
        #[cfg(not(target_arch = "wasm32"))]
        let task = Task::perform(update::check_for_updates(), Message::UpdateCheckComplete);

        #[cfg(target_arch = "wasm32")]
        let task = Task::none();

        (app, task)
    }

    fn theme(&self) -> Theme {
        if self.dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn window_title(&self) -> String {
        let file_name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");

        let modified = if self.is_modified { " *" } else { "" };
        let read_only = if self.lock_holder.is_some() {
            " [Read-Only]"
        } else {
            ""
        };

        format!("{}{}{} - Stratify", file_name, modified, read_only)
    }

    fn can_edit(&self) -> bool {
        self.lock_holder.is_none()
    }

    pub fn selected_beam_id(&self) -> Option<Uuid> {
        match self.selection {
            EditorSelection::Beam(Some(id)) => Some(id),
            _ => None,
        }
    }

    /// Build Material from current UI selections, if valid
    fn build_material(&self) -> Option<Material> {
        match self.selected_material_type {
            MaterialType::SawnLumber => {
                let species = self.selected_species?;
                let grade = self.selected_grade?;
                Some(Material::SawnLumber(WoodMaterial::new(species, grade)))
            }
            MaterialType::Glulam => {
                let stress_class = self.selected_glulam_class?;
                let layup = self.selected_glulam_layup?;
                Some(Material::Glulam(GlulamMaterial::new(stress_class, layup)))
            }
            MaterialType::Lvl => {
                let grade = self.selected_lvl_grade?;
                Some(Material::Lvl(LvlMaterial::new(grade)))
            }
            MaterialType::Psl => {
                let grade = self.selected_psl_grade?;
                Some(Material::Psl(PslMaterial::new(grade)))
            }
        }
    }

    /// Build AdjustmentFactors from current UI selections
    fn build_adjustment_factors(&self) -> AdjustmentFactors {
        AdjustmentFactors {
            load_duration: self.selected_load_duration,
            wet_service: self.selected_wet_service,
            temperature: self.selected_temperature,
            incising: self.selected_incising,
            repetitive_member: self.selected_repetitive_member,
            flat_use: self.selected_flat_use,
            compression_edge_braced: self.compression_edge_braced,
            unbraced_length_in: None,
        }
    }

    /// Build SectionDeductions from current UI selections
    fn build_section_deductions(&self) -> SectionDeductions {
        SectionDeductions {
            notch_location: self.selected_notch_location,
            notch_depth_left_in: self.notch_depth_left.parse().unwrap_or(0.0),
            notch_depth_right_in: self.notch_depth_right.parse().unwrap_or(0.0),
            hole_diameter_in: self.hole_diameter.parse().unwrap_or(0.0),
            hole_count: self.hole_count.parse().unwrap_or(0),
        }
    }

    /// Calculate total beam length from span table (for multi-span) or single span
    fn total_beam_length(&self, single_span_ft: f64) -> f64 {
        if self.multi_span_mode && self.span_table.len() > 1 {
            self.span_table
                .iter()
                .filter_map(|s| s.length_ft.parse::<f64>().ok())
                .filter(|&v| v > 0.0)
                .sum()
        } else {
            single_span_ft
        }
    }
}

// ============================================================================
// Messages
// ============================================================================

#[derive(Debug, Clone)]
pub enum Message {
    // File operations
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,

    // Project info changes
    EngineerNameChanged(String),
    JobIdChanged(String),
    ClientChanged(String),

    // Left panel section toggle
    ToggleSection(ItemSection),

    // Editor selection
    SelectProjectInfo,
    SelectBeam(Uuid),
    CreateBeam,

    // Beam input field changes
    BeamLabelChanged(String),
    SpanChanged(String),
    WidthChanged(String),
    DepthChanged(String),

    // Multi-span operations
    ToggleMultiSpanMode,
    AddSpan,
    RemoveSpan(Uuid),
    SpanLengthChanged(Uuid, String),
    SpanLeftSupportChanged(Uuid, SupportType),
    RightEndSupportChanged(SupportType),

    // Load table operations
    AddLoad,
    RemoveLoad(Uuid),
    LoadTypeChanged(Uuid, LoadType),
    LoadDistributionChanged(Uuid, DistributionType),
    LoadMagnitudeChanged(Uuid, String),
    LoadPositionChanged(Uuid, String),
    LoadStartChanged(Uuid, String),
    LoadEndChanged(Uuid, String),
    LoadTributaryChanged(Uuid, String),
    IncludeSelfWeightToggled(bool),

    // Material selection
    MaterialTypeSelected(MaterialType),
    SpeciesSelected(WoodSpecies),
    GradeSelected(WoodGrade),
    LumberSizeSelected(LumberSize),
    PlyCountSelected(PlyCount),
    GlulamClassSelected(GlulamStressClass),
    GlulamLayupSelected(GlulamLayup),
    LvlGradeSelected(LvlGrade),
    PslGradeSelected(PslGrade),

    // NDS Adjustment Factors
    LoadDurationSelected(LoadDuration),
    WetServiceSelected(WetService),
    TemperatureSelected(Temperature),
    IncisingSelected(Incising),
    RepetitiveMemberSelected(RepetitiveMember),
    FlatUseSelected(FlatUse),
    CompressionBracedToggled(bool),

    // Section Deductions
    NotchLocationSelected(NotchLocation),
    NotchDepthLeftChanged(String),
    NotchDepthRightChanged(String),
    HoleDiameterChanged(String),
    HoleCountChanged(String),

    // Actions
    DeleteSelectedBeam,
    ExportPdf,

    // Keyboard events
    KeyPressed(Key, Modifiers),
    FocusNext,
    FocusPrevious,

    // Settings
    ToggleSettingsMenu,
    ToggleDarkMode,

    // Modal interactions
    ModalSave,
    ModalDontSave,
    ModalCancel,

    // Category picker
    OpenCategoryPicker,
    AddCategory(ItemCategory),
    CloseCategoryPicker,

    // Panel resizing
    DividerDragStart(DividerType, f32), // divider type, initial x position
    DividerDragMove(f32),               // current x position
    DividerDragEnd,

    // Input panel tabs
    SelectInputTab(InputTab),

    // Results panel tabs
    SelectResultsTab(ResultsTab),

    // Async file operations
    FileOpenComplete(Result<(String, Vec<u8>), String>),
    FileSaveComplete(Result<String, String>),
    PdfExportComplete(Result<String, String>),

    // Update checking (native only)
    #[cfg(not(target_arch = "wasm32"))]
    CheckForUpdates,
    #[cfg(not(target_arch = "wasm32"))]
    UpdateCheckComplete(update::UpdateCheckResult),
    #[cfg(not(target_arch = "wasm32"))]
    OpenUpdateUrl(String),
}

// ============================================================================
// Subscriptions
// ============================================================================

impl App {
    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if key == Key::Named(keyboard::key::Named::Tab) {
                    if modifiers.shift() {
                        return Some(Message::FocusPrevious);
                    } else {
                        return Some(Message::FocusNext);
                    }
                }
                Some(Message::KeyPressed(key, modifiers))
            }
            // Always listen for mouse events - filtering happens in update()
            Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::DividerDragMove(position.x))
            }
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(Message::DividerDragEnd)
            }
            _ => None,
        })
    }
}

// ============================================================================
// Update Logic
// ============================================================================

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FocusNext => return operation::focus_next(),
            Message::FocusPrevious => return operation::focus_previous(),

            Message::KeyPressed(key, modifiers) => {
                if modifiers.control() {
                    match key.as_ref() {
                        Key::Character("s") => {
                            if modifiers.shift() {
                                return self.save_project_as();
                            } else {
                                return self.save_project();
                            }
                        }
                        Key::Character("o") => return self.open_project(),
                        Key::Character("n") => self.new_project(),
                        _ => {}
                    }
                }
            }

            Message::NewProject => self.new_project(),
            Message::OpenProject => return self.open_project(),
            Message::SaveProject => return self.save_project(),
            Message::SaveProjectAs => return self.save_project_as(),

            Message::EngineerNameChanged(value) => {
                if self.can_edit() {
                    self.project.meta.engineer = value;
                    self.mark_modified();
                }
            }
            Message::JobIdChanged(value) => {
                if self.can_edit() {
                    self.project.meta.job_id = value;
                    self.mark_modified();
                }
            }
            Message::ClientChanged(value) => {
                if self.can_edit() {
                    self.project.meta.client = value;
                    self.mark_modified();
                }
            }

            Message::ToggleSection(section) => {
                // HashSet::insert returns false if already present, so we can use
                // remove-then-insert pattern. However, toggle is clearer here.
                if !self.collapsed_sections.remove(&section) {
                    self.collapsed_sections.insert(section);
                }
            }

            Message::SelectProjectInfo => {
                self.selection = EditorSelection::ProjectInfo;
                self.result = None;
                self.error_message = None;
            }
            Message::SelectBeam(id) => self.select_beam(id),
            Message::CreateBeam => self.create_beam(),

            Message::BeamLabelChanged(value) => {
                self.beam_label = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::SpanChanged(value) => {
                self.span_ft = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::WidthChanged(value) => {
                self.width_in = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::DepthChanged(value) => {
                self.depth_in = value;
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::ToggleMultiSpanMode => {
                self.multi_span_mode = !self.multi_span_mode;
                if !self.multi_span_mode && !self.span_table.is_empty() {
                    self.span_ft = self.span_table[0].length_ft.clone();
                } else if self.multi_span_mode
                    && !self.span_ft.is_empty()
                    && !self.span_table.is_empty()
                {
                    self.span_table[0].length_ft = self.span_ft.clone();
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::AddSpan => {
                self.span_table.push(SpanTableRow::new());
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::RemoveSpan(id) => {
                if self.span_table.len() > 1 {
                    self.span_table.retain(|s| s.id != id);
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::SpanLengthChanged(id, value) => {
                if let Some(span) = self.span_table.iter_mut().find(|s| s.id == id) {
                    span.length_ft = value;
                }
                if !self.span_table.is_empty() && self.span_table[0].id == id {
                    self.span_ft = self.span_table[0].length_ft.clone();
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::SpanLeftSupportChanged(id, support) => {
                if let Some(span) = self.span_table.iter_mut().find(|s| s.id == id) {
                    span.left_support = support;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::RightEndSupportChanged(support) => {
                self.right_end_support = support;
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::AddLoad => {
                self.load_table.push(LoadTableRow::new());
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::RemoveLoad(id) => {
                self.load_table.retain(|row| row.id != id);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadTypeChanged(id, load_type) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.load_type = load_type;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadDistributionChanged(id, dist) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.distribution = dist;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadMagnitudeChanged(id, value) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.magnitude = value;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadPositionChanged(id, value) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.position = value;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadStartChanged(id, value) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.start_ft = value;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadEndChanged(id, value) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.end_ft = value;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LoadTributaryChanged(id, value) => {
                if let Some(row) = self.load_table.iter_mut().find(|r| r.id == id) {
                    row.tributary_width = value;
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::IncludeSelfWeightToggled(value) => {
                self.include_self_weight = value;
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::MaterialTypeSelected(material_type) => {
                self.selected_material_type = material_type;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::SpeciesSelected(species) => {
                self.selected_species = Some(species);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::GradeSelected(grade) => {
                self.selected_grade = Some(grade);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LumberSizeSelected(size) => {
                self.selected_lumber_size = size;
                if !size.is_custom() {
                    let (w, d) = size.actual_dimensions();
                    let total_width = w * self.selected_ply_count.count() as f64;
                    self.width_in = format!("{:.2}", total_width);
                    self.depth_in = format!("{:.2}", d);
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::PlyCountSelected(ply) => {
                self.selected_ply_count = ply;
                if !self.selected_lumber_size.is_custom() {
                    let (w, _) = self.selected_lumber_size.actual_dimensions();
                    let total_width = w * ply.count() as f64;
                    self.width_in = format!("{:.2}", total_width);
                }
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::GlulamClassSelected(class) => {
                self.selected_glulam_class = Some(class);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::GlulamLayupSelected(layup) => {
                self.selected_glulam_layup = Some(layup);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::LvlGradeSelected(grade) => {
                self.selected_lvl_grade = Some(grade);
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::PslGradeSelected(grade) => {
                self.selected_psl_grade = Some(grade);
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::LoadDurationSelected(duration) => {
                self.selected_load_duration = duration;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::WetServiceSelected(wet) => {
                self.selected_wet_service = wet;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::TemperatureSelected(temp) => {
                self.selected_temperature = temp;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::IncisingSelected(incising) => {
                self.selected_incising = incising;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::RepetitiveMemberSelected(rep) => {
                self.selected_repetitive_member = rep;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::FlatUseSelected(flat) => {
                self.selected_flat_use = flat;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::CompressionBracedToggled(braced) => {
                self.compression_edge_braced = braced;
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::NotchLocationSelected(loc) => {
                self.selected_notch_location = loc;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::NotchDepthLeftChanged(value) => {
                self.notch_depth_left = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::NotchDepthRightChanged(value) => {
                self.notch_depth_right = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::HoleDiameterChanged(value) => {
                self.hole_diameter = value;
                self.auto_save_beam();
                self.try_calculate();
            }
            Message::HoleCountChanged(value) => {
                self.hole_count = value;
                self.auto_save_beam();
                self.try_calculate();
            }

            Message::DeleteSelectedBeam => self.delete_selected_beam(),
            Message::ExportPdf => return self.export_pdf(),

            Message::ToggleSettingsMenu => {
                self.settings_menu_open = !self.settings_menu_open;
            }
            Message::ToggleDarkMode => {
                self.dark_mode = !self.dark_mode;
                self.settings_menu_open = false; // Close menu after toggling
                self.diagram_cache.clear();
            }

            // Modal interactions
            Message::ModalSave => {
                if let Some(ModalType::UnsavedChanges { action }) = self.active_modal.take() {
                    // For async save, we need to save first and handle the action after
                    // For now, use sync save on native when we have a current file
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = self.current_file.clone() {
                        self.do_save_native(path);
                        if !self.is_modified {
                            return self.perform_pending_action(action);
                        } else {
                            self.active_modal = Some(ModalType::UnsavedChanges { action });
                        }
                    } else {
                        // No current file - need async save dialog, action will be lost
                        // This is a limitation - for now just proceed without saving
                        self.status = "Please save manually before this action".to_string();
                        self.active_modal = Some(ModalType::UnsavedChanges { action });
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        // On WASM, just proceed without saving
                        self.is_modified = false;
                        return self.perform_pending_action(action);
                    }
                }
            }
            Message::ModalDontSave => {
                if let Some(ModalType::UnsavedChanges { action }) = self.active_modal.take() {
                    // Discard changes and perform the pending action
                    self.is_modified = false;
                    return self.perform_pending_action(action);
                }
            }
            Message::ModalCancel => {
                self.active_modal = None;
            }

            // Category picker
            Message::OpenCategoryPicker => {
                self.active_modal = Some(ModalType::CategoryPicker);
            }
            Message::AddCategory(category) => {
                self.enabled_categories.insert(category);
                // If category was just added, auto-expand it
                self.collapsed_sections.remove(&category.to_section());
                self.active_modal = None;
                self.status = format!("Added {} category", category.display_name());
            }
            Message::CloseCategoryPicker => {
                self.active_modal = None;
            }

            // Panel resizing
            Message::DividerDragStart(divider, _) => {
                // Mark that we're starting to drag, but wait for first mouse move
                // to capture the actual cursor position (button sends x=0)
                self.dragging_divider = Some(divider);
                self.drag_start_x = -1.0; // Sentinel value: not yet captured
                match divider {
                    DividerType::ItemsInput => {
                        self.drag_start_value = self.items_panel_width;
                    }
                    DividerType::InputResults => {
                        self.drag_start_value = self.input_panel_ratio;
                    }
                }
            }
            Message::DividerDragMove(x) => {
                if let Some(divider) = self.dragging_divider {
                    // Capture the start position on first move
                    if self.drag_start_x < 0.0 {
                        self.drag_start_x = x;
                        return Task::none();
                    }

                    let delta = x - self.drag_start_x;
                    match divider {
                        DividerType::ItemsInput => {
                            // Adjust items panel width with constraints
                            let new_width = (self.drag_start_value + delta).clamp(120.0, 400.0);
                            self.items_panel_width = new_width;
                        }
                        DividerType::InputResults => {
                            // For the second divider, we need to calculate ratio change
                            // Assume total available width for input+results is roughly 800px
                            // This is approximate - the actual width depends on window size
                            // A delta of 100px should move the ratio by about 0.1
                            let ratio_delta = delta / 800.0;
                            let new_ratio = (self.drag_start_value + ratio_delta).clamp(0.2, 0.8);
                            self.input_panel_ratio = new_ratio;
                        }
                    }
                }
            }
            Message::DividerDragEnd => {
                self.dragging_divider = None;
            }

            Message::SelectInputTab(tab) => {
                self.selected_input_tab = tab;
            }

            Message::SelectResultsTab(tab) => {
                self.selected_results_tab = tab;
            }

            // Async file operations
            Message::FileOpenComplete(result) => {
                match result {
                    Ok((file_name, bytes)) => {
                        match serde_json::from_slice::<Project>(&bytes) {
                            Ok(project) => {
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    self.file_lock = None;
                                }
                                self.project = project;

                                // Enable categories for items that exist in the loaded project
                                self.enabled_categories.clear();
                                for item in self.project.items.values() {
                                    match item {
                                        CalculationItem::Beam(_) => {
                                            self.enabled_categories.insert(ItemCategory::WoodBeams);
                                        }
                                        CalculationItem::Column(_) => {
                                            // WoodColumns category not yet implemented
                                        }
                                    }
                                }

                                // Store the file name (on both native and WASM we only get the name)
                                self.current_file = Some(PathBuf::from(&file_name));
                                self.is_modified = false;
                                self.lock_holder = None;
                                self.selection = EditorSelection::ProjectInfo;
                                self.result = None;
                                self.error_message = None;
                                self.status = format!("Opened: {}", file_name);
                            }
                            Err(e) => {
                                self.status = format!("Failed to parse project: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if e != "Open cancelled" {
                            self.status = format!("Failed to open: {}", e);
                        } else {
                            self.status = "Ready".to_string();
                        }
                    }
                }
            }

            Message::FileSaveComplete(result) => {
                match result {
                    Ok(file_name) => {
                        // Update current file name on save
                        self.current_file = Some(PathBuf::from(&file_name));
                        self.is_modified = false;
                        self.status = format!("Saved: {}", file_name);
                    }
                    Err(e) => {
                        if e != "Save cancelled" {
                            self.status = format!("Save failed: {}", e);
                        } else {
                            self.status = "Ready".to_string();
                        }
                    }
                }
            }

            Message::PdfExportComplete(result) => match result {
                Ok(file_name) => {
                    self.status = format!("Exported: {}", file_name);
                }
                Err(e) => {
                    if e != "Export cancelled" {
                        self.status = format!("Export failed: {}", e);
                    } else {
                        self.status = "Ready".to_string();
                    }
                }
            },

            // Update checking (native only)
            #[cfg(not(target_arch = "wasm32"))]
            Message::CheckForUpdates => {
                self.update_status = UpdateStatus::Checking;
                self.status = "Checking for updates...".to_string();
                return Task::perform(update::check_for_updates(), Message::UpdateCheckComplete);
            }

            #[cfg(not(target_arch = "wasm32"))]
            Message::UpdateCheckComplete(result) => match result {
                update::UpdateCheckResult::UpdateAvailable(info) => {
                    self.update_status = UpdateStatus::UpdateAvailable {
                        version: info.version.clone(),
                        download_url: info.download_url,
                        html_url: info.html_url,
                    };
                    self.status = format!("Update available: v{}", info.version);
                }
                update::UpdateCheckResult::UpToDate => {
                    self.update_status = UpdateStatus::UpToDate;
                    self.status = format!("Up to date (v{})", update::CURRENT_VERSION);
                }
                update::UpdateCheckResult::Failed(msg) => {
                    self.update_status = UpdateStatus::Failed(msg.clone());
                    self.status = format!("Update check failed: {}", msg);
                }
            },

            #[cfg(not(target_arch = "wasm32"))]
            Message::OpenUpdateUrl(url) => {
                update::open_url(&url);
            }
        }
        Task::none()
    }

    fn mark_modified(&mut self) {
        if self.can_edit() {
            self.is_modified = true;
        }
    }

    /// Check if there are unsaved changes and show modal if needed
    fn check_unsaved_changes(&mut self, action: PendingAction) -> bool {
        if self.is_modified {
            self.settings_menu_open = false; // Close settings menu if open
            self.active_modal = Some(ModalType::UnsavedChanges { action });
            true // Has unsaved changes, modal shown
        } else {
            false // No unsaved changes, can proceed
        }
    }

    /// Perform a pending action after modal confirmation
    fn perform_pending_action(&mut self, action: PendingAction) -> Task<Message> {
        match action {
            PendingAction::NewProject => {
                self.do_new_project();
                Task::none()
            }
            PendingAction::OpenProject => self.do_open_project(),
        }
    }

    fn new_project(&mut self) {
        if self.check_unsaved_changes(PendingAction::NewProject) {
            return; // Modal will handle continuation
        }
        self.do_new_project();
    }

    fn do_new_project(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.file_lock = None;
        }
        self.project = Project::new("Engineer", "25-001", "Client");
        self.current_file = None;
        self.is_modified = false;
        self.lock_holder = None;
        self.selection = EditorSelection::ProjectInfo;
        self.result = None;
        self.error_message = None;
        self.status = "New project created".to_string();
    }

    fn open_project(&mut self) -> Task<Message> {
        if self.check_unsaved_changes(PendingAction::OpenProject) {
            return Task::none(); // Modal will handle continuation
        }
        self.do_open_project()
    }

    fn do_open_project(&mut self) -> Task<Message> {
        self.status = "Opening file dialog...".to_string();
        Task::perform(
            async {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title("Open Project")
                    .add_filter("Stratify Project", &["stf"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                    .await;

                match handle {
                    Some(h) => {
                        let file_name = h.file_name();
                        let bytes = h.read().await;
                        Ok((file_name, bytes))
                    }
                    None => Err("Open cancelled".to_string()),
                }
            },
            Message::FileOpenComplete,
        )
    }

    fn save_project(&mut self) -> Task<Message> {
        if !self.can_edit() {
            self.status = "Cannot save: file is read-only".to_string();
            return Task::none();
        }

        // If we have a current file, save directly (native) or use async dialog (WASM)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref path) = self.current_file.clone() {
            self.do_save_native(path.clone());
            return Task::none();
        }

        // No current file or on WASM, need to show save dialog
        self.save_project_as()
    }

    fn save_project_as(&mut self) -> Task<Message> {
        if !self.can_edit() {
            self.status = "Cannot save: file is read-only".to_string();
            return Task::none();
        }

        let default_name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("project.stf")
            .to_string();

        // Serialize project for async save
        let project_json = match serde_json::to_string_pretty(&self.project) {
            Ok(json) => json,
            Err(e) => {
                self.status = format!("Serialization failed: {}", e);
                return Task::none();
            }
        };

        self.status = "Opening save dialog...".to_string();

        Task::perform(
            async move {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title("Save Project As")
                    .set_file_name(&default_name)
                    .add_filter("Stratify Project", &["stf"])
                    .save_file()
                    .await;

                match handle {
                    Some(h) => {
                        let file_name = h.file_name();
                        match h.write(project_json.as_bytes()).await {
                            Ok(()) => Ok(file_name),
                            Err(e) => Err(format!("Write failed: {}", e)),
                        }
                    }
                    None => Err("Save cancelled".to_string()),
                }
            },
            Message::FileSaveComplete,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn do_save_native(&mut self, path: PathBuf) {
        self.project.meta.modified = chrono::Utc::now();

        let need_new_lock = match &self.file_lock {
            Some(lock) => lock.info.user_id != whoami::username(),
            None => true,
        };

        if need_new_lock {
            let username = whoami::username();
            match FileLock::acquire(&path, &username) {
                Ok(lock) => {
                    self.file_lock = Some(lock);
                }
                Err(e) => {
                    self.status = format!("Cannot save (lock failed): {}", e);
                    return;
                }
            }
        }

        match save_project(&self.project, &path) {
            Ok(()) => {
                self.current_file = Some(path.clone());
                self.is_modified = false;
                self.status = format!("Saved: {}", path.display());
            }
            Err(e) => {
                self.status = format!("Save failed: {}", e);
            }
        }
    }

    fn select_beam(&mut self, id: Uuid) {
        if let Some(CalculationItem::Beam(beam)) = self.project.items.get(&id) {
            self.selection = EditorSelection::Beam(Some(id));
            self.beam_label = beam.label.clone();

            if let Some(first_span) = beam.spans.first() {
                self.span_ft = first_span.length_ft.to_string();
                self.width_in = first_span.width_in.to_string();
                self.depth_in = first_span.depth_in.to_string();

                let single_ply_width = first_span.width_in;
                let depth = first_span.depth_in;

                let mut found_size = LumberSize::Custom;
                let mut found_ply = PlyCount::Single;

                for ply in &PlyCount::ALL {
                    let ply_width = single_ply_width / ply.count() as f64;
                    let detected = LumberSize::from_actual_dimensions(ply_width, depth);
                    if !detected.is_custom() {
                        found_size = detected;
                        found_ply = *ply;
                        break;
                    }
                }

                self.selected_lumber_size = found_size;
                self.selected_ply_count = found_ply;

                match &first_span.material {
                    Material::SawnLumber(wood) => {
                        self.selected_material_type = MaterialType::SawnLumber;
                        self.selected_species = Some(wood.species);
                        self.selected_grade = Some(wood.grade);
                    }
                    Material::Glulam(glulam) => {
                        self.selected_material_type = MaterialType::Glulam;
                        self.selected_glulam_class = Some(glulam.stress_class);
                        self.selected_glulam_layup = Some(glulam.layup);
                    }
                    Material::Lvl(lvl) => {
                        self.selected_material_type = MaterialType::Lvl;
                        self.selected_lvl_grade = Some(lvl.grade);
                    }
                    Material::Psl(psl) => {
                        self.selected_material_type = MaterialType::Psl;
                        self.selected_psl_grade = Some(psl.grade);
                    }
                }
            }

            self.load_table = beam
                .load_case
                .loads
                .iter()
                .map(LoadTableRow::from_discrete_load)
                .collect();
            self.include_self_weight = beam.load_case.include_self_weight;

            self.selected_load_duration = beam.adjustment_factors.load_duration;
            self.selected_wet_service = beam.adjustment_factors.wet_service;
            self.selected_temperature = beam.adjustment_factors.temperature;
            self.selected_incising = beam.adjustment_factors.incising;
            self.selected_repetitive_member = beam.adjustment_factors.repetitive_member;
            self.selected_flat_use = beam.adjustment_factors.flat_use;
            self.compression_edge_braced = beam.adjustment_factors.compression_edge_braced;

            self.multi_span_mode = beam.spans.len() > 1;
            self.span_table = beam
                .spans
                .iter()
                .zip(beam.supports.iter())
                .map(|(span, support)| SpanTableRow::from_span(span, *support))
                .collect();
            self.right_end_support = beam.supports.last().copied().unwrap_or(SupportType::Roller);

            self.selected_notch_location = beam.section_deductions.notch_location;
            self.notch_depth_left = positive_or_empty(beam.section_deductions.notch_depth_left_in);
            self.notch_depth_right =
                positive_or_empty(beam.section_deductions.notch_depth_right_in);
            self.hole_diameter = positive_or_empty(beam.section_deductions.hole_diameter_in);
            self.hole_count = if beam.section_deductions.hole_count > 0 {
                beam.section_deductions.hole_count.to_string()
            } else {
                String::new()
            };

            self.error_message = None;
            self.status = format!("Selected: {}", beam.label);
            self.try_calculate();
        }
    }

    fn delete_selected_beam(&mut self) {
        if !self.can_edit() {
            self.status = "Cannot modify: file is read-only".to_string();
            return;
        }

        if let Some(id) = self.selected_beam_id() {
            if let Some(item) = self.project.items.remove(&id) {
                self.mark_modified();
                self.status = format!("Deleted: {}", item.label());
                self.selection = EditorSelection::ProjectInfo;
                self.result = None;
                self.calc_input = None;
            }
        } else {
            self.status = "No beam selected to delete".to_string();
        }
    }

    fn create_beam(&mut self) {
        if !self.can_edit() {
            self.status = "Cannot modify: file is read-only".to_string();
            return;
        }

        let beam_count = self
            .project
            .items
            .values()
            .filter(|i| matches!(i, CalculationItem::Beam(_)))
            .count();
        let new_label = format!("B-{}", beam_count + 1);

        let load_case = EnhancedLoadCase::new("Service Loads")
            .with_load(DiscreteLoad::uniform(LoadType::Dead, 15.0))
            .with_load(DiscreteLoad::uniform(LoadType::Live, 40.0));

        let beam = ContinuousBeamInput::simple_span(
            new_label.clone(),
            12.0,
            1.5,
            9.25,
            Material::SawnLumber(WoodMaterial::new(
                WoodSpecies::DouglasFirLarch,
                WoodGrade::No2,
            )),
            load_case,
        );

        let id = self.project.add_item(CalculationItem::Beam(beam));
        self.mark_modified();
        self.select_beam(id);
        self.status = format!("Created beam '{}'", new_label);
        self.try_calculate();
    }

    fn auto_save_beam(&mut self) {
        if !self.can_edit() {
            return;
        }

        let beam_id = match self.selection {
            EditorSelection::Beam(Some(id)) => id,
            _ => return,
        };

        let span_ft = match self.span_ft.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return,
        };
        let width_in = match self.width_in.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return,
        };
        let depth_in = match self.depth_in.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => return,
        };

        let material = match self.build_material() {
            Some(m) => m,
            None => return,
        };

        let total_length_ft = self.total_beam_length(span_ft);
        let mut load_case = EnhancedLoadCase::new("Service Loads");
        load_case.include_self_weight = self.include_self_weight;

        for row in &self.load_table {
            if let Some(load) = row.to_discrete_load(total_length_ft) {
                load_case = load_case.with_load(load);
            }
        }

        if load_case.loads.is_empty() && !load_case.include_self_weight {
            return;
        }

        let adjustment_factors = self.build_adjustment_factors();

        let beam = if self.multi_span_mode && self.span_table.len() > 1 {
            let mut spans = Vec::new();
            let mut supports = Vec::new();

            for (i, span_row) in self.span_table.iter().enumerate() {
                let span_len = match span_row.length_ft.parse::<f64>() {
                    Ok(v) if v > 0.0 => v,
                    _ => return,
                };
                supports.push(span_row.left_support);
                let span = SpanSegment::new(span_len, width_in, depth_in, material.clone())
                    .with_id(span_row.id);
                spans.push(span);
                if i == self.span_table.len() - 1 {
                    supports.push(self.right_end_support);
                }
            }

            let mut beam =
                ContinuousBeamInput::new(self.beam_label.clone(), spans, supports, load_case);
            beam.adjustment_factors = adjustment_factors;
            beam
        } else {
            // Single-span mode: use span_ft for length, but respect support type selections
            let left_support = self
                .span_table
                .first()
                .map(|s| s.left_support)
                .unwrap_or(SupportType::Pinned);
            let right_support = self.right_end_support;
            let span = SpanSegment::new(span_ft, width_in, depth_in, material);
            let mut beam = ContinuousBeamInput::new(
                self.beam_label.clone(),
                vec![span],
                vec![left_support, right_support],
                load_case,
            );
            beam.adjustment_factors = adjustment_factors;
            beam
        };

        let mut beam = beam;
        beam.section_deductions = self.build_section_deductions();

        self.project
            .items
            .insert(beam_id, CalculationItem::Beam(beam));
        self.mark_modified();
    }

    fn try_calculate(&mut self) {
        if !matches!(self.selection, EditorSelection::Beam(_)) {
            return;
        }

        let span_ft = match self.span_ft.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => {
                self.result = None;
                self.calc_input = None;
                return;
            }
        };
        let width_in = match self.width_in.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => {
                self.result = None;
                self.calc_input = None;
                return;
            }
        };
        let depth_in = match self.depth_in.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => {
                self.result = None;
                self.calc_input = None;
                return;
            }
        };

        let material = match self.build_material() {
            Some(m) => m,
            None => {
                self.result = None;
                self.calc_input = None;
                return;
            }
        };

        let total_length_ft = self.total_beam_length(span_ft);
        let mut load_case = EnhancedLoadCase::new("Service Loads");
        load_case.include_self_weight = self.include_self_weight;

        for row in &self.load_table {
            if let Some(load) = row.to_discrete_load(total_length_ft) {
                load_case = load_case.with_load(load);
            }
        }

        if load_case.loads.is_empty() && !load_case.include_self_weight {
            self.result = None;
            self.calc_input = None;
            return;
        }

        let adjustment_factors = self.build_adjustment_factors();

        let input = if self.multi_span_mode && self.span_table.len() > 1 {
            let mut spans = Vec::new();
            let mut supports = Vec::new();

            for (i, span_row) in self.span_table.iter().enumerate() {
                let span_len = match span_row.length_ft.parse::<f64>() {
                    Ok(v) if v > 0.0 => v,
                    _ => {
                        self.result = None;
                        self.calc_input = None;
                        return;
                    }
                };
                supports.push(span_row.left_support);
                let span = SpanSegment::new(span_len, width_in, depth_in, material.clone())
                    .with_id(span_row.id);
                spans.push(span);
                if i == self.span_table.len() - 1 {
                    supports.push(self.right_end_support);
                }
            }

            let mut input =
                ContinuousBeamInput::new(self.beam_label.clone(), spans, supports, load_case);
            input.adjustment_factors = adjustment_factors;
            input
        } else {
            let left_support = self
                .span_table
                .first()
                .map(|s| s.left_support)
                .unwrap_or(SupportType::Pinned);
            let right_support = self.right_end_support;
            let span = SpanSegment::new(span_ft, width_in, depth_in, material);
            let mut input = ContinuousBeamInput::new(
                self.beam_label.clone(),
                vec![span],
                vec![left_support, right_support],
                load_case,
            );
            input.adjustment_factors = adjustment_factors;
            input
        };

        let mut input = input;
        input.section_deductions = self.build_section_deductions();

        match calculate_continuous(&input, DesignMethod::Asd) {
            Ok(result) => {
                self.calc_input = Some(input);
                self.result = Some(result);
                self.error_message = None;
                self.diagram_cache.clear();
            }
            Err(e) => {
                self.result = None;
                self.calc_input = None;
                self.error_message = Some(format!("{}", e));
            }
        }
    }

    fn export_pdf(&mut self) -> Task<Message> {
        // Generate PDF bytes first
        let pdf_bytes = match render_project_pdf(&self.project) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.status = format!("PDF generation failed: {}", e);
                return Task::none();
            }
        };

        self.status = "Opening export dialog...".to_string();

        // Use async save dialog
        Task::perform(
            async move {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title("Export PDF")
                    .set_file_name("calculations.pdf")
                    .add_filter("PDF Document", &["pdf"])
                    .save_file()
                    .await;

                match handle {
                    Some(h) => {
                        let file_name = h.file_name();
                        match h.write(&pdf_bytes).await {
                            Ok(()) => Ok(file_name),
                            Err(e) => Err(format!("Write failed: {}", e)),
                        }
                    }
                    None => Err("Export cancelled".to_string()),
                }
            },
            Message::PdfExportComplete,
        )
    }
}

// ============================================================================
// View
// ============================================================================

impl App {
    fn view(&self) -> Element<'_, Message> {
        // Build header with stored window title
        let header = ui::toolbar::view_header_owned(self.window_title());

        // Check which divider is being dragged
        let dragging_items_input = matches!(self.dragging_divider, Some(DividerType::ItemsInput));
        let dragging_input_results =
            matches!(self.dragging_divider, Some(DividerType::InputResults));

        // Build content row with draggable dividers between panels
        // Layout: items_panel | divider | input_panel | divider | results_panel
        let content = row![
            ui::items_panel::view_items_panel(
                &self.project,
                &self.collapsed_sections,
                &self.enabled_categories,
                &self.selection,
                self.selected_beam_id(),
                self.items_panel_width,
            ),
            ui::shared::divider::view_divider(DividerType::ItemsInput, dragging_items_input),
            ui::input_panel::view_input_panel(self, self.input_panel_ratio),
            ui::shared::divider::view_divider(DividerType::InputResults, dragging_input_results),
            ui::results_panel::view_results_panel(self, self.input_panel_ratio),
        ];

        let main_content = column![
            header,
            rule::horizontal(2),
            ui::toolbar::view_toolbar(self.settings_menu_open),
            rule::horizontal(1),
            Space::new().height(10),
            content,
            Space::new().height(10),
            rule::horizontal(1),
            ui::status_bar::view_status_bar(
                &self.current_file,
                self.is_modified,
                &self.lock_holder,
                &self.status,
            ),
        ]
        .padding(15);

        let mut root_stack = stack![container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)];

        if self.settings_menu_open {
            // Transparent overlay to catch clicks outside the menu
            let backdrop = iced::widget::button(Space::new())
                .on_press(Message::ToggleSettingsMenu)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_, _| {
                    iced::widget::button::Style::default().with_background(iced::Color::TRANSPARENT)
                });

            // Position the dropdown in the top-right
            // The padding should align with the toolbar settings button
            // Toolbar is ~30px high + padding
            // We'll use a container aligned to top-right with padding
            #[cfg(not(target_arch = "wasm32"))]
            let settings_menu =
                ui::toolbar::view_settings_menu(self.dark_mode, &self.update_status);
            #[cfg(target_arch = "wasm32")]
            let settings_menu = ui::toolbar::view_settings_menu(self.dark_mode);

            let overlay = container(settings_menu)
                .padding(iced::Padding {
                    top: 50.0,
                    right: 15.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill);

            root_stack = root_stack.push(backdrop).push(overlay);
        }

        // Modal overlay (highest priority)
        if let Some(ref modal_type) = self.active_modal {
            let backdrop = ui::modal::view_backdrop();
            let modal_content = ui::modal::view_modal(modal_type);
            root_stack = root_stack.push(backdrop).push(modal_content);
        }

        root_stack.into()
    }
}

// ============================================================================
// Smoke tests
// ============================================================================
//
// These do not instantiate an iced runtime — they exercise pure App state
// and the calc_core boundary. They catch regressions in default-state setup
// and the GUI's input -> calc_core wiring without needing a GPU or window.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_app_state_has_expected_initial_values() {
        let app = App::default();

        assert_eq!(app.beam_label, "B-1");
        assert_eq!(app.span_ft, "12.0");
        assert_eq!(app.width_in, "1.5");
        assert_eq!(app.depth_in, "9.25");
        assert_eq!(app.selected_material_type, MaterialType::SawnLumber);
        assert_eq!(app.selected_species, Some(WoodSpecies::DouglasFirLarch));
        assert_eq!(app.selected_grade, Some(WoodGrade::No2));

        // Two default loads: D and L, both uniform-full.
        assert_eq!(app.load_table.len(), 2);
        assert_eq!(app.load_table[0].load_type, LoadType::Dead);
        assert_eq!(app.load_table[1].load_type, LoadType::Live);

        // No calculation has run yet.
        assert!(app.result.is_none());
        assert!(app.error_message.is_none());
    }

    #[test]
    fn try_calculate_on_default_beam_passes_with_sane_unity() {
        // The default form models a beam, but the welcome screen is the active
        // selection — switch focus to the beam editor so try_calculate runs.
        let mut app = App {
            selection: EditorSelection::Beam(None),
            ..App::default()
        };

        app.try_calculate();

        assert!(
            app.error_message.is_none(),
            "no error: {:?}",
            app.error_message
        );
        let result = app.result.as_ref().expect("result populated");

        // 2x10 DF-L No.2 at 12 ft with 15+40 plf is a textbook safe floor beam.
        let span_result = result.span_results.first().expect("span result");
        assert!(span_result.bending_unity > 0.0 && span_result.bending_unity < 1.0);
        assert!(span_result.shear_unity > 0.0 && span_result.shear_unity < 1.0);
        assert!(result.passes(), "default beam should pass all checks");
    }

    #[test]
    fn project_default_round_trips_through_json() {
        // The GUI persists Project via serde; verify the default shape survives
        // a save -> load cycle byte-for-byte. Catches a #[serde(skip)] regression
        // on a newly-added field.
        let app = App::default();
        let json = serde_json::to_string(&app.project).expect("serialize");
        let restored: Project = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.meta.engineer, app.project.meta.engineer);
        assert_eq!(restored.meta.job_id, app.project.meta.job_id);
        assert_eq!(restored.items.len(), app.project.items.len());
    }
}
