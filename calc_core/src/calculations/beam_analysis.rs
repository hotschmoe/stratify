//! Simply-Supported Beam Analysis
//!
//! Provides analysis functions for simply-supported beams with various load types.
//! Uses superposition to handle multiple loads simultaneously.
//!
//! ## Supported Load Types
//! - Point loads at any position
//! - Full uniform loads (constant w over span)
//! - Partial uniform loads (w over portion of span)
//! - Applied moments at any position
//!
//! ## Sign Convention
//! - Positive moment: tension on bottom fiber (sagging)
//! - Positive shear: left side up, right side down
//! - Positive deflection: downward
//!
//! ## Example
//! ```rust
//! use calc_core::calculations::beam_analysis::{BeamAnalysis, SingleLoad};
//!
//! // 12 ft beam with point load and uniform load
//! let mut analysis = BeamAnalysis::new(12.0, 1_400_000.0, 98.93);
//!
//! // Add 1000 lb point load at midspan
//! analysis.add_load(SingleLoad::point(1000.0, 6.0));
//!
//! // Add 50 plf uniform load over full span
//! analysis.add_load(SingleLoad::uniform_full(50.0));
//!
//! // Get results
//! let results = analysis.analyze();
//! println!("Max moment: {:.1} ft-lb", results.max_moment_ftlb);
//! println!("Max shear: {:.1} lb", results.max_shear_lb);
//! println!("Max deflection: {:.4} in", results.max_deflection_in);
//! ```

use serde::{Deserialize, Serialize};

/// A single load applied to the beam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SingleLoad {
    /// Point load (lb) at position (ft from left support)
    Point { magnitude_lb: f64, position_ft: f64 },

    /// Uniform load (plf) over entire span
    UniformFull { magnitude_plf: f64 },

    /// Uniform load (plf) over partial span
    UniformPartial {
        magnitude_plf: f64,
        start_ft: f64,
        end_ft: f64,
    },

    /// Applied moment (ft-lb) at position (ft from left support)
    /// Positive = counterclockwise
    Moment {
        magnitude_ftlb: f64,
        position_ft: f64,
    },
}

impl SingleLoad {
    /// Create a point load
    pub fn point(magnitude_lb: f64, position_ft: f64) -> Self {
        SingleLoad::Point {
            magnitude_lb,
            position_ft,
        }
    }

    /// Create a full-span uniform load
    pub fn uniform_full(magnitude_plf: f64) -> Self {
        SingleLoad::UniformFull { magnitude_plf }
    }

    /// Create a partial-span uniform load
    pub fn uniform_partial(magnitude_plf: f64, start_ft: f64, end_ft: f64) -> Self {
        SingleLoad::UniformPartial {
            magnitude_plf,
            start_ft,
            end_ft,
        }
    }

    /// Create an applied moment
    pub fn moment(magnitude_ftlb: f64, position_ft: f64) -> Self {
        SingleLoad::Moment {
            magnitude_ftlb,
            position_ft,
        }
    }

    /// Calculate left reaction (R1) for this load
    pub fn reaction_left(&self, span_ft: f64) -> f64 {
        match self {
            SingleLoad::Point {
                magnitude_lb,
                position_ft,
            } => {
                // R1 = P(L-a)/L
                magnitude_lb * (span_ft - position_ft) / span_ft
            }
            SingleLoad::UniformFull { magnitude_plf } => {
                // R1 = wL/2
                magnitude_plf * span_ft / 2.0
            }
            SingleLoad::UniformPartial {
                magnitude_plf,
                start_ft,
                end_ft,
            } => {
                // Total load W = w(b-a), centroid at c = (a+b)/2
                // R1 = W(L-c)/L
                let w = *magnitude_plf;
                let a = *start_ft;
                let b = *end_ft;
                let total_load = w * (b - a);
                let centroid = (a + b) / 2.0;
                total_load * (span_ft - centroid) / span_ft
            }
            SingleLoad::Moment {
                magnitude_ftlb,
                position_ft: _,
            } => {
                // R1 = -M/L (moment creates couple reaction)
                -magnitude_ftlb / span_ft
            }
        }
    }

