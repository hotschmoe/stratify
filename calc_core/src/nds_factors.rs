//! # NDS Adjustment Factors
//!
//! Adjustment factors for wood design per NDS 2018.
//!
//! ## Overview
//!
//! Reference design values (Fb, Fv, Fc, E, etc.) must be multiplied by
//! applicable adjustment factors to obtain adjusted design values:
//!
//! ```text
//! Fb' = Fb × C_D × C_M × C_t × C_L × C_F × C_fu × C_i × C_r
//! Fv' = Fv × C_D × C_M × C_t × C_i
//! E'  = E  × C_M × C_t × C_i
//! ```
//!
//! ## Factor Summary
//!
//! | Factor | Description              | Typical Values    |
//! |--------|--------------------------|-------------------|
//! | C_D    | Load duration            | 0.9 - 2.0         |
//! | C_M    | Wet service              | 0.85 - 1.0        |
//! | C_t    | Temperature              | 0.5 - 1.0         |
//! | C_L    | Beam stability           | Calculated        |
//! | C_F    | Size factor              | 0.9 - 1.5         |
//! | C_fu   | Flat use                 | 1.0 - 1.2         |
//! | C_i    | Incising                 | 0.80 - 1.0        |
//! | C_r    | Repetitive member        | 1.0 or 1.15       |
//!
//! ## Reference
//!
//! NDS 2018, Chapter 4: Sawn Lumber, Section 4.3

use serde::{Deserialize, Serialize};

// ============================================================================
// NDS Code Section References
// ============================================================================

/// NDS code section references for beam design checks and adjustment factors.
///
/// These constants provide traceable references to the National Design
/// Specification for Wood Construction (NDS 2018).
pub mod nds_ref {
    // Design checks
    /// Bending design value check
    pub const BENDING: &str = "NDS 3.3.1";
    /// Shear design value check
    pub const SHEAR: &str = "NDS 3.4.3";
    /// Deflection limits
    pub const DEFLECTION: &str = "NDS 3.2.2";

    // Adjustment factors
    /// Load duration factor C_D
    pub const C_D: &str = "NDS 2.3.2";
    /// Wet service factor C_M
    pub const C_M: &str = "NDS 4.3.3";
    /// Temperature factor C_t
    pub const C_T: &str = "NDS 2.3.3";
    /// Beam stability factor C_L
    pub const C_L: &str = "NDS 3.3.3";
    /// Size factor C_F
    pub const C_F: &str = "NDS Table 4A";
    /// Flat use factor C_fu
    pub const C_FU: &str = "NDS 4.3.7";
    /// Incising factor C_i
    pub const C_I: &str = "NDS 4.3.8";
    /// Repetitive member factor C_r
    pub const C_R: &str = "NDS 4.3.9";
    /// Modulus of elasticity adjustment
    pub const E_ADJUSTMENT: &str = "NDS 5.4.2";

    // Formulas
    /// Adjusted bending design value formula
    pub const FB_FORMULA: &str = "NDS 3.3.1";
    /// Adjusted shear design value formula
    pub const FV_FORMULA: &str = "NDS 3.4.3";
    /// Shear stress formula for rectangular sections
    pub const SHEAR_STRESS_FORMULA: &str = "NDS 3.4.3";
}

/// Load duration factor (C_D) per NDS Table 2.3.2
///
/// Accounts for the cumulative effect of load duration on wood strength.
/// Wood can sustain higher stresses for short durations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LoadDuration {
    /// Permanent loads (> 10 years): C_D = 0.9
    /// Examples: Dead load only
    Permanent,

    /// Normal duration (10 years): C_D = 1.0
    /// Examples: Floor live load (D + L)
    #[default]
    Normal,

    /// Snow load (2 months): C_D = 1.15
    /// Examples: Roof snow load
    Snow,

    /// Construction load (7 days): C_D = 1.25
    /// Examples: Construction activities
    Construction,

    /// Wind/Earthquake (10 minutes): C_D = 1.6
    /// Examples: Wind load, seismic load
    WindSeismic,

    /// Impact (instantaneous): C_D = 2.0
    /// Examples: Impact loads
    Impact,
}

