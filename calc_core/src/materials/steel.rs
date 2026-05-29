//! Steel Shapes Database (AISC)
//!
//! Section properties for structural steel shapes per the AISC Steel Construction Manual.
//! This module provides data structures and lookup functions for standard steel sections.
//!
//! ## Data Source
//!
//! Shape properties come from the AISC Shapes Database v16.0 (August 2023).
//! See `assets/steel/OBTAINING_AISC_DATA.md` for instructions on obtaining
//! the official database from AISC.
//!
//! ## Supported Shape Types
//!
//! - **W-shapes**: Wide flange beams (most common for beams and columns)
//! - **HSS**: Hollow Structural Sections (rectangular, square, round)
//! - **C-shapes**: American Standard channels
//! - **MC-shapes**: Miscellaneous channels
//! - **L-shapes**: Single angles
//! - **WT, MT, ST**: Structural tees (cut from W, M, S shapes)
//! - **Pipe**: Standard, Extra Strong, Double Extra Strong
//!
//! ## Example
//!
//! ```rust,ignore
//! use calc_core::materials::steel::{SteelShapeDb, ShapeType};
//!
//! let db = SteelShapeDb::load_from_csv("assets/steel/aisc-shapes-v16.csv")?;
//! let w14x90 = db.lookup("W14X90")?;
//!
//! println!("Area = {} in²", w14x90.area_in2);
//! println!("Ix = {} in⁴", w14x90.ix_in4);
//! println!("Zx = {} in³", w14x90.zx_in3);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use crate::errors::{CalcError, CalcResult};

/// Steel shape type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShapeType {
    /// Wide flange beam (W-shape)
    W,
    /// Miscellaneous shape (M-shape)
    M,
    /// American Standard beam (S-shape)
    S,
    /// H-pile (HP-shape)
    HP,
    /// American Standard channel (C-shape)
    C,
    /// Miscellaneous channel (MC-shape)
    MC,
    /// Single angle (L-shape)
    L,
    /// Double angle (2L-shape)
    TwoL,
    /// Structural tee cut from W-shape
    WT,
    /// Structural tee cut from M-shape
    MT,
    /// Structural tee cut from S-shape
    ST,
    /// Rectangular/square hollow structural section
    HssRect,
    /// Round hollow structural section
    HssRound,
    /// Pipe (standard, extra strong, double extra strong)
    Pipe,
}

impl ShapeType {
    /// All shape types for iteration
    pub const ALL: [ShapeType; 14] = [
        ShapeType::W,
        ShapeType::M,
        ShapeType::S,
        ShapeType::HP,
        ShapeType::C,
        ShapeType::MC,
        ShapeType::L,
        ShapeType::TwoL,
        ShapeType::WT,
        ShapeType::MT,
        ShapeType::ST,
        ShapeType::HssRect,
        ShapeType::HssRound,
        ShapeType::Pipe,
    ];

    /// Parse from AISC type code
    pub fn from_aisc_code(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "W" => Some(ShapeType::W),
            "M" => Some(ShapeType::M),
            "S" => Some(ShapeType::S),
            "HP" => Some(ShapeType::HP),
            "C" => Some(ShapeType::C),
            "MC" => Some(ShapeType::MC),
            "L" => Some(ShapeType::L),
            "2L" => Some(ShapeType::TwoL),
            "WT" => Some(ShapeType::WT),
            "MT" => Some(ShapeType::MT),
            "ST" => Some(ShapeType::ST),
            "HSS" => Some(ShapeType::HssRect), // Will be refined based on shape
            "PIPE" => Some(ShapeType::Pipe),
            _ => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ShapeType::W => "Wide Flange (W)",
            ShapeType::M => "Miscellaneous (M)",
            ShapeType::S => "American Standard (S)",
            ShapeType::HP => "H-Pile (HP)",
            ShapeType::C => "Channel (C)",
            ShapeType::MC => "Miscellaneous Channel (MC)",
            ShapeType::L => "Angle (L)",
            ShapeType::TwoL => "Double Angle (2L)",
            ShapeType::WT => "Tee (WT)",
            ShapeType::MT => "Tee (MT)",
            ShapeType::ST => "Tee (ST)",
            ShapeType::HssRect => "HSS Rectangular/Square",
            ShapeType::HssRound => "HSS Round",
            ShapeType::Pipe => "Pipe",
        }
    }

    /// Check if shape type has flanges (bf, tf properties)
    pub fn has_flanges(&self) -> bool {
        matches!(
            self,
            ShapeType::W
                | ShapeType::M
                | ShapeType::S
                | ShapeType::HP
                | ShapeType::C
                | ShapeType::MC
                | ShapeType::WT
                | ShapeType::MT
                | ShapeType::ST
        )
    }

    /// Check if shape type is hollow (HSS or Pipe)
    pub fn is_hollow(&self) -> bool {
        matches!(
            self,
            ShapeType::HssRect | ShapeType::HssRound | ShapeType::Pipe
        )
    }
}