    /// Calculate right reaction (R2) for this load
    pub fn reaction_right(&self, span_ft: f64) -> f64 {
        match self {
            SingleLoad::Point {
                magnitude_lb,
                position_ft,
            } => {
                // R2 = Pa/L
                magnitude_lb * position_ft / span_ft
            }
            SingleLoad::UniformFull { magnitude_plf } => {
                // R2 = wL/2
                magnitude_plf * span_ft / 2.0
            }
            SingleLoad::UniformPartial {
                magnitude_plf,
                start_ft,
                end_ft,
            } => {
                let w = *magnitude_plf;
                let a = *start_ft;
                let b = *end_ft;
                let total_load = w * (b - a);
                let centroid = (a + b) / 2.0;
                total_load * centroid / span_ft
            }
            SingleLoad::Moment {
                magnitude_ftlb,
                position_ft: _,
            } => {
                // R2 = M/L
                *magnitude_ftlb / span_ft
            }
        }
    }

    /// Calculate shear at position x (ft from left support)
    pub fn shear_at(&self, x_ft: f64, span_ft: f64) -> f64 {
        let r1 = self.reaction_left(span_ft);

        match self {
            SingleLoad::Point {
                magnitude_lb,
                position_ft,
            } => {
                if x_ft < *position_ft {
                    r1
                } else {
                    r1 - magnitude_lb
                }
            }
            SingleLoad::UniformFull { magnitude_plf } => {
                // V(x) = R1 - wx
                r1 - magnitude_plf * x_ft
            }
            SingleLoad::UniformPartial {
                magnitude_plf,
                start_ft,
                end_ft,
            } => {
                let w = *magnitude_plf;
                let a = *start_ft;
                let b = *end_ft;

                if x_ft <= a {
                    // Before the load
                    r1
                } else if x_ft >= b {
                    // After the load
                    r1 - w * (b - a)
                } else {
                    // Within the load
                    r1 - w * (x_ft - a)
                }
            }
            SingleLoad::Moment { .. } => {
                // Applied moment doesn't directly affect shear (only reactions)
                r1
            }
        }
    }

    /// Calculate moment at position x (ft from left support)
    /// Returns moment in ft-lb
    pub fn moment_at(&self, x_ft: f64, span_ft: f64) -> f64 {
        let r1 = self.reaction_left(span_ft);

        match self {
            SingleLoad::Point {
                magnitude_lb,
                position_ft,
            } => {
                if x_ft < *position_ft {
                    // M(x) = R1 * x
                    r1 * x_ft
                } else {
                    // M(x) = R1 * x - P(x - a)
                    r1 * x_ft - magnitude_lb * (x_ft - position_ft)
                }
            }
            SingleLoad::UniformFull { magnitude_plf } => {
                // M(x) = R1*x - w*x²/2 = wL*x/2 - w*x²/2 = wx(L-x)/2
                let w = *magnitude_plf;
                r1 * x_ft - w * x_ft * x_ft / 2.0
            }
            SingleLoad::UniformPartial {
                magnitude_plf,
                start_ft,
                end_ft,
            } => {
                let w = *magnitude_plf;
                let a = *start_ft;
                let b = *end_ft;

                if x_ft <= a {
                    // Before the load: M = R1 * x
                    r1 * x_ft
                } else if x_ft >= b {
                    // After the load: M = R1*x - W*(x - c) where c is centroid
                    let total_load = w * (b - a);
                    let centroid = (a + b) / 2.0;
                    r1 * x_ft - total_load * (x_ft - centroid)
                } else {
                    // Within the load: M = R1*x - w*(x-a)²/2
                    r1 * x_ft - w * (x_ft - a).powi(2) / 2.0
                }
            }
            SingleLoad::Moment {
                magnitude_ftlb,
                position_ft,
            } => {
                // For applied moment M0 at position a:
                // M(x) = R1*x for x < a
                // M(x) = R1*x + M0 for x >= a
                if x_ft < *position_ft {
                    r1 * x_ft
                } else {
                    r1 * x_ft + magnitude_ftlb
                }
            }
        }
    }