impl LoadDuration {
    /// All load duration variants for UI selection
    pub const ALL: [LoadDuration; 6] = [
        LoadDuration::Permanent,
        LoadDuration::Normal,
        LoadDuration::Snow,
        LoadDuration::Construction,
        LoadDuration::WindSeismic,
        LoadDuration::Impact,
    ];

    /// Get the C_D factor value
    pub fn factor(&self) -> f64 {
        match self {
            LoadDuration::Permanent => 0.9,
            LoadDuration::Normal => 1.0,
            LoadDuration::Snow => 1.15,
            LoadDuration::Construction => 1.25,
            LoadDuration::WindSeismic => 1.6,
            LoadDuration::Impact => 2.0,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            LoadDuration::Permanent => "Permanent (0.90)",
            LoadDuration::Normal => "Normal (1.00)",
            LoadDuration::Snow => "Snow (1.15)",
            LoadDuration::Construction => "Construction (1.25)",
            LoadDuration::WindSeismic => "Wind/Seismic (1.60)",
            LoadDuration::Impact => "Impact (2.00)",
        }
    }
}

impl std::fmt::Display for LoadDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Wet service condition for C_M factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WetService {
    /// Dry conditions (MC ≤ 19%): C_M = 1.0
    /// Most interior applications
    #[default]
    Dry,

    /// Wet conditions (MC > 19%): C_M varies by property
    /// Exterior exposed, submerged, high humidity
    Wet,
}

impl WetService {
    /// All wet service variants for UI selection
    pub const ALL: [WetService; 2] = [WetService::Dry, WetService::Wet];

    /// Get C_M factor for bending (Fb)
    /// NDS Table 4.3.3 footnote
    pub fn factor_fb(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 0.85, // When Fb × C_F ≤ 1150 psi, use 1.0
        }
    }

    /// Get C_M factor for tension (Ft)
    pub fn factor_ft(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 1.0,
        }
    }

    /// Get C_M factor for shear (Fv)
    pub fn factor_fv(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 0.97,
        }
    }

    /// Get C_M factor for compression perpendicular (Fc_perp)
    pub fn factor_fc_perp(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 0.67,
        }
    }

    /// Get C_M factor for compression parallel (Fc)
    /// NDS Table 4.3.3 footnote
    pub fn factor_fc(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 0.8, // When Fc × C_F ≤ 750 psi, use 1.0
        }
    }

    /// Get C_M factor for modulus of elasticity (E, Emin)
    pub fn factor_e(&self) -> f64 {
        match self {
            WetService::Dry => 1.0,
            WetService::Wet => 0.9,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            WetService::Dry => "Dry (MC ≤ 19%)",
            WetService::Wet => "Wet (MC > 19%)",
        }
    }
}

impl std::fmt::Display for WetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Temperature condition for C_t factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Temperature {
    /// Normal temperature (T ≤ 100°F): C_t = 1.0
    #[default]
    Normal,

    /// Elevated temperature (100°F < T ≤ 125°F)
    Elevated,

    /// High temperature (125°F < T ≤ 150°F)
    High,
}

impl Temperature {
    /// All temperature variants for UI selection
    pub const ALL: [Temperature; 3] = [
        Temperature::Normal,
        Temperature::Elevated,
        Temperature::High,
    ];

    /// Get C_t factor for Fb, Fv, Fc, E (dry conditions)
    /// NDS Table 2.3.3
    pub fn factor_dry(&self) -> f64 {
        match self {
            Temperature::Normal => 1.0,
            Temperature::Elevated => 0.8,
            Temperature::High => 0.7,
        }
    }