impl std::fmt::Display for ShapeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Structural steel shape with all section properties
///
/// Properties follow the AISC Shapes Database naming conventions.
/// All dimensional values are in US customary units (inches, in², in³, in⁴, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteelShape {
    /// Shape type (W, HSS, C, L, etc.)
    pub shape_type: ShapeType,

    /// AISC Manual label (e.g., "W14X90", "HSS8X8X1/2")
    pub label: String,

    /// EDI standard nomenclature for electronic interchange
    pub edi_name: Option<String>,

    // === Dimensional Properties ===
    /// Nominal weight per linear foot (lb/ft)
    pub weight_plf: f64,

    /// Cross-sectional area (in²)
    pub area_in2: f64,

    /// Overall depth (in) - for W, S, C, etc.
    pub depth_in: Option<f64>,

    /// Overall height for HSS (in)
    pub height_in: Option<f64>,

    /// Overall width for HSS (in)
    pub width_in: Option<f64>,

    /// Outside diameter for round HSS/pipe (in)
    pub od_in: Option<f64>,

    /// Inside diameter for round HSS/pipe (in)
    pub id_in: Option<f64>,

    /// Flange width (in) - for shapes with flanges
    pub bf_in: Option<f64>,

    /// Flange thickness (in)
    pub tf_in: Option<f64>,

    /// Web thickness (in)
    pub tw_in: Option<f64>,

    /// Wall thickness for HSS/pipe (in)
    pub wall_thickness_in: Option<f64>,

    /// Distance from outer flange face to web toe of fillet (in)
    pub k_des_in: Option<f64>,

    /// Detailing dimension for k (in)
    pub k_det_in: Option<f64>,

    /// Distance from web centerline to flange toe of fillet (in)
    pub k1_in: Option<f64>,

    // === Section Properties - Strong Axis (X-X) ===
    /// Moment of inertia about X-axis (in⁴)
    pub ix_in4: f64,

    /// Elastic section modulus about X-axis (in³)
    pub sx_in3: f64,

    /// Radius of gyration about X-axis (in)
    pub rx_in: f64,

    /// Plastic section modulus about X-axis (in³)
    pub zx_in3: f64,

    // === Section Properties - Weak Axis (Y-Y) ===
    /// Moment of inertia about Y-axis (in⁴)
    pub iy_in4: f64,

    /// Elastic section modulus about Y-axis (in³)
    pub sy_in3: f64,

    /// Radius of gyration about Y-axis (in)
    pub ry_in: f64,

    /// Plastic section modulus about Y-axis (in³)
    pub zy_in3: f64,

    // === Torsional Properties ===
    /// Torsional constant (in⁴)
    pub j_in4: f64,

    /// Warping constant (in⁶)
    pub cw_in6: Option<f64>,

    // === Flexural-Torsional Properties ===
    /// Effective radius of gyration for LTB (in)
    pub rts_in: Option<f64>,

    /// Distance between flange centroids (in)
    pub ho_in: Option<f64>,

    // === Slenderness Ratios ===
    /// Flange slenderness ratio: bf / (2*tf)
    pub bf_2tf: Option<f64>,

    /// Web slenderness ratio: h / tw
    pub h_tw: Option<f64>,

    /// Diameter-to-thickness ratio for round sections: D/t
    pub d_t: Option<f64>,
}

impl SteelShape {
    /// Get the shape's display name (same as label)
    pub fn display_name(&self) -> &str {
        &self.label
    }

    /// Check if this shape has valid bending properties
    pub fn can_bend(&self) -> bool {
        self.ix_in4 > 0.0 && self.sx_in3 > 0.0
    }

    /// Get the governing radius of gyration (minimum of rx, ry)
    pub fn r_min(&self) -> f64 {
        self.rx_in.min(self.ry_in)
    }