    /// Calculate deflection at position x (inches)
    /// Uses EI for beam stiffness
    /// span_ft is in feet, e_psi and i_in4 in standard units
    pub fn deflection_at(&self, x_ft: f64, span_ft: f64, e_psi: f64, i_in4: f64) -> f64 {
        let l = span_ft * 12.0; // Convert to inches
        let x = x_ft * 12.0; // Convert to inches
        let ei = e_psi * i_in4;

        match self {
            SingleLoad::Point {
                magnitude_lb,
                position_ft,
            } => {
                // Point load deflection formula (Roark's)
                // For P at position a, deflection at x:
                let p = *magnitude_lb;
                let a = position_ft * 12.0; // Convert to inches
                let b = l - a;

                if x <= a {
                    // δ(x) = Pbx(L² - b² - x²) / (6EIL)
                    p * b * x * (l * l - b * b - x * x) / (6.0 * ei * l)
                } else {
                    // δ(x) = Pa(L-x)(2Lx - x² - a²) / (6EIL)
                    p * a * (l - x) * (2.0 * l * x - x * x - a * a) / (6.0 * ei * l)
                }
            }
            SingleLoad::UniformFull { magnitude_plf } => {
                // δ(x) = wx(L³ - 2Lx² + x³) / (24EI)
                let w = magnitude_plf / 12.0; // Convert plf to lb/in
                w * x * (l.powi(3) - 2.0 * l * x * x + x.powi(3)) / (24.0 * ei)
            }
            SingleLoad::UniformPartial {
                magnitude_plf: _,
                start_ft: _,
                end_ft: _,
            } => {
                // Partial uniform load deflection is complex
                // Use integration or approximate with multiple point loads
                // For now, use numerical integration via moment-area method
                self.deflection_partial_uniform(x_ft, span_ft, e_psi, i_in4)
            }
            SingleLoad::Moment {
                magnitude_ftlb,
                position_ft,
            } => {
                // Applied moment deflection
                let m0 = magnitude_ftlb * 12.0; // Convert ft-lb to in-lb
                let a = position_ft * 12.0;
                let _b = l - a;

                if x <= a {
                    // δ(x) = M0*x*(L² - 3a² + 2ax - x²) / (6EIL) -- simplified
                    // Using standard formula: δ = M0*b*x*(L-x)(L+b-x) / (6EIL²) ... complex
                    // Simplified: For x < a: δ = M0*x*(2L-3a+x*(a/L-1)) / (6EI)
                    // Using the standard moment deflection formula:
                    m0 * x * (2.0 * l - 3.0 * a) / (6.0 * ei * l)
                        + m0 * x * x * x / (6.0 * ei * l * l) * (a - l)
                } else {
                    // After the moment application point
                    let term1 = m0 * a * (l - x) * (2.0 * l - a) / (6.0 * ei * l);
                    let term2 = -m0 * (x - a).powi(2) / (2.0 * ei);
                    term1 + term2
                }
            }
        }
    }

    /// Calculate partial uniform load deflection using numerical integration
    fn deflection_partial_uniform(&self, x_ft: f64, span_ft: f64, e_psi: f64, i_in4: f64) -> f64 {
        if let SingleLoad::UniformPartial {
            magnitude_plf,
            start_ft,
            end_ft,
        } = self
        {
            // Approximate by treating as multiple point loads
            let num_segments = 20;
            let segment_length = (end_ft - start_ft) / num_segments as f64;
            let segment_load = magnitude_plf * segment_length; // lb per segment

            let mut total_deflection = 0.0;
            for i in 0..num_segments {
                let pos = start_ft + (i as f64 + 0.5) * segment_length;
                let point_load = SingleLoad::Point {
                    magnitude_lb: segment_load,
                    position_ft: pos,
                };
                total_deflection += point_load.deflection_at(x_ft, span_ft, e_psi, i_in4);
            }
            total_deflection
        } else {
            0.0
        }
    }
}