    /// Get C_t factor for Fb, Fv, Fc, E (wet conditions)
    /// NDS Table 2.3.3
    pub fn factor_wet(&self) -> f64 {
        match self {
            Temperature::Normal => 1.0,
            Temperature::Elevated => 0.7,
            Temperature::High => 0.5,
        }
    }

    /// Get C_t factor based on moisture condition
    pub fn factor(&self, wet_service: WetService) -> f64 {
        match wet_service {
            WetService::Dry => self.factor_dry(),
            WetService::Wet => self.factor_wet(),
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Temperature::Normal => "Normal (≤ 100°F)",
            Temperature::Elevated => "Elevated (100-125°F)",
            Temperature::High => "High (125-150°F)",
        }
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Incising treatment condition for C_i factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Incising {
    /// Not incised: C_i = 1.0
    #[default]
    None,

    /// Incised for preservative treatment: C_i varies
    /// Per NDS Table 4.3.8
    Incised,
}

impl Incising {
    /// All incising variants for UI selection
    pub const ALL: [Incising; 2] = [Incising::None, Incising::Incised];

    /// Get C_i factor for modulus of elasticity (E)
    pub fn factor_e(&self) -> f64 {
        match self {
            Incising::None => 1.0,
            Incising::Incised => 0.95,
        }
    }

    /// Get C_i factor for bending, shear, compression (Fb, Fv, Fc)
    pub fn factor_strength(&self) -> f64 {
        match self {
            Incising::None => 1.0,
            Incising::Incised => 0.80,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Incising::None => "Not Incised",
            Incising::Incised => "Incised Treatment",
        }
    }
}

impl std::fmt::Display for Incising {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Repetitive member factor (C_r) condition
///
/// Per NDS 4.3.9: Applies when 3 or more members spaced ≤ 24" OC
/// are joined by floor, roof, or other load-distributing elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RepetitiveMember {
    /// Single member or > 24" spacing: C_r = 1.0
    #[default]
    Single,

    /// 3+ members at ≤ 24" OC with load distribution: C_r = 1.15
    Repetitive,
}

impl RepetitiveMember {
    /// All repetitive member variants for UI selection
    pub const ALL: [RepetitiveMember; 2] = [RepetitiveMember::Single, RepetitiveMember::Repetitive];

    /// Get C_r factor
    pub fn factor(&self) -> f64 {
        match self {
            RepetitiveMember::Single => 1.0,
            RepetitiveMember::Repetitive => 1.15,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            RepetitiveMember::Single => "Single Member",
            RepetitiveMember::Repetitive => "Repetitive (3+ @ ≤24\" OC)",
        }
    }
}

impl std::fmt::Display for RepetitiveMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Size factor (C_F) calculation per NDS Table 4A adjustment factors
///
/// For sawn lumber, C_F depends on member depth and width.
/// For 2"-4" thick lumber with depths:
/// - d ≤ 12": C_F from tables (typically 1.0 - 1.5)
/// - d > 12": C_F = (12/d)^(1/9)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SizeFactor {
    /// Member depth in inches
    pub depth_in: f64,
    /// Member width (thickness) in inches
    pub width_in: f64,
}

impl SizeFactor {
    /// Create new size factor calculator
    pub fn new(depth_in: f64, width_in: f64) -> Self {
        Self { depth_in, width_in }
    }

    /// Calculate C_F for bending (Fb) - sawn lumber 2"-4" thick
    ///
    /// NDS Table 4A footnote:
    /// - For d ≤ 12": Use tabulated values (we use interpolation)
    /// - For d > 12": C_F = (12/d)^(1/9)
    pub fn factor_fb(&self) -> f64 {
        if self.depth_in <= 2.5 {
            // 2x3 nominal: very small, conservative
            1.5
        } else if self.depth_in <= 3.5 {
            // 2x4 nominal
            1.5
        } else if self.depth_in <= 5.5 {
            // 2x6 nominal
            1.3
        } else if self.depth_in <= 7.25 {
            // 2x8 nominal
            1.2
        } else if self.depth_in <= 9.25 {
            // 2x10 nominal
            1.1
        } else if self.depth_in <= 11.25 {
            // 2x12 nominal
            1.0
        } else {
            // d > 12": C_F = (12/d)^(1/9)
            (12.0 / self.depth_in).powf(1.0 / 9.0)
        }
    }

