//! # Moment Distribution Method (Hardy Cross)
//!
//! Implementation of the Hardy Cross moment distribution method for
//! analyzing statically indeterminate continuous beams.
//!
//! ## Algorithm Overview
//!
//! 1. Calculate distribution factors at each joint
//! 2. Calculate fixed-end moments (FEM) for all spans under applied loads
//! 3. At each joint, distribute unbalanced moment to adjacent spans
//! 4. Carry over 50% of distributed moment to far end
//! 5. Iterate until convergence
//!
//! ## References
//!
//! - "Structural Analysis" by R.C. Hibbeler, Chapter 11
//! - "Moment Distribution" by Hardy Cross (1930)

use crate::calculations::continuous_beam::{ContinuousBeamInput, SupportType};
use crate::equations::beam::{fem_partial_uniform, fem_point_load, fem_uniform_full};
use crate::errors::CalcResult;
use crate::loads::{LoadDistribution, LoadType};

/// Maximum iterations for moment distribution
const MAX_ITERATIONS: usize = 50;

/// Convergence tolerance (ft-lb)
const TOLERANCE: f64 = 0.1;

/// Data for a single span in moment distribution
#[derive(Debug, Clone)]
pub struct SpanData {
    /// Span length (ft)
    pub length_ft: f64,
    /// Flexural stiffness EI (lb-in²)
    pub ei: f64,
    /// Stiffness factor K = EI/L (lb-in) - calculated for inches
    pub k: f64,
    /// Fixed-end moment at left (ft-lb)
    pub fem_left: f64,
    /// Fixed-end moment at right (ft-lb)
    pub fem_right: f64,
    /// Final moment at left after distribution (ft-lb)
    pub moment_left: f64,
    /// Final moment at right after distribution (ft-lb)
    pub moment_right: f64,
}

/// Data for a joint (node) in moment distribution
#[derive(Debug, Clone)]
pub struct JointData {
    /// Support type at this joint
    pub support_type: SupportType,
    /// Distribution factors for connected spans [left_span, right_span]
    /// Values sum to 1.0 at interior joints
    pub distribution_factors: Vec<f64>,
    /// Indices of spans connected to this joint
    pub connected_spans: Vec<usize>,
    /// Whether this is the left end (index 0) of connected span
    pub is_left_end: Vec<bool>,
}

/// Moment distribution solver for continuous beams
#[derive(Debug)]
pub struct MomentDistribution {
    /// Number of spans
    n_spans: usize,
    /// Span data (geometry and moments)
    spans: Vec<SpanData>,
    /// Joint data (support conditions and distribution factors)
    joints: Vec<JointData>,
}

impl MomentDistribution {
    /// Create a new moment distribution solver from continuous beam input
    pub fn from_input(input: &ContinuousBeamInput) -> CalcResult<Self> {
        let n_spans = input.span_count();
        let n_joints = input.node_count();

        // Build span data. Engineered-wood lookups inside `span.ei()` can fail if
        // the build script didn't emit data for a referenced variant; propagate
        // that as a CalcError instead of panicking.
        let spans: Vec<SpanData> = input
            .spans
            .iter()
            .map(|span| {
                let l_in = span.length_ft * 12.0;
                let ei = span.ei()?;
                Ok(SpanData {
                    length_ft: span.length_ft,
                    ei,
                    k: ei / l_in, // Basic stiffness = EI/L
                    fem_left: 0.0,
                    fem_right: 0.0,
                    moment_left: 0.0,
                    moment_right: 0.0,
                })
            })
            .collect::<CalcResult<Vec<_>>>()?;

        // Build joint data with distribution factors
        let mut joints: Vec<JointData> = Vec::with_capacity(n_joints);

        for j in 0..n_joints {
            let support_type = input.supports[j];
            let mut connected_spans = Vec::new();
            let mut is_left_end = Vec::new();
            let mut stiffnesses = Vec::new();

            // Left span (if exists)
            if j > 0 {
                let span_idx = j - 1;
                connected_spans.push(span_idx);
                is_left_end.push(false); // This joint is at right end of left span

                // Adjust stiffness for far-end condition
                let far_end = input.supports[j - 1];
                let k = if far_end == SupportType::Pinned || far_end == SupportType::Roller {
                    spans[span_idx].k * 0.75 // 3EI/L for pinned far end
                } else {
                    spans[span_idx].k // 4EI/L for fixed/continuous far end
                };
                stiffnesses.push(k);
            }

            // Right span (if exists)
            if j < n_spans {
                let span_idx = j;
                connected_spans.push(span_idx);
                is_left_end.push(true); // This joint is at left end of right span

                // Adjust stiffness for far-end condition
                let far_end = input.supports[j + 1];
                let k = if far_end == SupportType::Pinned || far_end == SupportType::Roller {
                    spans[span_idx].k * 0.75 // 3EI/L for pinned far end
                } else {
                    spans[span_idx].k // 4EI/L for fixed/continuous far end
                };
                stiffnesses.push(k);
            }

            // Calculate distribution factors
            let total_k: f64 = stiffnesses.iter().sum();
            let distribution_factors = if total_k > 0.0 && support_type != SupportType::Fixed {
                stiffnesses.iter().map(|k| k / total_k).collect()
            } else {
                // Fixed joint or no stiffness: no distribution (absorb all moment)
                vec![0.0; stiffnesses.len()]
            };

            joints.push(JointData {
                support_type,
                distribution_factors,
                connected_spans,
                is_left_end,
            });
        }

        Ok(Self {
            n_spans,
            spans,
            joints,
        })
    }