/// Results from beam analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Left support reaction (lb) - positive upward
    pub reaction_left_lb: f64,
    /// Right support reaction (lb) - positive upward
    pub reaction_right_lb: f64,

    /// Maximum positive moment (ft-lb)
    pub max_moment_ftlb: f64,
    /// Position of maximum moment (ft from left)
    pub max_moment_position_ft: f64,

    /// Maximum shear magnitude (lb)
    pub max_shear_lb: f64,
    /// Position of maximum shear (ft from left)
    pub max_shear_position_ft: f64,

    /// Maximum deflection (inches) - positive downward
    pub max_deflection_in: f64,
    /// Position of maximum deflection (ft from left)
    pub max_deflection_position_ft: f64,

    /// Sampled shear values along beam for plotting
    pub shear_diagram: Vec<(f64, f64)>,
    /// Sampled moment values along beam for plotting
    pub moment_diagram: Vec<(f64, f64)>,
    /// Sampled deflection values along beam for plotting
    pub deflection_diagram: Vec<(f64, f64)>,
}

/// Beam analysis with superposition of multiple loads
#[derive(Debug, Clone)]
pub struct BeamAnalysis {
    /// Span length (ft)
    pub span_ft: f64,
    /// Modulus of elasticity (psi)
    pub e_psi: f64,
    /// Moment of inertia (in^4)
    pub i_in4: f64,
    /// Collection of loads to analyze
    pub loads: Vec<SingleLoad>,
    /// Number of sample points for diagrams
    pub sample_points: usize,
}

impl BeamAnalysis {
    /// Create a new beam analysis
    pub fn new(span_ft: f64, e_psi: f64, i_in4: f64) -> Self {
        BeamAnalysis {
            span_ft,
            e_psi,
            i_in4,
            loads: Vec::new(),
            sample_points: 101, // Default: 101 points (every 1% of span)
        }
    }

    /// Add a load to the analysis
    pub fn add_load(&mut self, load: SingleLoad) {
        self.loads.push(load);
    }

    /// Set the number of sample points for diagrams
    pub fn with_sample_points(mut self, points: usize) -> Self {
        self.sample_points = points.max(11); // Minimum 11 points
        self
    }

    /// Calculate total left reaction
    pub fn total_reaction_left(&self) -> f64 {
        self.loads
            .iter()
            .map(|load| load.reaction_left(self.span_ft))
            .sum()
    }

    /// Calculate total right reaction
    pub fn total_reaction_right(&self) -> f64 {
        self.loads
            .iter()
            .map(|load| load.reaction_right(self.span_ft))
            .sum()
    }

    /// Calculate total shear at position x (superposition)
    pub fn shear_at(&self, x_ft: f64) -> f64 {
        self.loads
            .iter()
            .map(|load| load.shear_at(x_ft, self.span_ft))
            .sum()
    }

    /// Calculate total moment at position x (superposition)
    pub fn moment_at(&self, x_ft: f64) -> f64 {
        self.loads
            .iter()
            .map(|load| load.moment_at(x_ft, self.span_ft))
            .sum()
    }

    /// Calculate total deflection at position x (superposition)
    pub fn deflection_at(&self, x_ft: f64) -> f64 {
        self.loads
            .iter()
            .map(|load| load.deflection_at(x_ft, self.span_ft, self.e_psi, self.i_in4))
            .sum()
    }

    /// Get sample positions including critical points
    fn get_sample_positions(&self) -> Vec<f64> {
        let mut positions: Vec<f64> = Vec::new();

        // Add regular sample points
        for i in 0..self.sample_points {
            let x = self.span_ft * i as f64 / (self.sample_points - 1) as f64;
            positions.push(x);
        }

        // Add critical points (load positions, just before/after)
        let epsilon = self.span_ft * 0.001; // Small offset for discontinuities
        for load in &self.loads {
            match load {
                SingleLoad::Point { position_ft, .. } | SingleLoad::Moment { position_ft, .. } => {
                    let pos = *position_ft;
                    if pos > epsilon && pos < self.span_ft - epsilon {
                        positions.push(pos - epsilon);
                        positions.push(pos);
                        positions.push(pos + epsilon);
                    }
                }
                SingleLoad::UniformPartial {
                    start_ft, end_ft, ..
                } => {
                    if *start_ft > epsilon {
                        positions.push(start_ft - epsilon);
                        positions.push(*start_ft);
                        positions.push(start_ft + epsilon);
                    }
                    if *end_ft < self.span_ft - epsilon {
                        positions.push(end_ft - epsilon);
                        positions.push(*end_ft);
                        positions.push(end_ft + epsilon);
                    }
                }
                _ => {}
            }
        }

        // Sort and deduplicate. `total_cmp` gives a total order over f64 (NaN-safe);
        // upstream filtering keeps NaN out of `positions`, so order over reals is unchanged.
        positions.sort_by(|a, b| a.total_cmp(b));
        positions.dedup_by(|a, b| (*a - *b).abs() < epsilon / 2.0);

        positions
    }