    /// Calculate C_F for tension (Ft) - sawn lumber
    ///
    /// Similar to bending but with different table values
    pub fn factor_ft(&self) -> f64 {
        if self.depth_in <= 5.5 {
            1.3
        } else if self.depth_in <= 7.25 {
            1.2
        } else if self.depth_in <= 9.25 {
            1.1
        } else if self.depth_in <= 11.25 {
            1.0
        } else {
            (12.0 / self.depth_in).powf(1.0 / 9.0)
        }
    }

    /// Calculate C_F for compression parallel (Fc) - sawn lumber
    ///
    /// NDS Table 4A footnote
    pub fn factor_fc(&self) -> f64 {
        if self.depth_in <= 5.5 {
            1.15
        } else if self.depth_in <= 7.25 {
            1.1
        } else if self.depth_in <= 9.25 {
            1.05
        } else if self.depth_in <= 11.25 {
            1.0
        } else {
            (12.0 / self.depth_in).powf(1.0 / 9.0)
        }
    }
}

/// Flat use factor (C_fu) per NDS Table 4.3.7
///
/// Applies when lumber is loaded on wide face (bending about weak axis).
/// Only applies to bending design value Fb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FlatUse {
    /// Normal use (loaded on narrow face): C_fu = 1.0
    #[default]
    Normal,

    /// Flat use (loaded on wide face): C_fu from table
    /// Applied to Fb only
    Flat,
}

impl FlatUse {
    /// All flat use variants for UI selection
    pub const ALL: [FlatUse; 2] = [FlatUse::Normal, FlatUse::Flat];

    /// Get C_fu factor for bending (Fb)
    ///
    /// Values from NDS Table 4.3.7 for 2"-4" thick lumber
    pub fn factor(&self, width_in: f64) -> f64 {
        match self {
            FlatUse::Normal => 1.0,
            FlatUse::Flat => {
                // C_fu varies by width (depth when flat)
                if width_in <= 1.5 {
                    // 2x nominal
                    1.0
                } else if width_in <= 2.5 {
                    // 3x nominal
                    1.04
                } else if width_in <= 3.5 {
                    // 4x nominal
                    1.10
                } else {
                    // > 4x
                    1.15
                }
            }
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            FlatUse::Normal => "Normal (edge bending)",
            FlatUse::Flat => "Flat (wide face bending)",
        }
    }
}

impl std::fmt::Display for FlatUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Beam stability factor (C_L) per NDS 3.3.3
///
/// Accounts for lateral-torsional buckling in beams.
/// C_L = 1.0 when compression edge is continuously braced,
/// otherwise calculated based on slenderness ratio.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeamStability {
    /// Effective unbraced length (le) in inches
    pub unbraced_length_in: f64,
    /// Beam width (b) in inches
    pub width_in: f64,
    /// Beam depth (d) in inches
    pub depth_in: f64,
}

impl BeamStability {
    /// Create stability factor calculator
    pub fn new(unbraced_length_in: f64, width_in: f64, depth_in: f64) -> Self {
        Self {
            unbraced_length_in,
            width_in,
            depth_in,
        }
    }

    /// Calculate slenderness ratio R_B per NDS 3.3.3.5
    ///
    /// R_B = sqrt(le × d / b²)
    /// where le = effective span length
    pub fn slenderness_ratio(&self) -> f64 {
        let le = self.unbraced_length_in;
        let d = self.depth_in;
        let b = self.width_in;

        (le * d / (b * b)).sqrt()
    }