    /// Add loads from the load case to compute fixed-end moments
    pub fn add_loads(&mut self, input: &ContinuousBeamInput, load_factors: &[(LoadType, f64)]) {
        // Reset FEMs
        for span in &mut self.spans {
            span.fem_left = 0.0;
            span.fem_right = 0.0;
        }

        let node_positions = input.node_positions();

        // Process each discrete load
        for load in &input.load_case.loads {
            // Get load factor for this type
            let factor = load_factors
                .iter()
                .find(|(lt, _)| *lt == load.load_type)
                .map(|(_, f)| *f)
                .unwrap_or(0.0);

            if factor.abs() < 1e-10 {
                continue;
            }

            let magnitude = load.effective_magnitude() * factor;

            match &load.distribution {
                LoadDistribution::UniformFull => {
                    // Apply to all spans
                    for (i, span) in self.spans.iter_mut().enumerate() {
                        let (fem_a, fem_b) = fem_uniform_full(magnitude, span.length_ft);
                        span.fem_left += fem_a;
                        span.fem_right += fem_b;
                        // Note: FEM signs are negative at left, positive at right
                        // for our convention (hogging = negative)
                        let _ = i; // span index unused here but could be for partial
                    }
                }
                LoadDistribution::Point { position_ft } => {
                    // Find which span contains this point
                    for (i, span) in self.spans.iter_mut().enumerate() {
                        let span_start = node_positions[i];
                        let span_end = node_positions[i + 1];

                        if *position_ft >= span_start && *position_ft <= span_end {
                            let local_pos = position_ft - span_start;
                            let (fem_a, fem_b) =
                                fem_point_load(magnitude, local_pos, span.length_ft);
                            span.fem_left += fem_a;
                            span.fem_right += fem_b;
                            break;
                        }
                    }
                }
                LoadDistribution::UniformPartial { start_ft, end_ft } => {
                    // May span multiple elements
                    for (i, span) in self.spans.iter_mut().enumerate() {
                        let span_start = node_positions[i];
                        let span_end = node_positions[i + 1];

                        // Check for overlap
                        if *start_ft < span_end && *end_ft > span_start {
                            let local_start = (start_ft - span_start).max(0.0);
                            let local_end = (end_ft - span_start).min(span.length_ft);

                            if local_end > local_start {
                                let (fem_a, fem_b) = fem_partial_uniform(
                                    magnitude,
                                    local_start,
                                    local_end,
                                    span.length_ft,
                                );
                                span.fem_left += fem_a;
                                span.fem_right += fem_b;
                            }
                        }
                    }
                }
                LoadDistribution::Moment { position_ft } => {
                    // Applied moment - find which span and add directly
                    // For now, treat as point moment (no FEM effect, add to final)
                    // This is an approximation; full treatment would require
                    // separate handling in the moment diagram
                    let _ = position_ft;
                    let _ = magnitude;
                }
                LoadDistribution::Trapezoidal { .. } => {
                    // Approximate as partial uniform with average magnitude
                    // Already handled in beam_analysis conversion
                }
            }
        }

        // Add self-weight if enabled
        if input.load_case.include_self_weight {
            // Find dead load factor
            let dead_factor = load_factors
                .iter()
                .find(|(lt, _)| *lt == LoadType::Dead)
                .map(|(_, f)| *f)
                .unwrap_or(1.0);

            for (i, span_input) in input.spans.iter().enumerate() {
                let sw = span_input.self_weight_plf() * dead_factor;
                let (fem_a, fem_b) = fem_uniform_full(sw, self.spans[i].length_ft);
                self.spans[i].fem_left += fem_a;
                self.spans[i].fem_right += fem_b;
            }
        }
    }