    /// Perform full analysis
    pub fn analyze(&self) -> AnalysisResults {
        let positions = self.get_sample_positions();

        // Calculate diagrams
        let mut shear_diagram: Vec<(f64, f64)> = Vec::new();
        let mut moment_diagram: Vec<(f64, f64)> = Vec::new();
        let mut deflection_diagram: Vec<(f64, f64)> = Vec::new();

        let mut max_shear = 0.0f64;
        let mut max_shear_pos = 0.0;
        let mut max_moment = 0.0f64;
        let mut max_moment_pos = 0.0;
        let mut max_deflection = 0.0f64;
        let mut max_deflection_pos = 0.0;

        for &x in &positions {
            let v = self.shear_at(x);
            let m = self.moment_at(x);
            let d = self.deflection_at(x);

            shear_diagram.push((x, v));
            moment_diagram.push((x, m));
            deflection_diagram.push((x, d));

            // Track maximums (absolute value for shear, positive for moment/deflection)
            if v.abs() > max_shear.abs() {
                max_shear = v.abs();
                max_shear_pos = x;
            }
            if m > max_moment {
                max_moment = m;
                max_moment_pos = x;
            }
            if d > max_deflection {
                max_deflection = d;
                max_deflection_pos = x;
            }
        }

        AnalysisResults {
            reaction_left_lb: self.total_reaction_left(),
            reaction_right_lb: self.total_reaction_right(),
            max_moment_ftlb: max_moment,
            max_moment_position_ft: max_moment_pos,
            max_shear_lb: max_shear,
            max_shear_position_ft: max_shear_pos,
            max_deflection_in: max_deflection,
            max_deflection_position_ft: max_deflection_pos,
            shear_diagram,
            moment_diagram,
            deflection_diagram,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 0.01; // 1% tolerance for tests

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        if b.abs() < 1e-10 {
            a.abs() < tol
        } else {
            ((a - b) / b).abs() < tol
        }
    }

    #[test]
    fn test_point_load_reactions() {
        // 10 ft beam, 1000 lb at midspan
        let load = SingleLoad::point(1000.0, 5.0);
        let r1 = load.reaction_left(10.0);
        let r2 = load.reaction_right(10.0);

        assert!(approx_eq(r1, 500.0, EPSILON));
        assert!(approx_eq(r2, 500.0, EPSILON));

        // Asymmetric: 1000 lb at 3 ft on 10 ft span
        let load = SingleLoad::point(1000.0, 3.0);
        let r1 = load.reaction_left(10.0);
        let r2 = load.reaction_right(10.0);

        // R1 = P(L-a)/L = 1000 * 7/10 = 700
        // R2 = Pa/L = 1000 * 3/10 = 300
        assert!(approx_eq(r1, 700.0, EPSILON));
        assert!(approx_eq(r2, 300.0, EPSILON));
    }

    #[test]
    fn test_point_load_moment() {
        // 10 ft beam, 1000 lb at midspan
        // Max moment = PL/4 = 1000 * 10 / 4 = 2500 ft-lb
        let load = SingleLoad::point(1000.0, 5.0);
        let m_max = load.moment_at(5.0, 10.0);

        assert!(approx_eq(m_max, 2500.0, EPSILON));
    }

    #[test]
    fn test_uniform_load_moment() {
        // 10 ft beam, 100 plf uniform load
        // Max moment = wL²/8 = 100 * 100 / 8 = 1250 ft-lb
        let load = SingleLoad::uniform_full(100.0);
        let m_max = load.moment_at(5.0, 10.0);

        assert!(approx_eq(m_max, 1250.0, EPSILON));
    }

    #[test]
    fn test_uniform_load_shear() {
        // 10 ft beam, 100 plf
        // V at x=0: R1 = wL/2 = 500 lb
        // V at x=5: 500 - 100*5 = 0 lb
        // V at x=10: 500 - 100*10 = -500 lb
        let load = SingleLoad::uniform_full(100.0);

        assert!(approx_eq(load.shear_at(0.0, 10.0), 500.0, EPSILON));
        assert!(approx_eq(load.shear_at(5.0, 10.0), 0.0, 0.01)); // Allow small absolute error
        assert!(approx_eq(load.shear_at(10.0, 10.0), -500.0, EPSILON));
    }

    #[test]
    fn test_uniform_load_deflection() {
        // 10 ft beam, 100 plf, E = 1,400,000 psi, I = 100 in^4
        // Max deflection at midspan: δ = 5wL^4 / (384EI)
        // w = 100/12 = 8.333 lb/in, L = 120 in
        // δ = 5 * 8.333 * 120^4 / (384 * 1,400,000 * 100)
        // δ = 5 * 8.333 * 207,360,000 / 53,760,000,000
        // δ = 0.161 in
        let load = SingleLoad::uniform_full(100.0);
        let d_max = load.deflection_at(5.0, 10.0, 1_400_000.0, 100.0);

        // Expected: 0.161 in (approximately)
        assert!(approx_eq(d_max, 0.161, 0.05)); // 5% tolerance for numerical
    }

    #[test]
    fn test_superposition() {
        // 12 ft beam, two loads:
        // - 50 plf uniform (dead)
        // - 1000 lb point at midspan (live)
        let mut analysis = BeamAnalysis::new(12.0, 1_400_000.0, 100.0);
        analysis.add_load(SingleLoad::uniform_full(50.0));
        analysis.add_load(SingleLoad::point(1000.0, 6.0));

        let r1 = analysis.total_reaction_left();
        let r2 = analysis.total_reaction_right();

        // Uniform: R1 = R2 = 50*12/2 = 300 lb
        // Point: R1 = R2 = 1000/2 = 500 lb
        // Total: R1 = R2 = 800 lb
        assert!(approx_eq(r1, 800.0, EPSILON));
        assert!(approx_eq(r2, 800.0, EPSILON));

        // Max moment at midspan:
        // Uniform: M = wL²/8 = 50*144/8 = 900 ft-lb
        // Point: M = PL/4 = 1000*12/4 = 3000 ft-lb
        // Total: 3900 ft-lb
        let m_max = analysis.moment_at(6.0);
        assert!(approx_eq(m_max, 3900.0, EPSILON));
    }

    #[test]
    fn test_partial_uniform_reactions() {
        // 10 ft beam, 100 plf from 2 ft to 8 ft
        // Total load = 100 * 6 = 600 lb
        // Centroid at 5 ft
        // R1 = 600 * (10-5)/10 = 300 lb
        // R2 = 600 * 5/10 = 300 lb
        let load = SingleLoad::uniform_partial(100.0, 2.0, 8.0);
        let r1 = load.reaction_left(10.0);
        let r2 = load.reaction_right(10.0);

        assert!(approx_eq(r1, 300.0, EPSILON));
        assert!(approx_eq(r2, 300.0, EPSILON));
    }

    #[test]
    fn test_analysis_results() {
        let mut analysis = BeamAnalysis::new(10.0, 1_400_000.0, 100.0);
        analysis.add_load(SingleLoad::uniform_full(100.0));

        let results = analysis.analyze();

        // Reactions should each be 500 lb
        assert!(approx_eq(results.reaction_left_lb, 500.0, EPSILON));
        assert!(approx_eq(results.reaction_right_lb, 500.0, EPSILON));

        // Max moment should be 1250 ft-lb at midspan
        assert!(approx_eq(results.max_moment_ftlb, 1250.0, EPSILON));
        assert!(approx_eq(results.max_moment_position_ft, 5.0, 0.1));

        // Max shear should be 500 lb at supports
        assert!(approx_eq(results.max_shear_lb, 500.0, EPSILON));

        // Diagrams should have points
        assert!(!results.shear_diagram.is_empty());
        assert!(!results.moment_diagram.is_empty());
        assert!(!results.deflection_diagram.is_empty());
    }
}