    /// Calculate beam stability factor C_L per NDS 3.3.3.8
    ///
    /// # Arguments
    /// * `fb_star` - Fb* = Fb × all factors except C_L and C_V (psi)
    /// * `e_min_prime` - E'min = Emin × C_M × C_t × C_i (psi)
    ///
    /// # Returns
    /// C_L factor (0.0 - 1.0)
    pub fn factor(&self, fb_star: f64, e_min_prime: f64) -> f64 {
        let rb = self.slenderness_ratio();

        // Check slenderness limit (NDS 3.3.3.3)
        // R_B shall not exceed 50
        if rb > 50.0 {
            return 0.0; // Beam is too slender
        }

        // F_bE = critical buckling design value (NDS Eq. 3.3-6)
        // F_bE = 1.20 × E'min / R_B²
        let f_be = 1.20 * e_min_prime / (rb * rb);

        // C_L per NDS Eq. 3.3-6
        // C_L = (1 + F_bE/Fb*) / 1.9 - sqrt[((1 + F_bE/Fb*)/ 1.9)² - (F_bE/Fb*) / 0.95]
        let ratio = f_be / fb_star;
        let term1 = (1.0 + ratio) / 1.9;
        let term2 = (term1 * term1 - ratio / 0.95).sqrt();

        let c_l = term1 - term2;

        // C_L cannot exceed 1.0 (`fb_star` ensures `ratio` and `term*` are finite,
        // so neither bound is NaN — clamp is safe here).
        c_l.clamp(0.0, 1.0)
    }

    /// Check if compression edge is fully braced
    ///
    /// When compression edge is continuously supported, C_L = 1.0
    /// NDS 3.3.3.1
    pub fn is_fully_braced(&self) -> bool {
        // Simplified check: if unbraced length is very short relative to depth
        // Consider it fully braced. Full bracing means le/d approaches 0.
        // In practice, set a threshold like le ≤ 2d
        self.unbraced_length_in <= 2.0 * self.depth_in
    }
}

/// Collection of all adjustment factors for a beam design
///
/// This struct collects all the factors that affect Fb', Fv', E', etc.
/// and provides methods to calculate adjusted design values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentFactors {
    /// Load duration factor selection
    pub load_duration: LoadDuration,

    /// Wet service condition
    pub wet_service: WetService,

    /// Temperature condition
    pub temperature: Temperature,

    /// Incising treatment
    pub incising: Incising,

    /// Repetitive member condition (for bending only)
    pub repetitive_member: RepetitiveMember,

    /// Flat use condition (for bending only)
    pub flat_use: FlatUse,

    /// Whether compression edge is continuously braced (C_L = 1.0)
    /// If false, C_L must be calculated from beam geometry
    pub compression_edge_braced: bool,

    /// Unbraced length for beam stability (le) in inches
    /// Only used if compression_edge_braced is false
    pub unbraced_length_in: Option<f64>,
}

impl Default for AdjustmentFactors {
    fn default() -> Self {
        Self {
            load_duration: LoadDuration::default(),
            wet_service: WetService::default(),
            temperature: Temperature::default(),
            incising: Incising::default(),
            repetitive_member: RepetitiveMember::default(),
            flat_use: FlatUse::default(),
            compression_edge_braced: true, // Conservative default: assume braced
            unbraced_length_in: None,
        }
    }
}

impl AdjustmentFactors {
    /// Create default factors for typical interior floor beam
    pub fn new() -> Self {
        Self::default()
    }

    /// Set load duration
    pub fn with_load_duration(mut self, duration: LoadDuration) -> Self {
        self.load_duration = duration;
        self
    }

    /// Set wet service condition
    pub fn with_wet_service(mut self, wet: WetService) -> Self {
        self.wet_service = wet;
        self
    }

    /// Set temperature condition
    pub fn with_temperature(mut self, temp: Temperature) -> Self {
        self.temperature = temp;
        self
    }

    /// Set incising treatment
    pub fn with_incising(mut self, incising: Incising) -> Self {
        self.incising = incising;
        self
    }