    /// Run moment distribution iteration
    ///
    /// Returns true if converged, false if max iterations reached
    pub fn solve(&mut self) -> bool {
        // Initialize moments to FEM
        for span in &mut self.spans {
            span.moment_left = span.fem_left;
            span.moment_right = span.fem_right;
        }

        // Handle special cases first
        if self.n_spans == 0 {
            return true;
        }

        // For single-span beams, handle special cases
        if self.n_spans == 1 {
            let left_support = self.joints[0].support_type;
            let right_support = self.joints[1].support_type;

            let left_is_pinned =
                left_support == SupportType::Pinned || left_support == SupportType::Roller;
            let right_is_pinned =
                right_support == SupportType::Pinned || right_support == SupportType::Roller;
            let left_is_fixed = left_support == SupportType::Fixed;
            let right_is_fixed = right_support == SupportType::Fixed;
            let left_is_free = left_support == SupportType::Free;
            let right_is_free = right_support == SupportType::Free;

            // Case 1: Both fixed - FEM are the final moments (no distribution)
            if left_is_fixed && right_is_fixed {
                return true;
            }

            // Case 2: Both pinned (simply-supported) - zero moments at both ends
            if left_is_pinned && right_is_pinned {
                self.spans[0].moment_left = 0.0;
                self.spans[0].moment_right = 0.0;
                return true;
            }

            // Case 3: Fixed-Pinned (propped cantilever)
            // Release the pinned end and carry over to fixed end
            if left_is_fixed && right_is_pinned {
                // Pinned end has zero moment, fixed end gets carryover
                let release = -self.spans[0].moment_right;
                self.spans[0].moment_right = 0.0;
                self.spans[0].moment_left += release * 0.5;
                return true;
            }

            // Case 4: Pinned-Fixed (propped cantilever, reversed)
            if left_is_pinned && right_is_fixed {
                let release = -self.spans[0].moment_left;
                self.spans[0].moment_left = 0.0;
                self.spans[0].moment_right += release * 0.5;
                return true;
            }

            // Case 5: Cantilever (Fixed-Free or Free-Fixed)
            if left_is_fixed && right_is_free {
                // Free end has zero moment, fixed end keeps FEM
                self.spans[0].moment_right = 0.0;
                return true;
            }
            if left_is_free && right_is_fixed {
                self.spans[0].moment_left = 0.0;
                return true;
            }

            // Case 6: Any remaining combinations (e.g., free-pinned) - just zero the free/pinned ends
            if left_is_free || left_is_pinned {
                self.spans[0].moment_left = 0.0;
            }
            if right_is_free || right_is_pinned {
                self.spans[0].moment_right = 0.0;
            }

            return true;
        }

        // First, release moments at exterior supports that can't resist moment
        // (Pinned, Roller, and Free all have zero moment at the end)
        // Left exterior joint
        let left_joint = &self.joints[0];
        let left_cant_resist = matches!(
            left_joint.support_type,
            SupportType::Pinned | SupportType::Roller | SupportType::Free
        );
        if left_cant_resist {
            // Left exterior is connected to span 0's left end
            if !left_joint.connected_spans.is_empty() {
                let span_idx = left_joint.connected_spans[0];
                let release = -self.spans[span_idx].moment_left;
                self.spans[span_idx].moment_left = 0.0;
                // Carry over to the other end of this span (only for Pinned/Roller, not Free)
                // Free ends don't transfer moment through carryover
                if left_joint.support_type != SupportType::Free {
                    self.spans[span_idx].moment_right += release * 0.5;
                }
            }
        }

        // Right exterior joint
        let n_joints = self.joints.len();
        let right_joint = &self.joints[n_joints - 1];
        let right_cant_resist = matches!(
            right_joint.support_type,
            SupportType::Pinned | SupportType::Roller | SupportType::Free
        );
        if right_cant_resist {
            // Right exterior is connected to last span's right end
            if !right_joint.connected_spans.is_empty() {
                let span_idx = right_joint.connected_spans[0];
                let release = -self.spans[span_idx].moment_right;
                self.spans[span_idx].moment_right = 0.0;
                // Carry over to the other end of this span (only for Pinned/Roller, not Free)
                if right_joint.support_type != SupportType::Free {
                    self.spans[span_idx].moment_left += release * 0.5;
                }
            }
        }

        // Multi-span moment distribution iteration
        for _iteration in 0..MAX_ITERATIONS {
            let mut max_unbalance = 0.0f64;

            // Process each interior joint
            for j in 1..self.joints.len() - 1 {
                let joint = &self.joints[j];

                // Skip fixed joints (absorb all moment)
                if joint.support_type == SupportType::Fixed {
                    continue;
                }

                // Calculate unbalanced moment at this joint
                let mut unbalanced = 0.0;
                for (idx, &span_idx) in joint.connected_spans.iter().enumerate() {
                    let moment = if joint.is_left_end[idx] {
                        self.spans[span_idx].moment_left
                    } else {
                        self.spans[span_idx].moment_right
                    };
                    unbalanced += moment;
                }

                max_unbalance = max_unbalance.max(unbalanced.abs());

                if unbalanced.abs() < TOLERANCE {
                    continue;
                }

                // Distribute the unbalanced moment
                for (idx, &span_idx) in joint.connected_spans.iter().enumerate() {
                    let df = joint.distribution_factors[idx];
                    let distributed = -unbalanced * df;

                    // Apply to near end
                    if joint.is_left_end[idx] {
                        self.spans[span_idx].moment_left += distributed;
                    } else {
                        self.spans[span_idx].moment_right += distributed;
                    }

                    // Carry over to far end (50% for continuous, 0% for pinned)
                    let far_joint = if joint.is_left_end[idx] {
                        span_idx + 1
                    } else {
                        span_idx
                    };

                    let far_support = if far_joint < self.joints.len() {
                        self.joints[far_joint].support_type
                    } else {
                        SupportType::Free
                    };

                    let carryover = if far_support == SupportType::Fixed {
                        0.5 // Fixed far end - full carryover
                    } else if matches!(
                        far_support,
                        SupportType::Pinned | SupportType::Roller | SupportType::Free
                    ) {
                        0.0 // Pinned/Roller/Free far end - no carryover (can't resist moment)
                    } else {
                        0.5 // Continuous - full carryover
                    };

                    // Apply carryover
                    if carryover > 0.0 {
                        if joint.is_left_end[idx] {
                            self.spans[span_idx].moment_right += distributed * carryover;
                        } else {
                            self.spans[span_idx].moment_left += distributed * carryover;
                        }
                    }
                }
            }

            // Check convergence
            if max_unbalance < TOLERANCE {
                return true;
            }
        }

        false // Did not converge
    }