    /// Get the governing slenderness ratio for a given unbraced length
    pub fn slenderness(&self, unbraced_length_in: f64) -> f64 {
        unbraced_length_in / self.r_min()
    }

    /// Get depth (works for any shape type)
    pub fn depth(&self) -> f64 {
        self.depth_in
            .or(self.height_in)
            .or(self.od_in)
            .unwrap_or(0.0)
    }

    /// Get width (works for any shape type)
    pub fn width(&self) -> f64 {
        self.bf_in.or(self.width_in).or(self.od_in).unwrap_or(0.0)
    }
}

impl std::fmt::Display for SteelShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (A={:.2} in², Ix={:.1} in⁴, Sx={:.1} in³)",
            self.label, self.area_in2, self.ix_in4, self.sx_in3
        )
    }
}

/// Steel shapes database loaded from AISC CSV
///
/// This holds all steel shapes in memory for fast lookup.
/// Shapes are indexed by their AISC label (e.g., "W14X90").
#[derive(Debug, Clone, Default)]
pub struct SteelShapeDb {
    /// Shapes indexed by uppercase label
    shapes: HashMap<String, SteelShape>,

    /// Shapes grouped by type for filtering
    by_type: HashMap<ShapeType, Vec<String>>,

    /// Database version (e.g., "16.0")
    pub version: Option<String>,
}

impl SteelShapeDb {
    /// Create an empty database
    pub fn new() -> Self {
        Self::default()
    }

    /// Load shapes from a CSV file
    ///
    /// The CSV should be exported from the AISC Shapes Database Excel file.
    /// See `assets/steel/OBTAINING_AISC_DATA.md` for format details.
    pub fn load_from_csv(path: &str) -> CalcResult<Self> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(path)
            .map_err(|e| CalcError::file_error("open", path, format!("Failed to open CSV: {e}")))?;

        let reader = BufReader::new(file);
        let lines = reader.lines();

        // Read header line
        let mut lines = lines;
        let header_line = lines
            .next()
            .ok_or_else(|| CalcError::file_error("read", path, "CSV file is empty"))?
            .map_err(|e| {
                CalcError::file_error("read", path, format!("Failed to read header: {e}"))
            })?;

        let headers: Vec<&str> = header_line.split(',').collect();
        let col_index = |name: &str| -> Option<usize> {
            headers.iter().position(|h| h.eq_ignore_ascii_case(name))
        };

        // Find required column indices
        let type_idx = col_index("Type")
            .ok_or_else(|| CalcError::file_error("parse", path, "Missing 'Type' column"))?;
        let label_idx = col_index("AISC_Manual_Label").ok_or_else(|| {
            CalcError::file_error("parse", path, "Missing 'AISC_Manual_Label' column")
        })?;

        // Find optional column indices
        let edi_idx = col_index("EDI_Std_Nomenclature");
        let w_idx = col_index("W");
        let a_idx = col_index("A");
        let d_idx = col_index("d");
        let ht_idx = col_index("Ht");
        let b_idx = col_index("B");
        let od_idx = col_index("OD");
        let id_idx = col_index("ID");
        let bf_idx = col_index("bf");
        let tf_idx = col_index("tf");
        let tw_idx = col_index("tw");
        let t_idx = col_index("tdes").or_else(|| col_index("t"));
        let kdes_idx = col_index("kdes");
        let kdet_idx = col_index("kdet");
        let k1_idx = col_index("k1");
        let ix_idx = col_index("Ix");
        let sx_idx = col_index("Sx");
        let rx_idx = col_index("rx");
        let zx_idx = col_index("Zx");
        let iy_idx = col_index("Iy");
        let sy_idx = col_index("Sy");
        let ry_idx = col_index("ry");
        let zy_idx = col_index("Zy");
        let j_idx = col_index("J");
        let cw_idx = col_index("Cw");
        let rts_idx = col_index("rts");
        let ho_idx = col_index("ho");
        let bf2tf_idx = col_index("bf/2tf");
        let htw_idx = col_index("h/tw");
        let dt_idx = col_index("D/t");

        let mut db = SteelShapeDb::new();