    /// Set repetitive member factor
    pub fn with_repetitive(mut self, repetitive: RepetitiveMember) -> Self {
        self.repetitive_member = repetitive;
        self
    }

    /// Set flat use factor
    pub fn with_flat_use(mut self, flat_use: FlatUse) -> Self {
        self.flat_use = flat_use;
        self
    }

    /// Set compression edge bracing
    pub fn with_bracing(mut self, braced: bool, unbraced_length_in: Option<f64>) -> Self {
        self.compression_edge_braced = braced;
        self.unbraced_length_in = unbraced_length_in;
        self
    }

    /// Get C_D factor
    pub fn c_d(&self) -> f64 {
        self.load_duration.factor()
    }

    /// Get C_M factor for bending
    pub fn c_m_fb(&self) -> f64 {
        self.wet_service.factor_fb()
    }

    /// Get C_M factor for shear
    pub fn c_m_fv(&self) -> f64 {
        self.wet_service.factor_fv()
    }

    /// Get C_M factor for modulus of elasticity
    pub fn c_m_e(&self) -> f64 {
        self.wet_service.factor_e()
    }

    /// Get C_t factor
    pub fn c_t(&self) -> f64 {
        self.temperature.factor(self.wet_service)
    }

    /// Get C_i factor for strength
    pub fn c_i_strength(&self) -> f64 {
        self.incising.factor_strength()
    }

    /// Get C_i factor for E
    pub fn c_i_e(&self) -> f64 {
        self.incising.factor_e()
    }

    /// Get C_r factor
    pub fn c_r(&self) -> f64 {
        self.repetitive_member.factor()
    }

    /// Get C_fu factor
    pub fn c_fu(&self, width_in: f64) -> f64 {
        self.flat_use.factor(width_in)
    }

    /// Calculate adjusted bending stress Fb'
    ///
    /// Fb' = Fb × C_D × C_M × C_t × C_L × C_F × C_fu × C_i × C_r
    ///
    /// Note: C_L requires E'min and Fb* to calculate. If compression edge
    /// is braced, C_L = 1.0. Otherwise, pass calculated C_L value.
    pub fn adjusted_fb(&self, fb_reference: f64, c_f: f64, c_l: f64, width_in: f64) -> f64 {
        fb_reference
            * self.c_d()
            * self.c_m_fb()
            * self.c_t()
            * c_l
            * c_f
            * self.c_fu(width_in)
            * self.c_i_strength()
            * self.c_r()
    }

    /// Calculate adjusted shear stress Fv'
    ///
    /// Fv' = Fv × C_D × C_M × C_t × C_i
    pub fn adjusted_fv(&self, fv_reference: f64) -> f64 {
        fv_reference * self.c_d() * self.c_m_fv() * self.c_t() * self.c_i_strength()
    }

    /// Calculate adjusted modulus of elasticity E'
    ///
    /// E' = E × C_M × C_t × C_i
    pub fn adjusted_e(&self, e_reference: f64) -> f64 {
        e_reference * self.c_m_e() * self.c_t() * self.c_i_e()
    }

    /// Calculate adjusted minimum E (E'min) for stability calculations
    ///
    /// E'min = Emin × C_M × C_t × C_i
    pub fn adjusted_e_min(&self, e_min_reference: f64) -> f64 {
        e_min_reference * self.c_m_e() * self.c_t() * self.c_i_e()
    }

    /// Get a summary of all applied factors for reporting
    pub fn summary(&self, width_in: f64, depth_in: f64, c_f: f64, c_l: f64) -> AdjustmentSummary {
        AdjustmentSummary {
            c_d: self.c_d(),
            c_m_fb: self.c_m_fb(),
            c_m_fv: self.c_m_fv(),
            c_m_e: self.c_m_e(),
            c_t: self.c_t(),
            c_l,
            c_f,
            c_fu: self.c_fu(width_in),
            c_i_strength: self.c_i_strength(),
            c_i_e: self.c_i_e(),
            c_r: self.c_r(),
            net_fb_factor: self.c_d()
                * self.c_m_fb()
                * self.c_t()
                * c_l
                * c_f
                * self.c_fu(width_in)
                * self.c_i_strength()
                * self.c_r(),
            net_fv_factor: self.c_d() * self.c_m_fv() * self.c_t() * self.c_i_strength(),
            net_e_factor: self.c_m_e() * self.c_t() * self.c_i_e(),
            width_in,
            depth_in,
        }
    }
}