    /// Get final moments at each span end
    pub fn get_end_moments(&self) -> Vec<(f64, f64)> {
        self.spans
            .iter()
            .map(|span| (span.moment_left, span.moment_right))
            .collect()
    }

    /// Get support moments (moment at each joint)
    ///
    /// For interior joints, this returns the beam moment at the support location.
    /// At equilibrium, moments from both sides are equal and opposite, so we take
    /// the moment from the left span's right end (or right span's left end with sign flip).
    pub fn get_support_moments(&self) -> Vec<f64> {
        let mut moments = Vec::with_capacity(self.joints.len());

        for j in 0..self.joints.len() {
            let joint = &self.joints[j];

            if joint.connected_spans.is_empty() {
                moments.push(0.0);
                continue;
            }

            // For exterior joints (only one span connected), use that span's end moment
            // For interior joints, use the right end moment of the left span
            // (which equals negative of left end moment of right span at equilibrium)
            if joint.connected_spans.len() == 1 {
                // Exterior joint
                let span_idx = joint.connected_spans[0];
                let moment = if joint.is_left_end[0] {
                    self.spans[span_idx].moment_left
                } else {
                    self.spans[span_idx].moment_right
                };
                moments.push(moment);
            } else {
                // Interior joint - take the moment from the left span's right end
                // This is the conventional "support moment" in continuous beam analysis
                // Find the span where this joint is at the right end (is_left_end = false)
                let mut moment = 0.0;
                for (idx, &span_idx) in joint.connected_spans.iter().enumerate() {
                    if !joint.is_left_end[idx] {
                        // This is the left span's right end
                        moment = self.spans[span_idx].moment_right;
                        break;
                    }
                }
                moments.push(moment);
            }
        }

        moments
    }
}