        for (line_num, line_result) in (2..).zip(lines) {
            let line = line_result.map_err(|e| {
                CalcError::file_error("read", path, format!("Failed to read line {line_num}: {e}"))
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split(',').collect();

            // Parse required fields
            let type_str = fields.get(type_idx).copied().unwrap_or("");
            let label = fields.get(label_idx).copied().unwrap_or("").to_string();

            if label.is_empty() {
                continue; // Skip rows without a label
            }

            let shape_type = match ShapeType::from_aisc_code(type_str) {
                Some(t) => {
                    // Refine HSS type based on presence of OD
                    if t == ShapeType::HssRect {
                        let has_od = od_idx
                            .and_then(|i| fields.get(i))
                            .map(|v| parse_optional_f64(v).is_some())
                            .unwrap_or(false);
                        if has_od {
                            ShapeType::HssRound
                        } else {
                            ShapeType::HssRect
                        }
                    } else {
                        t
                    }
                }
                None => continue, // Skip unknown shape types
            };

            let get_f64 = |idx: Option<usize>| -> f64 {
                idx.and_then(|i| fields.get(i))
                    .and_then(|v| parse_optional_f64(v))
                    .unwrap_or(0.0)
            };

            let get_opt_f64 = |idx: Option<usize>| -> Option<f64> {
                idx.and_then(|i| fields.get(i))
                    .and_then(|v| parse_optional_f64(v))
            };

            let shape = SteelShape {
                shape_type,
                label: label.clone(),
                edi_name: edi_idx
                    .and_then(|i| fields.get(i))
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                weight_plf: get_f64(w_idx),
                area_in2: get_f64(a_idx),
                depth_in: get_opt_f64(d_idx),
                height_in: get_opt_f64(ht_idx),
                width_in: get_opt_f64(b_idx),
                od_in: get_opt_f64(od_idx),
                id_in: get_opt_f64(id_idx),
                bf_in: get_opt_f64(bf_idx),
                tf_in: get_opt_f64(tf_idx),
                tw_in: get_opt_f64(tw_idx),
                wall_thickness_in: get_opt_f64(t_idx),
                k_des_in: get_opt_f64(kdes_idx),
                k_det_in: get_opt_f64(kdet_idx),
                k1_in: get_opt_f64(k1_idx),
                ix_in4: get_f64(ix_idx),
                sx_in3: get_f64(sx_idx),
                rx_in: get_f64(rx_idx),
                zx_in3: get_f64(zx_idx),
                iy_in4: get_f64(iy_idx),
                sy_in3: get_f64(sy_idx),
                ry_in: get_f64(ry_idx),
                zy_in3: get_f64(zy_idx),
                j_in4: get_f64(j_idx),
                cw_in6: get_opt_f64(cw_idx),
                rts_in: get_opt_f64(rts_idx),
                ho_in: get_opt_f64(ho_idx),
                bf_2tf: get_opt_f64(bf2tf_idx),
                h_tw: get_opt_f64(htw_idx),
                d_t: get_opt_f64(dt_idx),
            };

            db.insert(shape);
        }

        Ok(db)
    }

    /// Insert a shape into the database
    pub fn insert(&mut self, shape: SteelShape) {
        let key = shape.label.to_uppercase();
        let shape_type = shape.shape_type;

        self.shapes.insert(key.clone(), shape);

        self.by_type.entry(shape_type).or_default().push(key);
    }

    /// Look up a shape by its AISC label
    ///
    /// Label matching is case-insensitive.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let shape = db.lookup("W14X90")?;
    /// let shape = db.lookup("w14x90")?; // Also works
    /// ```
    pub fn lookup(&self, label: &str) -> CalcResult<&SteelShape> {
        let key = label.to_uppercase();
        self.shapes.get(&key).ok_or_else(|| {
            CalcError::material_not_found(format!("Steel shape '{label}' not found in database"))
        })
    }

    /// Get all shapes of a specific type
    pub fn shapes_of_type(&self, shape_type: ShapeType) -> Vec<&SteelShape> {
        self.by_type
            .get(&shape_type)
            .map(|labels| labels.iter().filter_map(|l| self.shapes.get(l)).collect())
            .unwrap_or_default()
    }

    /// Get all W-shapes (wide flange beams)
    pub fn w_shapes(&self) -> Vec<&SteelShape> {
        self.shapes_of_type(ShapeType::W)
    }

    /// Get all HSS shapes (rectangular and round)
    pub fn hss_shapes(&self) -> Vec<&SteelShape> {
        let mut shapes = self.shapes_of_type(ShapeType::HssRect);
        shapes.extend(self.shapes_of_type(ShapeType::HssRound));
        shapes
    }

    /// Get all channel shapes (C and MC)
    pub fn channel_shapes(&self) -> Vec<&SteelShape> {
        let mut shapes = self.shapes_of_type(ShapeType::C);
        shapes.extend(self.shapes_of_type(ShapeType::MC));
        shapes
    }

    /// Get all angle shapes (L and 2L)
    pub fn angle_shapes(&self) -> Vec<&SteelShape> {
        let mut shapes = self.shapes_of_type(ShapeType::L);
        shapes.extend(self.shapes_of_type(ShapeType::TwoL));
        shapes
    }

    /// Get all shape labels in the database
    pub fn all_labels(&self) -> Vec<&str> {
        self.shapes.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of shapes in the database
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Check if the database is empty
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Search for shapes matching a pattern
    ///
    /// Supports simple prefix matching (e.g., "W14" matches all W14 shapes).
    pub fn search(&self, pattern: &str) -> Vec<&SteelShape> {
        let pattern_upper = pattern.to_uppercase();
        self.shapes
            .iter()
            .filter(|(k, _)| k.starts_with(&pattern_upper))
            .map(|(_, v)| v)
            .collect()
    }
}

/// Parse an optional f64 from a CSV field
///
/// Returns None for empty strings, dashes, or invalid numbers.
fn parse_optional_f64(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed == "—" {
        return None;
    }
    f64::from_str(trimmed).ok()
}

// ============================================================================
// Built-in Common Shapes (for use without CSV file)
// ============================================================================

/// Get a database with common W-shapes pre-loaded
///
/// This provides a minimal set of shapes for testing and demos without
/// requiring the full AISC CSV file.
pub fn builtin_common_shapes() -> SteelShapeDb {
    let mut db = SteelShapeDb::new();

    // Common W-shapes (selected from AISC Manual 16th Ed).
    // `allow(approx_constant)`: AISC section properties (notably W14X132's rT=6.28) coincidentally
    // approximate TAU; they are tabulated values, not the math constant.
    #[allow(clippy::approx_constant)]
    let common_w_shapes = [
        // W14 series (common columns)
        (
            "W14X22", 22.0, 6.49, 13.7, 7.0, 0.335, 0.22, 199.0, 29.0, 5.54, 33.2, 7.0, 2.8, 1.04,
            4.39, 0.208,
        ),
        (
            "W14X30", 30.0, 8.85, 13.8, 6.73, 0.385, 0.27, 291.0, 42.0, 5.73, 47.3, 19.6, 5.82,
            1.49, 8.99, 0.38,
        ),
        (
            "W14X48", 48.0, 14.1, 13.8, 8.03, 0.595, 0.34, 485.0, 70.2, 5.85, 78.4, 51.4, 12.8,
            1.91, 26.7, 1.45,
        ),
        (
            "W14X90", 90.0, 26.5, 14.0, 14.5, 0.71, 0.44, 999.0, 143.0, 6.14, 157.0, 362.0, 49.9,
            3.7, 362.0, 4.06,
        ),
        (
            "W14X132", 132.0, 38.8, 14.7, 14.7, 1.03, 0.645, 1530.0, 209.0, 6.28, 234.0, 548.0,
            74.5, 3.76, 548.0, 12.3,
        ),
        // W12 series (common beams/columns)
        (
            "W12X19", 19.0, 5.57, 12.2, 4.01, 0.35, 0.235, 130.0, 21.3, 4.82, 24.7, 3.76, 1.88,
            0.822, 2.98, 0.18,
        ),
        (
            "W12X26", 26.0, 7.65, 12.2, 6.49, 0.38, 0.23, 204.0, 33.4, 5.17, 37.2, 17.3, 5.34,
            1.51, 17.3, 0.3,
        ),
        (
            "W12X40", 40.0, 11.7, 11.9, 8.01, 0.515, 0.295, 307.0, 51.5, 5.13, 57.0, 44.1, 11.0,
            1.94, 44.1, 0.86,
        ),
        (
            "W12X58", 58.0, 17.0, 12.2, 10.0, 0.64, 0.36, 475.0, 78.0, 5.28, 86.4, 107.0, 21.4,
            2.51, 107.0, 2.1,
        ),
        (
            "W12X96", 96.0, 28.2, 12.7, 12.2, 0.9, 0.55, 833.0, 131.0, 5.44, 147.0, 270.0, 44.4,
            3.09, 270.0, 6.85,
        ),
        // W10 series
        (
            "W10X22", 22.0, 6.49, 10.2, 5.75, 0.36, 0.24, 118.0, 23.2, 4.27, 26.0, 11.4, 3.97,
            1.33, 11.4, 0.239,
        ),
        (
            "W10X33", 33.0, 9.71, 9.73, 7.96, 0.435, 0.29, 170.0, 35.0, 4.19, 38.8, 36.6, 9.2,
            1.94, 36.6, 0.583,
        ),
        (
            "W10X49", 49.0, 14.4, 10.0, 10.0, 0.56, 0.34, 272.0, 54.6, 4.35, 60.4, 93.4, 18.7,
            2.54, 93.4, 1.39,
        ),
        // W8 series
        (
            "W8X18", 18.0, 5.26, 8.14, 5.25, 0.33, 0.23, 61.9, 15.2, 3.43, 17.0, 7.97, 3.04, 1.23,
            7.97, 0.172,
        ),
        (
            "W8X24", 24.0, 7.08, 7.93, 6.5, 0.4, 0.245, 82.7, 20.9, 3.42, 23.1, 18.3, 5.63, 1.61,
            18.3, 0.346,
        ),
        (
            "W8X31", 31.0, 9.12, 8.0, 8.0, 0.435, 0.285, 110.0, 27.5, 3.47, 30.4, 37.1, 9.27, 2.02,
            37.1, 0.536,
        ),
        // W6 series
        (
            "W6X9", 9.0, 2.68, 5.9, 3.94, 0.215, 0.17, 16.4, 5.56, 2.47, 6.23, 2.2, 1.11, 0.905,
            2.2, 0.0398,
        ),
        (
            "W6X15", 15.0, 4.43, 5.99, 5.99, 0.26, 0.23, 29.1, 9.72, 2.56, 10.8, 9.32, 3.11, 1.45,
            9.32, 0.103,
        ),
        // W16 series (common beams)
        (
            "W16X26", 26.0, 7.68, 15.7, 5.5, 0.345, 0.25, 301.0, 38.4, 6.26, 44.2, 9.59, 3.49,
            1.12, 5.48, 0.262,
        ),
        (
            "W16X36", 36.0, 10.6, 15.9, 6.99, 0.43, 0.295, 448.0, 56.5, 6.51, 64.0, 24.5, 7.0,
            1.52, 24.5, 0.545,
        ),
        (
            "W16X50", 50.0, 14.7, 16.3, 7.07, 0.63, 0.38, 659.0, 81.0, 6.68, 92.0, 37.2, 10.5,
            1.59, 37.2, 1.52,
        ),
        // W18 series (common beams)
        (
            "W18X35", 35.0, 10.3, 17.7, 6.0, 0.425, 0.3, 510.0, 57.6, 7.04, 66.5, 15.3, 5.12, 1.22,
            8.06, 0.506,
        ),
        (
            "W18X50", 50.0, 14.7, 18.0, 7.5, 0.57, 0.355, 800.0, 88.9, 7.38, 101.0, 40.1, 10.7,
            1.65, 40.1, 1.24,
        ),
        (
            "W18X71", 71.0, 20.8, 18.5, 7.64, 0.81, 0.495, 1170.0, 127.0, 7.5, 146.0, 60.3, 15.8,
            1.7, 60.3, 3.49,
        ),
        // W21 series
        (
            "W21X44", 44.0, 13.0, 20.7, 6.5, 0.45, 0.35, 843.0, 81.6, 8.06, 95.4, 20.7, 6.36, 1.26,
            9.77, 0.77,
        ),
        (
            "W21X62", 62.0, 18.3, 21.0, 8.24, 0.615, 0.4, 1330.0, 127.0, 8.54, 144.0, 57.5, 13.9,
            1.77, 57.5, 1.83,
        ),
        // W24 series (common beams)
        (
            "W24X55", 55.0, 16.2, 23.6, 7.01, 0.505, 0.395, 1350.0, 114.0, 9.11, 134.0, 29.1, 8.3,
            1.34, 13.4, 1.18,
        ),
        (
            "W24X76", 76.0, 22.4, 23.9, 8.99, 0.68, 0.44, 2100.0, 176.0, 9.69, 200.0, 82.5, 18.4,
            1.92, 82.5, 2.68,
        ),
        (
            "W24X94", 94.0, 27.7, 24.3, 9.07, 0.875, 0.515, 2700.0, 222.0, 9.87, 254.0, 109.0,
            24.0, 1.98, 109.0, 5.26,
        ),
    ];

    for (label, w, a, d, bf, tf, tw, ix, sx, rx, zx, iy, sy, ry, zy, j) in common_w_shapes {
        db.insert(SteelShape {
            shape_type: ShapeType::W,
            label: label.to_string(),
            edi_name: None,
            weight_plf: w,
            area_in2: a,
            depth_in: Some(d),
            height_in: None,
            width_in: None,
            od_in: None,
            id_in: None,
            bf_in: Some(bf),
            tf_in: Some(tf),
            tw_in: Some(tw),
            wall_thickness_in: None,
            k_des_in: None,
            k_det_in: None,
            k1_in: None,
            ix_in4: ix,
            sx_in3: sx,
            rx_in: rx,
            zx_in3: zx,
            iy_in4: iy,
            sy_in3: sy,
            ry_in: ry,
            zy_in3: zy,
            j_in4: j,
            cw_in6: None,
            rts_in: None,
            ho_in: None,
            bf_2tf: Some(bf / (2.0 * tf)),
            h_tw: None,
            d_t: None,
        });
    }

    db.version = Some("builtin-common".to_string());
    db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_type_parsing() {
        assert_eq!(ShapeType::from_aisc_code("W"), Some(ShapeType::W));
        assert_eq!(ShapeType::from_aisc_code("HSS"), Some(ShapeType::HssRect));
        assert_eq!(ShapeType::from_aisc_code("PIPE"), Some(ShapeType::Pipe));
        assert_eq!(ShapeType::from_aisc_code("2L"), Some(ShapeType::TwoL));
        assert_eq!(ShapeType::from_aisc_code("UNKNOWN"), None);
    }

    #[test]
    fn test_builtin_shapes() {
        let db = builtin_common_shapes();
        // Exact count: 5 W14 + 5 W12 + 3 W10 + 3 W8 + 2 W6 + 3 W16 + 3 W18 + 2 W21 + 3 W24 = 29
        assert_eq!(db.len(), 29);

        // Test lookup with AISC property verification
        let w14x90 = db.lookup("W14X90").unwrap();
        assert_eq!(w14x90.weight_plf, 90.0);
        assert_eq!(w14x90.area_in2, 26.5);
        assert!((w14x90.ix_in4 - 999.0).abs() < 1.0);
        assert!((w14x90.sx_in3 - 143.0).abs() < 1.0);

        // Test case-insensitive lookup
        let w14x90_lower = db.lookup("w14x90").unwrap();
        assert_eq!(w14x90.label, w14x90_lower.label);
    }

    #[test]
    fn test_shape_filtering() {
        let db = builtin_common_shapes();

        let w_shapes = db.w_shapes();
        assert!(!w_shapes.is_empty());
        assert!(w_shapes.iter().all(|s| s.shape_type == ShapeType::W));
    }

    #[test]
    fn test_shape_search() {
        let db = builtin_common_shapes();

        let w14_shapes = db.search("W14");
        assert!(!w14_shapes.is_empty());
        assert!(w14_shapes.iter().all(|s| s.label.starts_with("W14")));
    }

    #[test]
    fn test_shape_properties() {
        let db = builtin_common_shapes();
        let shape = db.lookup("W12X26").unwrap();

        assert!(shape.can_bend());
        assert!(shape.depth() > 0.0);
        assert!(shape.width() > 0.0);
        assert!(shape.r_min() > 0.0);

        // Test slenderness calculation
        let slenderness = shape.slenderness(120.0); // 10 ft unbraced
        assert!(slenderness > 0.0);
    }

    #[test]
    fn test_parse_optional_f64() {
        assert_eq!(parse_optional_f64("123.45"), Some(123.45));
        assert_eq!(parse_optional_f64("  456  "), Some(456.0));
        assert_eq!(parse_optional_f64(""), None);
        assert_eq!(parse_optional_f64("-"), None);
        assert_eq!(parse_optional_f64("—"), None);
        assert_eq!(parse_optional_f64("not a number"), None);
    }

    #[test]
    fn test_shape_not_found() {
        let db = builtin_common_shapes();
        let result = db.lookup("NONEXISTENT");
        assert!(result.is_err());
    }
}