/// Summary of adjustment factors for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentSummary {
    /// Load duration factor
    pub c_d: f64,
    /// Wet service factor for Fb
    pub c_m_fb: f64,
    /// Wet service factor for Fv
    pub c_m_fv: f64,
    /// Wet service factor for E
    pub c_m_e: f64,
    /// Temperature factor
    pub c_t: f64,
    /// Beam stability factor
    pub c_l: f64,
    /// Size factor
    pub c_f: f64,
    /// Flat use factor
    pub c_fu: f64,
    /// Incising factor for strength
    pub c_i_strength: f64,
    /// Incising factor for E
    pub c_i_e: f64,
    /// Repetitive member factor
    pub c_r: f64,
    /// Net factor for Fb' = Fb × (this)
    pub net_fb_factor: f64,
    /// Net factor for Fv' = Fv × (this)
    pub net_fv_factor: f64,
    /// Net factor for E' = E × (this)
    pub net_e_factor: f64,
    /// Section width for reference
    pub width_in: f64,
    /// Section depth for reference
    pub depth_in: f64,
}

impl AdjustmentSummary {
    /// Format as a multi-line string for reports
    pub fn format_report(&self) -> String {
        format!(
            "NDS Adjustment Factors ({}\" x {}\")\n\
             ================================================\n\
             C_D  (Load Duration)    = {:.2}    {}\n\
             C_M  (Wet Service - Fb) = {:.2}    {}\n\
             C_M  (Wet Service - Fv) = {:.2}    {}\n\
             C_M  (Wet Service - E)  = {:.2}    {}\n\
             C_t  (Temperature)      = {:.2}    {}\n\
             C_L  (Beam Stability)   = {:.3}   {}\n\
             C_F  (Size)             = {:.2}    {}\n\
             C_fu (Flat Use)         = {:.2}    {}\n\
             C_i  (Incising)         = {:.2}    {}\n\
             C_r  (Repetitive)       = {:.2}    {}\n\
             ------------------------------------------------\n\
             Net Fb factor           = {:.3}   {}\n\
             Net Fv factor           = {:.3}   {}\n\
             Net E factor            = {:.3}   {}",
            self.width_in,
            self.depth_in,
            self.c_d,
            nds_ref::C_D,
            self.c_m_fb,
            nds_ref::C_M,
            self.c_m_fv,
            nds_ref::C_M,
            self.c_m_e,
            nds_ref::C_M,
            self.c_t,
            nds_ref::C_T,
            self.c_l,
            nds_ref::C_L,
            self.c_f,
            nds_ref::C_F,
            self.c_fu,
            nds_ref::C_FU,
            self.c_i_strength,
            nds_ref::C_I,
            self.c_r,
            nds_ref::C_R,
            self.net_fb_factor,
            nds_ref::FB_FORMULA,
            self.net_fv_factor,
            nds_ref::FV_FORMULA,
            self.net_e_factor,
            nds_ref::E_ADJUSTMENT,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_duration_factors() {
        assert_eq!(LoadDuration::Permanent.factor(), 0.9);
        assert_eq!(LoadDuration::Normal.factor(), 1.0);
        assert_eq!(LoadDuration::Snow.factor(), 1.15);
        assert_eq!(LoadDuration::WindSeismic.factor(), 1.6);
    }

    #[test]
    fn test_wet_service_factors() {
        assert_eq!(WetService::Dry.factor_fb(), 1.0);
        assert_eq!(WetService::Wet.factor_fb(), 0.85);
        assert_eq!(WetService::Wet.factor_e(), 0.9);
    }

    #[test]
    fn test_temperature_factors() {
        assert_eq!(Temperature::Normal.factor(WetService::Dry), 1.0);
        assert_eq!(Temperature::Elevated.factor(WetService::Dry), 0.8);
        assert_eq!(Temperature::Elevated.factor(WetService::Wet), 0.7);
    }

    #[test]
    fn test_size_factor_fb() {
        let sf = SizeFactor::new(9.25, 1.5);
        assert!((sf.factor_fb() - 1.1).abs() < 0.01);

        let sf_large = SizeFactor::new(15.0, 1.5);
        let expected = (12.0_f64 / 15.0).powf(1.0 / 9.0);
        assert!((sf_large.factor_fb() - expected).abs() < 0.001);
    }

    #[test]
    fn test_beam_stability_slenderness() {
        // 12' span, 1.5" x 9.25" beam
        let stability = BeamStability::new(144.0, 1.5, 9.25);
        let rb = stability.slenderness_ratio();
        // R_B = sqrt(144 × 9.25 / 1.5²) = sqrt(592) = 24.3
        assert!((rb - 24.3).abs() < 0.5);
    }

    #[test]
    fn test_beam_stability_factor() {
        let stability = BeamStability::new(144.0, 1.5, 9.25);
        // Typical values for DF-L No.2
        let fb_star = 900.0 * 1.0 * 1.1; // Fb × C_D × C_F
        let e_min_prime = 580_000.0; // E'min

        let c_l = stability.factor(fb_star, e_min_prime);
        // Should be < 1.0 for an unbraced beam
        assert!(c_l > 0.0 && c_l < 1.0);
    }

    #[test]
    fn test_default_adjustment_factors() {
        let factors = AdjustmentFactors::default();
        assert_eq!(factors.c_d(), 1.0);
        assert_eq!(factors.c_m_fb(), 1.0);
        assert_eq!(factors.c_t(), 1.0);
        assert_eq!(factors.c_r(), 1.0);
    }

    #[test]
    fn test_adjusted_fb_calculation() {
        let factors = AdjustmentFactors::new()
            .with_load_duration(LoadDuration::Snow)
            .with_repetitive(RepetitiveMember::Repetitive);

        // Fb = 900 psi, C_F = 1.1, C_L = 1.0, width = 1.5"
        let fb_adj = factors.adjusted_fb(900.0, 1.1, 1.0, 1.5);
        // Expected: 900 × 1.15 × 1.0 × 1.0 × 1.0 × 1.1 × 1.0 × 1.0 × 1.15 = 1309.5
        assert!((fb_adj - 1309.5).abs() < 1.0);
    }

    #[test]
    fn test_adjusted_fv_calculation() {
        let factors = AdjustmentFactors::new().with_load_duration(LoadDuration::Snow);

        let fv_adj = factors.adjusted_fv(180.0);
        // Expected: 180 × 1.15 = 207
        assert!((fv_adj - 207.0).abs() < 0.1);
    }

    #[test]
    fn test_adjustment_summary() {
        let factors = AdjustmentFactors::new();
        let summary = factors.summary(1.5, 9.25, 1.1, 1.0);

        assert_eq!(summary.c_d, 1.0);
        assert_eq!(summary.c_f, 1.1);
        assert!(summary.net_fb_factor > 1.0); // Should include C_F
    }

    #[test]
    fn test_serialization() {
        let factors = AdjustmentFactors::new()
            .with_load_duration(LoadDuration::Snow)
            .with_wet_service(WetService::Wet);

        let json = serde_json::to_string(&factors).unwrap();
        assert!(json.contains("Snow"));
        assert!(json.contains("Wet"));

        let parsed: AdjustmentFactors = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.load_duration, LoadDuration::Snow);
        assert_eq!(parsed.wet_service, WetService::Wet);
    }
}