/// Result of moment distribution analysis
#[derive(Debug, Clone)]
pub struct DistributionResult {
    /// Moment at left end of each span (ft-lb)
    pub span_moments_left: Vec<f64>,
    /// Moment at right end of each span (ft-lb)
    pub span_moments_right: Vec<f64>,
    /// Support moment at each node (ft-lb)
    pub support_moments: Vec<f64>,
    /// Whether solution converged
    pub converged: bool,
}

/// Analyze a continuous beam using moment distribution
pub fn analyze_moment_distribution(
    input: &ContinuousBeamInput,
    load_factors: &[(LoadType, f64)],
) -> CalcResult<DistributionResult> {
    let mut solver = MomentDistribution::from_input(input)?;
    solver.add_loads(input, load_factors);
    let converged = solver.solve();

    let end_moments = solver.get_end_moments();
    let support_moments = solver.get_support_moments();

    Ok(DistributionResult {
        span_moments_left: end_moments.iter().map(|(l, _)| *l).collect(),
        span_moments_right: end_moments.iter().map(|(_, r)| *r).collect(),
        support_moments,
        converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loads::{DiscreteLoad, EnhancedLoadCase};
    use crate::materials::{Material, WoodGrade, WoodMaterial, WoodSpecies};

    fn test_material() -> Material {
        Material::SawnLumber(WoodMaterial::new(
            WoodSpecies::DouglasFirLarch,
            WoodGrade::No2,
        ))
    }

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_two_span_equal_uniform() {
        // Two equal spans, 10 ft each, uniform load 100 plf
        // Expected: M at center support = -wL²/8 = -1250 ft-lb
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0));

        let input = ContinuousBeamInput {
            label: "Two-span test".to_string(),
            spans: vec![
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![
                SupportType::Pinned,
                SupportType::Pinned,
                SupportType::Pinned,
            ],
            load_case,
            ..Default::default()
        };

        let load_factors = vec![(LoadType::Dead, 1.0)];
        let result = analyze_moment_distribution(&input, &load_factors).unwrap();

        assert!(result.converged, "Should converge");

        // Center support moment should be approximately -wL²/8 = -1250 ft-lb
        // Due to moment distribution sign convention, this may be positive
        let center_moment = result.support_moments[1].abs();
        assert!(
            approx_eq(center_moment, 1250.0, 100.0),
            "Center moment = {} (expected ~1250)",
            center_moment
        );

        // End moments should be near zero (pinned ends)
        assert!(
            result.span_moments_left[0].abs() < 50.0,
            "Left end moment should be ~0, got {}",
            result.span_moments_left[0]
        );
    }

    #[test]
    fn test_single_span_simply_supported() {
        // Single span simply-supported - should have zero end moments
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0));

        let input = ContinuousBeamInput::simple_span(
            "Simple span",
            10.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let load_factors = vec![(LoadType::Dead, 1.0)];
        let result = analyze_moment_distribution(&input, &load_factors).unwrap();

        assert!(result.converged, "Should converge");

        // Both ends should have zero moment (pinned)
        assert!(
            result.span_moments_left[0].abs() < 10.0,
            "Left moment should be ~0, got {}",
            result.span_moments_left[0]
        );
        assert!(
            result.span_moments_right[0].abs() < 10.0,
            "Right moment should be ~0, got {}",
            result.span_moments_right[0]
        );
    }

    #[test]
    fn test_single_span_fixed_fixed() {
        // Single span fixed-fixed
        // FEM = wL²/12 = 100 * 100 / 12 = 833.33 ft-lb at each end
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0));

        let input = ContinuousBeamInput::fixed_fixed(
            "Fixed-fixed",
            10.0,
            1.5,
            9.25,
            test_material(),
            load_case,
        );

        let load_factors = vec![(LoadType::Dead, 1.0)];
        let result = analyze_moment_distribution(&input, &load_factors).unwrap();

        assert!(result.converged, "Should converge");

        // End moments should be -wL²/12 = -833.33 ft-lb (hogging)
        let left_moment = result.span_moments_left[0].abs();
        let right_moment = result.span_moments_right[0].abs();

        assert!(
            approx_eq(left_moment, 833.33, 50.0),
            "Left moment = {} (expected ~833.33)",
            left_moment
        );
        assert!(
            approx_eq(right_moment, 833.33, 50.0),
            "Right moment = {} (expected ~833.33)",
            right_moment
        );
    }

    #[test]
    fn test_two_span_fixed_left_exterior() {
        // Bug repro: Two-span beam with Fixed left exterior should produce non-zero results
        // [Fixed, Pinned, Pinned]
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0));

        let input = ContinuousBeamInput {
            label: "Fixed-Pin-Pin".to_string(),
            spans: vec![
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![SupportType::Fixed, SupportType::Pinned, SupportType::Pinned],
            load_case,
            ..Default::default()
        };

        let load_factors = vec![(LoadType::Dead, 1.0)];
        let result = analyze_moment_distribution(&input, &load_factors).unwrap();

        assert!(result.converged, "Should converge");

        // Left end (Fixed) should have non-zero moment
        assert!(
            result.span_moments_left[0].abs() > 100.0,
            "Fixed left end should have significant moment, got {}",
            result.span_moments_left[0]
        );

        // Right end of span 2 (Pinned) should have ~zero moment
        assert!(
            result.span_moments_right[1].abs() < 50.0,
            "Pinned right end should have ~0 moment, got {}",
            result.span_moments_right[1]
        );

        // Interior support should have non-zero moment
        assert!(
            result.support_moments[1].abs() > 100.0,
            "Interior support should have significant moment, got {}",
            result.support_moments[1]
        );
    }

    #[test]
    fn test_two_span_free_left_exterior() {
        // Bug repro: Two-span beam with Free left exterior (cantilever + continuation)
        // [Free, Pinned, Pinned] - This is an unusual configuration but should not produce zeros
        // Note: This requires at least 2 vertical supports for stability
        let load_case =
            EnhancedLoadCase::new("Test").with_load(DiscreteLoad::uniform(LoadType::Dead, 100.0));

        let input = ContinuousBeamInput {
            label: "Free-Pin-Pin".to_string(),
            spans: vec![
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
                super::super::continuous_beam::SpanSegment::new(10.0, 1.5, 9.25, test_material()),
            ],
            supports: vec![SupportType::Free, SupportType::Pinned, SupportType::Pinned],
            load_case,
            ..Default::default()
        };

        let load_factors = vec![(LoadType::Dead, 1.0)];
        let result = analyze_moment_distribution(&input, &load_factors).unwrap();

        assert!(result.converged, "Should converge");

        // Free left end should have zero moment (can't resist moment)
        assert!(
            result.span_moments_left[0].abs() < 10.0,
            "Free left end should have ~0 moment, got {}",
            result.span_moments_left[0]
        );

        // Right end of span 2 (Pinned) should have ~zero moment
        assert!(
            result.span_moments_right[1].abs() < 50.0,
            "Pinned right end should have ~0 moment, got {}",
            result.span_moments_right[1]
        );

        // Interior support should have non-zero moment (from cantilever effect)
        // For a uniform load on a cantilever portion, M = wL²/2 at the support
        // Span 1 has 100 plf * 10 ft overhang, M = 100 * 10² / 2 = 5000 ft-lb
        assert!(
            result.support_moments[1].abs() > 1000.0,
            "Interior support should have significant moment from cantilever, got {}",
            result.support_moments[1]
        );
    }
}
