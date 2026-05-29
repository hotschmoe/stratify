//! Canvas drawing utilities for beam diagrams
//!
//! Renders beam schematics with supports, shear diagrams, moment diagrams,
//! and deflection diagrams.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Theme};

use calc_core::calculations::continuous_beam::{
    ContinuousBeamInput, ContinuousBeamResult, SupportType,
};
use calc_core::loads::{LoadDistribution, LoadType};

use crate::Message;

/// Simplified load info for diagram drawing
#[derive(Debug, Clone)]
pub struct DiagramLoad {
    /// Load index (1-based for display)
    pub index: usize,
    /// Load type (D, L, S, W, etc.)
    pub load_type: LoadType,
    /// Distribution pattern
    pub distribution: LoadDistribution,
    /// Magnitude (plf for uniform, lb for point)
    pub magnitude: f64,
}

/// Data needed to draw beam diagrams
pub struct BeamDiagramData {
    pub total_length_ft: f64,
    #[allow(dead_code)]
    pub load_plf: f64,
    pub max_shear_lb: f64,
    pub max_moment_ftlb: f64,
    pub max_deflection_in: f64,
    // Multi-span support
    pub span_lengths_ft: Vec<f64>,
    pub support_types: Vec<SupportType>,
    pub reactions: Vec<f64>,
    // Pre-computed diagram points from analysis
    pub shear_diagram: Vec<(f64, f64)>,
    pub moment_diagram: Vec<(f64, f64)>,
    pub deflection_diagram: Vec<(f64, f64)>,
    // Discrete loads for individual load visualization
    pub discrete_loads: Vec<DiagramLoad>,
}

impl BeamDiagramData {
    pub fn from_calc(input: &ContinuousBeamInput, result: &ContinuousBeamResult) -> Self {
        // Convert discrete loads for diagram display
        let discrete_loads: Vec<DiagramLoad> = input
            .load_case
            .loads
            .iter()
            .enumerate()
            .map(|(i, load)| DiagramLoad {
                index: i + 1,
                load_type: load.load_type,
                distribution: load.distribution.clone(),
                magnitude: load.effective_magnitude(),
            })
            .collect();

        Self {
            total_length_ft: input.total_length_ft(),
            load_plf: input.load_case.total_uniform_plf(),
            max_shear_lb: result.max_shear_lb,
            max_moment_ftlb: result.max_positive_moment_ftlb,
            max_deflection_in: result.max_deflection_in,
            span_lengths_ft: input.spans.iter().map(|s| s.length_ft).collect(),
            support_types: input.supports.clone(),
            reactions: result.reactions.clone(),
            shear_diagram: result.shear_diagram.clone(),
            moment_diagram: result.moment_diagram.clone(),
            deflection_diagram: result.deflection_diagram.clone(),
            discrete_loads,
        }
    }

    /// Check if this is a multi-span beam
    #[allow(dead_code)]
    pub fn is_multi_span(&self) -> bool {
        self.span_lengths_ft.len() > 1
    }

    /// Get node positions (cumulative span lengths starting from 0)
    pub fn node_positions_ft(&self) -> Vec<f64> {
        let mut positions = vec![0.0];
        let mut cumulative = 0.0;
        for len in &self.span_lengths_ft {
            cumulative += len;
            positions.push(cumulative);
        }
        positions
    }
}

/// Canvas program for drawing beam diagrams
pub struct BeamDiagram {
    data: BeamDiagramData,
}

impl BeamDiagram {
    pub fn new(data: BeamDiagramData) -> Self {
        Self { data }
    }

    /// Draw vertical dashed lines at interior support positions (for multi-span beams)
    fn draw_support_lines(
        &self,
        frame: &mut Frame,
        x: f32,
        width: f32,
        top_y: f32,
        bottom_y: f32,
        color: Color,
    ) {
        if self.data.span_lengths_ft.len() <= 1 {
            return; // Single span - no interior supports
        }

        let node_positions = self.data.node_positions_ft();
        let total_length = self.data.total_length_ft;

        // Draw vertical lines at interior supports (skip first and last)
        for &node_ft in node_positions.iter().skip(1).take(node_positions.len() - 2) {
            let node_x = x + (node_ft / total_length) as f32 * width;

            // Draw dashed vertical line
            let dash_length = 4.0;
            let gap_length = 3.0;
            let mut y = top_y;
            while y < bottom_y {
                let dash_end = (y + dash_length).min(bottom_y);
                let dash = Path::line(Point::new(node_x, y), Point::new(node_x, dash_end));
                frame.stroke(&dash, Stroke::default().with_color(color).with_width(1.0));
                y += dash_length + gap_length;
            }
        }
    }

    /// Find extrema (max/min) values per span from diagram data
    fn find_span_extrema(&self, diagram: &[(f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
        // Returns Vec of (span_start, span_end, max_value, min_value) per span
        let node_positions = self.data.node_positions_ft();
        let mut results = Vec::new();

        for i in 0..self.data.span_lengths_ft.len() {
            let span_start = node_positions[i];
            let span_end = node_positions[i + 1];

            let span_points: Vec<_> = diagram
                .iter()
                .filter(|(pos, _)| *pos >= span_start && *pos <= span_end)
                .collect();

            if span_points.is_empty() {
                results.push((span_start, span_end, 0.0, 0.0));
                continue;
            }

            let max_val = span_points
                .iter()
                .map(|(_, v)| *v)
                .fold(f64::NEG_INFINITY, f64::max);
            let min_val = span_points
                .iter()
                .map(|(_, v)| *v)
                .fold(f64::INFINITY, f64::min);

            results.push((span_start, span_end, max_val, min_val));
        }

        results
    }

    /// Find position of extrema value within a span
    fn find_extrema_position(
        &self,
        diagram: &[(f64, f64)],
        span_start: f64,
        span_end: f64,
        target_value: f64,
    ) -> Option<f64> {
        diagram
            .iter()
            .filter(|(pos, _)| *pos >= span_start && *pos <= span_end)
            .find(|(_, v)| (*v - target_value).abs() < 1e-6)
            .map(|(pos, _)| *pos)
    }

    fn draw_beam_schematic(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) {
        // Position beam lower to give more space for load arrows above
        // At 70% down, with section_height=100px, this gives ~70px for loads
        let beam_y = y + height * 0.70;
        let beam_thickness = 4.0;
        let support_size = 10.0;
        let total_length = self.data.total_length_ft;
        let reaction_color = Color::from_rgb(0.7, 0.2, 0.2);

        // Draw beam line
        let beam = Path::line(Point::new(x, beam_y), Point::new(x + width, beam_y));
        frame.stroke(
            &beam,
            Stroke::default()
                .with_color(color)
                .with_width(beam_thickness),
        );

        // Get node positions
        let node_positions = self.data.node_positions_ft();

        // Draw supports at each node
        for (i, &node_ft) in node_positions.iter().enumerate() {
            let node_x = x + (node_ft / total_length) as f32 * width;
            let support_type = self
                .data
                .support_types
                .get(i)
                .copied()
                .unwrap_or(SupportType::Pinned);

            self.draw_support(
                frame,
                node_x,
                beam_y + beam_thickness / 2.0,
                support_size,
                support_type,
                color,
            );

            // Draw reaction arrow and label for supported nodes (not Free)
            if support_type.restrains_vertical() {
                let reaction = self.data.reactions.get(i).copied().unwrap_or(0.0);
                if reaction.abs() > 1.0 {
                    let reaction_arrow_length = height * 0.15;
                    let reaction_start_y = beam_y + support_size + 8.0;

                    // Draw reaction arrow (upward for positive, downward for negative)
                    let (arrow_start, arrow_end) = if reaction >= 0.0 {
                        (reaction_start_y + reaction_arrow_length, reaction_start_y)
                    } else {
                        (reaction_start_y, reaction_start_y + reaction_arrow_length)
                    };

                    let reaction_arrow = Path::line(
                        Point::new(node_x, arrow_start),
                        Point::new(node_x, arrow_end),
                    );
                    frame.stroke(
                        &reaction_arrow,
                        Stroke::default().with_color(reaction_color).with_width(2.0),
                    );

                    // Arrow head
                    let head_y = arrow_end;
                    let head_dir = if reaction >= 0.0 { 1.0 } else { -1.0 };
                    let head = Path::new(|builder| {
                        builder.move_to(Point::new(node_x, head_y));
                        builder.line_to(Point::new(node_x - 3.0, head_y + head_dir * 6.0));
                        builder.move_to(Point::new(node_x, head_y));
                        builder.line_to(Point::new(node_x + 3.0, head_y + head_dir * 6.0));
                    });
                    frame.stroke(
                        &head,
                        Stroke::default().with_color(reaction_color).with_width(2.0),
                    );

                    // Reaction label - show R_1, R_2, etc.
                    let label = format!("R_{} = {:.0}", i + 1, reaction.abs());
                    let label_x = if i == 0 { node_x + 3.0 } else { node_x - 45.0 };
                    let reaction_text = Text {
                        content: label,
                        position: Point::new(
                            label_x,
                            reaction_start_y + reaction_arrow_length + 2.0,
                        ),
                        color: reaction_color,
                        size: iced::Pixels(8.0),
                        ..Text::default()
                    };
                    frame.fill_text(reaction_text);
                }
            }
        }

        // Draw individual loads with stacking
        self.draw_discrete_loads(frame, x, y, width, beam_y, total_length);

        // Span labels - one for each span
        for (i, span_len) in self.data.span_lengths_ft.iter().enumerate() {
            let start = node_positions[i];
            let end = node_positions[i + 1];
            let mid = (start + end) / 2.0;
            let span_x = x + (mid / total_length) as f32 * width;

            let span_text = Text {
                content: format!("L{} = {:.1}'", i + 1, span_len),
                position: Point::new(span_x, beam_y + support_size + 5.0),
                color,
                size: iced::Pixels(8.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                ..Text::default()
            };
            frame.fill_text(span_text);
        }
    }

    /// Draw a support symbol at the given position
    fn draw_support(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        size: f32,
        support_type: SupportType,
        color: Color,
    ) {
        match support_type {
            SupportType::Pinned => {
                // Triangle (filled)
                let support = Path::new(|builder| {
                    builder.move_to(Point::new(x, y));
                    builder.line_to(Point::new(x - size / 2.0, y + size));
                    builder.line_to(Point::new(x + size / 2.0, y + size));
                    builder.close();
                });
                frame.fill(&support, color);
            }
            SupportType::Roller => {
                // Triangle with circle underneath
                let triangle = Path::new(|builder| {
                    builder.move_to(Point::new(x, y));
                    builder.line_to(Point::new(x - size / 2.0, y + size * 0.7));
                    builder.line_to(Point::new(x + size / 2.0, y + size * 0.7));
                    builder.close();
                });
                frame.stroke(
                    &triangle,
                    Stroke::default().with_color(color).with_width(2.0),
                );

                // Circle
                let circle_radius = size * 0.15;
                let circle = Path::circle(
                    Point::new(x, y + size * 0.7 + circle_radius + 1.0),
                    circle_radius,
                );
                frame.stroke(&circle, Stroke::default().with_color(color).with_width(2.0));
            }
            SupportType::Fixed => {
                // Filled rectangle with hatching
                let rect_height = size;
                let rect_width = size * 0.3;

                let rect = Path::new(|builder| {
                    builder.move_to(Point::new(x - rect_width, y));
                    builder.line_to(Point::new(x + rect_width, y));
                    builder.line_to(Point::new(x + rect_width, y + rect_height));
                    builder.line_to(Point::new(x - rect_width, y + rect_height));
                    builder.close();
                });
                frame.fill(&rect, color);

                // Hatching lines
                for i in 0..3 {
                    let hatch_y = y + (i as f32 + 0.5) * rect_height / 3.0;
                    let hatch = Path::line(
                        Point::new(x - rect_width - 3.0, hatch_y + 3.0),
                        Point::new(x + rect_width + 3.0, hatch_y - 3.0),
                    );
                    frame.stroke(&hatch, Stroke::default().with_color(color).with_width(1.0));
                }
            }
            SupportType::Free => {
                // No support symbol - maybe a small dot to show the end
                let dot = Path::circle(Point::new(x, y + 2.0), 2.0);
                frame.stroke(&dot, Stroke::default().with_color(color).with_width(1.5));
            }
        }
    }

    /// Get color for a load type
    fn load_type_color(load_type: LoadType) -> Color {
        match load_type {
            LoadType::Dead => Color::from_rgb(0.4, 0.4, 0.4), // Dark gray
            LoadType::Live => Color::from_rgb(0.2, 0.5, 0.8), // Blue
            LoadType::LiveRoof => Color::from_rgb(0.3, 0.6, 0.9), // Light blue
            LoadType::Snow => Color::from_rgb(0.5, 0.7, 0.9), // Pale blue
            LoadType::Rain => Color::from_rgb(0.2, 0.6, 0.7), // Teal
            LoadType::Wind => Color::from_rgb(0.6, 0.3, 0.7), // Purple
            LoadType::Seismic => Color::from_rgb(0.8, 0.3, 0.3), // Red
            LoadType::SoilLateral => Color::from_rgb(0.7, 0.5, 0.2), // Orange-brown
            LoadType::Fluid => Color::from_rgb(0.3, 0.5, 0.7), // Steel blue
            LoadType::SelfStraining => Color::from_rgb(0.5, 0.5, 0.3), // Olive
        }
    }

    /// Draw individual discrete loads with stacking for overlap prevention
    fn draw_discrete_loads(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        width: f32,
        beam_y: f32,
        total_length: f64,
    ) {
        if self.data.discrete_loads.is_empty() {
            return;
        }

        let num_loads = self.data.discrete_loads.len();

        // Calculate available space for loads (from top margin to just above beam)
        // With beam at 70% and section_height ~100px, we have ~70px for loads
        let top_margin = 8.0_f32; // Space for top label
        let arrow_gap = 4.0_f32; // Gap between arrows and beam
        let available_height = beam_y - y - top_margin - arrow_gap;

        // Dynamic sizing based on number of loads
        // Target: 5 loads should fit comfortably with ~12px each
        let min_row_height = 10.0_f32; // Minimum to keep readable (compressed)
        let max_row_height = 18.0_f32; // Maximum comfortable spacing (1-2 loads)
        let load_row_height =
            (available_height / num_loads as f32).clamp(min_row_height, max_row_height);

        // Arrow length scales with row height (but has min/max bounds)
        let base_arrow_length = (load_row_height * 0.6).clamp(10.0, 18.0);
        let label_offset_y = 3.0_f32;

        // Stack loads from bottom (closest to beam) to top
        for (row, load) in self.data.discrete_loads.iter().enumerate() {
            let row_offset = row as f32 * load_row_height;
            let arrow_bottom_y = beam_y - arrow_gap - row_offset;
            let arrow_top_y = arrow_bottom_y - base_arrow_length;
            let load_color = Self::load_type_color(load.load_type);

            match &load.distribution {
                LoadDistribution::UniformFull => {
                    // Draw uniform load across full span
                    self.draw_uniform_load_region(
                        frame,
                        x,
                        x + width,
                        arrow_top_y,
                        arrow_bottom_y,
                        load_color,
                    );
                    // Label at center
                    let label = format!("L{} ({})", load.index, load.load_type.code());
                    let magnitude_label = format!("{:.0} plf", load.magnitude);
                    self.draw_load_label(
                        frame,
                        x + width / 2.0,
                        arrow_top_y - label_offset_y,
                        &label,
                        &magnitude_label,
                        load_color,
                    );
                }
                LoadDistribution::UniformPartial { start_ft, end_ft } => {
                    // Draw uniform load over partial span
                    let start_x = x + (*start_ft as f32 / total_length as f32) * width;
                    let end_x = x + (*end_ft as f32 / total_length as f32) * width;
                    self.draw_uniform_load_region(
                        frame,
                        start_x,
                        end_x,
                        arrow_top_y,
                        arrow_bottom_y,
                        load_color,
                    );
                    // Label at center of loaded region
                    let center_x = (start_x + end_x) / 2.0;
                    let label = format!("L{} ({})", load.index, load.load_type.code());
                    let magnitude_label = format!("{:.0} plf", load.magnitude);
                    self.draw_load_label(
                        frame,
                        center_x,
                        arrow_top_y - label_offset_y,
                        &label,
                        &magnitude_label,
                        load_color,
                    );
                }
                LoadDistribution::Point { position_ft } => {
                    // Draw point load at specific position
                    let point_x = x + (*position_ft as f32 / total_length as f32) * width;
                    self.draw_point_load(frame, point_x, arrow_top_y, arrow_bottom_y, load_color);
                    // Label above the arrow
                    let label = format!("L{} ({})", load.index, load.load_type.code());
                    let magnitude_label = format!("{:.0} lb", load.magnitude);
                    self.draw_load_label(
                        frame,
                        point_x,
                        arrow_top_y - label_offset_y,
                        &label,
                        &magnitude_label,
                        load_color,
                    );
                }
                LoadDistribution::Trapezoidal {
                    start_ft, end_ft, ..
                } => {
                    // Draw trapezoidal load (simplified as uniform for now)
                    let start_x = x + (*start_ft as f32 / total_length as f32) * width;
                    let end_x = x + (*end_ft as f32 / total_length as f32) * width;
                    self.draw_uniform_load_region(
                        frame,
                        start_x,
                        end_x,
                        arrow_top_y,
                        arrow_bottom_y,
                        load_color,
                    );
                    let center_x = (start_x + end_x) / 2.0;
                    let label = format!("L{} ({})", load.index, load.load_type.code());
                    let magnitude_label = "Trap.".to_string();
                    self.draw_load_label(
                        frame,
                        center_x,
                        arrow_top_y - label_offset_y,
                        &label,
                        &magnitude_label,
                        load_color,
                    );
                }
                LoadDistribution::Moment { position_ft } => {
                    // Draw moment as a curved arrow
                    let moment_x = x + (*position_ft as f32 / total_length as f32) * width;
                    self.draw_moment_load(frame, moment_x, arrow_top_y, arrow_bottom_y, load_color);
                    let label = format!("L{} ({})", load.index, load.load_type.code());
                    let magnitude_label = format!("{:.0} ft-lb", load.magnitude);
                    self.draw_load_label(
                        frame,
                        moment_x,
                        arrow_top_y - label_offset_y,
                        &label,
                        &magnitude_label,
                        load_color,
                    );
                }
            }
        }
    }

    /// Draw a uniform load region with multiple arrows
    fn draw_uniform_load_region(
        &self,
        frame: &mut Frame,
        start_x: f32,
        end_x: f32,
        top_y: f32,
        bottom_y: f32,
        color: Color,
    ) {
        let region_width = end_x - start_x;
        let num_arrows = (region_width / 15.0).clamp(3.0, 12.0) as i32;
        let arrow_spacing = region_width / (num_arrows as f32);

        // Draw top connecting line
        let top_line = Path::line(Point::new(start_x, top_y), Point::new(end_x, top_y));
        frame.stroke(
            &top_line,
            Stroke::default().with_color(color).with_width(1.5),
        );

        // Draw arrows
        for i in 0..=num_arrows {
            let ax = start_x + i as f32 * arrow_spacing;
            let arrow = Path::line(Point::new(ax, top_y), Point::new(ax, bottom_y));
            frame.stroke(&arrow, Stroke::default().with_color(color).with_width(1.0));

            // Arrow head
            let head = Path::new(|builder| {
                builder.move_to(Point::new(ax, bottom_y));
                builder.line_to(Point::new(ax - 2.5, bottom_y - 5.0));
                builder.move_to(Point::new(ax, bottom_y));
                builder.line_to(Point::new(ax + 2.5, bottom_y - 5.0));
            });
            frame.stroke(&head, Stroke::default().with_color(color).with_width(1.0));
        }
    }

    /// Draw a point load arrow
    fn draw_point_load(
        &self,
        frame: &mut Frame,
        x_pos: f32,
        top_y: f32,
        bottom_y: f32,
        color: Color,
    ) {
        // Main arrow line (thicker for point load)
        let arrow = Path::line(Point::new(x_pos, top_y), Point::new(x_pos, bottom_y));
        frame.stroke(&arrow, Stroke::default().with_color(color).with_width(2.5));

        // Arrow head (larger)
        let head = Path::new(|builder| {
            builder.move_to(Point::new(x_pos, bottom_y));
            builder.line_to(Point::new(x_pos - 4.0, bottom_y - 8.0));
            builder.line_to(Point::new(x_pos + 4.0, bottom_y - 8.0));
            builder.close();
        });
        frame.fill(&head, color);
    }

    /// Draw a moment load (curved arrow)
    fn draw_moment_load(
        &self,
        frame: &mut Frame,
        x_pos: f32,
        top_y: f32,
        bottom_y: f32,
        color: Color,
    ) {
        let center_y = (top_y + bottom_y) / 2.0;
        let radius = (bottom_y - top_y) / 2.0;

        // Draw a curved arrow (semi-circle with arrow head)
        let arc = Path::new(|builder| {
            // Approximate arc with line segments
            let segments = 12;
            for i in 0..=segments {
                let angle = std::f32::consts::PI * (i as f32 / segments as f32);
                let px = x_pos + radius * angle.cos();
                let py = center_y - radius * angle.sin();
                if i == 0 {
                    builder.move_to(Point::new(px, py));
                } else {
                    builder.line_to(Point::new(px, py));
                }
            }
        });
        frame.stroke(&arc, Stroke::default().with_color(color).with_width(2.0));

        // Arrow head at end of arc
        let head = Path::new(|builder| {
            builder.move_to(Point::new(x_pos - radius, center_y));
            builder.line_to(Point::new(x_pos - radius + 4.0, center_y - 4.0));
            builder.move_to(Point::new(x_pos - radius, center_y));
            builder.line_to(Point::new(x_pos - radius - 4.0, center_y - 4.0));
        });
        frame.stroke(&head, Stroke::default().with_color(color).with_width(2.0));
    }

    /// Draw load label (index and magnitude)
    fn draw_load_label(
        &self,
        frame: &mut Frame,
        x_pos: f32,
        y_pos: f32,
        label: &str,
        magnitude: &str,
        color: Color,
    ) {
        // Combined label: "L1 (D): 50 plf"
        let combined = format!("{}: {}", label, magnitude);
        let text = Text {
            content: combined,
            position: Point::new(x_pos, y_pos - 8.0),
            color,
            size: iced::Pixels(8.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            ..Text::default()
        };
        frame.fill_text(text);
    }

    #[allow(clippy::too_many_arguments)] // canvas drawing API — splitting into a
                                         // params struct would obscure call sites.
    fn draw_shear_diagram(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        axis_color: Color,
    ) {
        // Internal margins to keep content within bounds
        let top_margin = 18.0; // Space for title
        let bottom_margin = 12.0; // Space for labels
        let usable_height = height - top_margin - bottom_margin;

        let center_y = y + top_margin + usable_height / 2.0;
        let plot_height = usable_height * 0.4; // 40% above and below axis

        // Axis line
        let axis = Path::line(Point::new(x, center_y), Point::new(x + width, center_y));
        frame.stroke(
            &axis,
            Stroke::default().with_color(axis_color).with_width(1.0),
        );

        // Draw shear diagram using pre-computed points
        if !self.data.shear_diagram.is_empty() && self.data.max_shear_lb.abs() > 1e-6 {
            // Find min/max shear for scaling
            let max_v = self
                .data
                .shear_diagram
                .iter()
                .map(|(_, v)| v.abs())
                .fold(0.0f64, |a, b| a.max(b));

            if max_v > 1e-6 {
                // Draw filled area
                let shear_path = Path::new(|builder| {
                    let first = &self.data.shear_diagram[0];
                    let px = x + (first.0 as f32 / self.data.total_length_ft as f32) * width;
                    let v_norm = first.1 / max_v;
                    let py = center_y - (v_norm as f32) * plot_height;
                    builder.move_to(Point::new(px, center_y));
                    builder.line_to(Point::new(px, py));

                    for (pos, v) in &self.data.shear_diagram {
                        let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                        let v_norm = v / max_v;
                        let py = center_y - (v_norm as f32) * plot_height;
                        builder.line_to(Point::new(px, py));
                    }

                    if let Some(last) = self.data.shear_diagram.last() {
                        let px = x + (last.0 as f32 / self.data.total_length_ft as f32) * width;
                        builder.line_to(Point::new(px, center_y));
                    }
                    builder.close();
                });
                frame.fill(&shear_path, Color { a: 0.3, ..color });

                // Draw line
                let shear_line = Path::new(|builder| {
                    let first = &self.data.shear_diagram[0];
                    let px = x + (first.0 as f32 / self.data.total_length_ft as f32) * width;
                    let v_norm = first.1 / max_v;
                    let py = center_y - (v_norm as f32) * plot_height;
                    builder.move_to(Point::new(px, py));

                    for (pos, v) in &self.data.shear_diagram {
                        let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                        let v_norm = v / max_v;
                        let py = center_y - (v_norm as f32) * plot_height;
                        builder.line_to(Point::new(px, py));
                    }
                });
                frame.stroke(
                    &shear_line,
                    Stroke::default().with_color(color).with_width(2.0),
                );

                // Draw per-span max markers for multi-span beams
                if self.data.span_lengths_ft.len() > 1 {
                    let span_extrema = self.find_span_extrema(&self.data.shear_diagram);
                    for (i, (span_start, span_end, max_val, min_val)) in
                        span_extrema.iter().enumerate()
                    {
                        // Draw max shear marker
                        if let Some(max_pos) = self.find_extrema_position(
                            &self.data.shear_diagram,
                            *span_start,
                            *span_end,
                            *max_val,
                        ) {
                            let px =
                                x + (max_pos as f32 / self.data.total_length_ft as f32) * width;
                            let v_norm = max_val / max_v;
                            let py = center_y - (v_norm as f32) * plot_height;

                            // Small circle at max point
                            let marker = Path::circle(Point::new(px, py), 3.0);
                            frame.fill(&marker, color);

                            // Label (only show if significant and space permits)
                            if max_val.abs() > max_v * 0.1 {
                                let label = Text {
                                    content: format!("{:.0}", max_val),
                                    position: Point::new(px, py - 8.0),
                                    color,
                                    size: iced::Pixels(7.0),
                                    align_x: iced::alignment::Horizontal::Center.into(),
                                    ..Text::default()
                                };
                                frame.fill_text(label);
                            }
                        }

                        // Draw min shear marker (if different from max)
                        if (min_val - max_val).abs() > max_v * 0.1 {
                            if let Some(min_pos) = self.find_extrema_position(
                                &self.data.shear_diagram,
                                *span_start,
                                *span_end,
                                *min_val,
                            ) {
                                let px =
                                    x + (min_pos as f32 / self.data.total_length_ft as f32) * width;
                                let v_norm = min_val / max_v;
                                let py = center_y - (v_norm as f32) * plot_height;

                                let marker = Path::circle(Point::new(px, py), 3.0);
                                frame.fill(&marker, color);

                                if min_val.abs() > max_v * 0.1 {
                                    let label = Text {
                                        content: format!("{:.0}", min_val),
                                        position: Point::new(px, py + 10.0),
                                        color,
                                        size: iced::Pixels(7.0),
                                        align_x: iced::alignment::Horizontal::Center.into(),
                                        ..Text::default()
                                    };
                                    frame.fill_text(label);
                                }
                            }
                        }
                        let _ = i; // Suppress unused warning
                    }
                }
            }
        }

        // Draw support lines at interior supports (for multi-span)
        self.draw_support_lines(
            frame,
            x,
            width,
            y + top_margin,
            y + height - bottom_margin,
            axis_color,
        );

        // Labels - positioned within bounds
        let title = Text {
            content: "Shear (V)".to_string(),
            position: Point::new(x + 5.0, y + 3.0),
            color,
            size: iced::Pixels(10.0),
            ..Text::default()
        };
        frame.fill_text(title);

        let max_label = Text {
            content: format!("+{:.0} lb", self.data.max_shear_lb),
            position: Point::new(x + 55.0, y + 3.0), // Next to title
            color,
            size: iced::Pixels(9.0),
            ..Text::default()
        };
        frame.fill_text(max_label);

        let min_label = Text {
            content: format!("-{:.0} lb", self.data.max_shear_lb),
            position: Point::new(x + width - 55.0, y + 3.0), // Right side of title row
            color,
            size: iced::Pixels(9.0),
            ..Text::default()
        };
        frame.fill_text(min_label);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_moment_diagram(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        axis_color: Color,
    ) {
        // Internal margins to keep content within bounds
        let top_margin = 18.0; // Space for title
        let bottom_margin = 8.0; // Small bottom margin
        let usable_height = height - top_margin - bottom_margin;

        // Find min and max moments to properly scale and position axis
        let (min_m, max_m) = if !self.data.moment_diagram.is_empty() {
            self.data
                .moment_diagram
                .iter()
                .map(|(_, m)| *m)
                .fold((0.0f64, 0.0f64), |(min, max), m| (min.min(m), max.max(m)))
        } else {
            (0.0, self.data.max_moment_ftlb)
        };

        // Calculate axis position based on moment range
        // Positive moments go down, negative moments go up
        let total_range = max_m - min_m;
        let axis_ratio = if total_range.abs() > 1e-6 {
            // Position axis so that max positive is at bottom, max negative at top
            (-min_m / total_range) as f32 // Fraction of space above axis
        } else {
            0.15 // Default: axis near top if no range
        };

        // Clamp axis position to leave room for content
        let axis_ratio = axis_ratio.clamp(0.15, 0.85);
        let axis_y = y + top_margin + axis_ratio * usable_height;

        // Calculate scale factor to fit all moments within bounds
        let scale = if total_range.abs() > 1e-6 {
            usable_height / total_range as f32
        } else {
            1.0
        };

        // Axis line
        let axis = Path::line(Point::new(x, axis_y), Point::new(x + width, axis_y));
        frame.stroke(
            &axis,
            Stroke::default().with_color(axis_color).with_width(1.0),
        );

        // Draw moment diagram using pre-computed points
        if !self.data.moment_diagram.is_empty() && total_range.abs() > 1e-6 {
            // Draw filled area
            let moment_path = Path::new(|builder| {
                builder.move_to(Point::new(x, axis_y));
                for (pos, m) in &self.data.moment_diagram {
                    let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                    let py = axis_y + (*m as f32) * scale;
                    builder.line_to(Point::new(px, py));
                }
                builder.line_to(Point::new(x + width, axis_y));
                builder.close();
            });
            frame.fill(&moment_path, Color { a: 0.3, ..color });

            // Draw outline
            let outline = Path::new(|builder| {
                let first = &self.data.moment_diagram[0];
                let px = x + (first.0 as f32 / self.data.total_length_ft as f32) * width;
                let py = axis_y + (first.1 as f32) * scale;
                builder.move_to(Point::new(px, py));

                for (pos, m) in &self.data.moment_diagram {
                    let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                    let py = axis_y + (*m as f32) * scale;
                    builder.line_to(Point::new(px, py));
                }
            });
            frame.stroke(
                &outline,
                Stroke::default().with_color(color).with_width(2.0),
            );

            // Draw per-span max moment markers for multi-span beams
            if self.data.span_lengths_ft.len() > 1 {
                let span_extrema = self.find_span_extrema(&self.data.moment_diagram);
                for (i, (span_start, span_end, span_max, span_min)) in
                    span_extrema.iter().enumerate()
                {
                    // Draw max positive moment marker (if significant)
                    if *span_max > total_range * 0.05 {
                        if let Some(max_pos) = self.find_extrema_position(
                            &self.data.moment_diagram,
                            *span_start,
                            *span_end,
                            *span_max,
                        ) {
                            let px =
                                x + (max_pos as f32 / self.data.total_length_ft as f32) * width;
                            let py = axis_y + (*span_max as f32) * scale;

                            let marker = Path::circle(Point::new(px, py), 3.0);
                            frame.fill(&marker, color);

                            // Label for max moment
                            let label = Text {
                                content: format!("+{:.0}", span_max),
                                position: Point::new(px, py + 10.0),
                                color,
                                size: iced::Pixels(7.0),
                                align_x: iced::alignment::Horizontal::Center.into(),
                                ..Text::default()
                            };
                            frame.fill_text(label);
                        }
                    }

                    // Draw max negative moment marker at supports (if significant)
                    if span_min.abs() > total_range * 0.05 {
                        if let Some(min_pos) = self.find_extrema_position(
                            &self.data.moment_diagram,
                            *span_start,
                            *span_end,
                            *span_min,
                        ) {
                            let px =
                                x + (min_pos as f32 / self.data.total_length_ft as f32) * width;
                            let py = axis_y + (*span_min as f32) * scale;

                            let marker = Path::circle(Point::new(px, py), 3.0);
                            frame.fill(&marker, color);

                            let label = Text {
                                content: format!("{:.0}", span_min),
                                position: Point::new(px, py - 8.0),
                                color,
                                size: iced::Pixels(7.0),
                                align_x: iced::alignment::Horizontal::Center.into(),
                                ..Text::default()
                            };
                            frame.fill_text(label);
                        }
                    }
                    let _ = i; // Suppress unused warning
                }
            }
        }

        // Draw support lines at interior supports (for multi-span)
        self.draw_support_lines(
            frame,
            x,
            width,
            y + top_margin,
            y + height - bottom_margin,
            axis_color,
        );

        // Labels - positioned within bounds
        let title = Text {
            content: "Moment (M)".to_string(),
            position: Point::new(x + 5.0, y + 3.0),
            color,
            size: iced::Pixels(10.0),
            ..Text::default()
        };
        frame.fill_text(title);

        // Show both max positive and max negative if applicable
        let label_text = if min_m < -1.0 {
            format!("+{:.0} / {:.0} ft-lb", max_m, min_m)
        } else {
            format!("Max: {:.0} ft-lb", max_m)
        };
        let max_label = Text {
            content: label_text,
            position: Point::new(x + 75.0, y + 3.0),
            color,
            size: iced::Pixels(9.0),
            ..Text::default()
        };
        frame.fill_text(max_label);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_deflection_diagram(
        &self,
        frame: &mut Frame,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        axis_color: Color,
    ) {
        // Internal margins to keep content within bounds
        let top_margin = 18.0; // Space for title/axis
        let bottom_margin = 8.0; // Small bottom margin
        let usable_height = height - top_margin - bottom_margin;

        let axis_y = y + top_margin;
        let plot_height = usable_height * 0.85; // Deflection goes downward from axis

        // Axis line (represents undeflected beam)
        let axis = Path::line(Point::new(x, axis_y), Point::new(x + width, axis_y));
        frame.stroke(
            &axis,
            Stroke::default().with_color(axis_color).with_width(1.0),
        );

        // Draw deflection using pre-computed points
        // Use absolute max for scale so positive=down, negative=up renders correctly
        if !self.data.deflection_diagram.is_empty() && self.data.max_deflection_in.abs() > 1e-9 {
            let scale = self.data.max_deflection_in.abs();

            // Draw curve (positive deflection = downward = below axis)
            let defl_path = Path::new(|builder| {
                let first = &self.data.deflection_diagram[0];
                let px = x + (first.0 as f32 / self.data.total_length_ft as f32) * width;
                let d_ratio = first.1 / scale;
                let py = axis_y + (d_ratio as f32) * plot_height;
                builder.move_to(Point::new(px, py));

                for (pos, d) in &self.data.deflection_diagram {
                    let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                    let d_ratio = d / scale;
                    let py = axis_y + (d_ratio as f32) * plot_height;
                    builder.line_to(Point::new(px, py));
                }
            });
            frame.stroke(
                &defl_path,
                Stroke::default().with_color(color).with_width(2.0),
            );

            // Fill under curve
            let fill_path = Path::new(|builder| {
                builder.move_to(Point::new(x, axis_y));
                for (pos, d) in &self.data.deflection_diagram {
                    let px = x + (*pos as f32 / self.data.total_length_ft as f32) * width;
                    let d_ratio = d / scale;
                    let py = axis_y + (d_ratio as f32) * plot_height;
                    builder.line_to(Point::new(px, py));
                }
                builder.line_to(Point::new(x + width, axis_y));
                builder.close();
            });
            frame.fill(&fill_path, Color { a: 0.2, ..color });

            // Draw per-span max deflection markers for multi-span beams
            if self.data.span_lengths_ft.len() > 1 {
                let span_extrema = self.find_span_extrema(&self.data.deflection_diagram);
                for (i, (span_start, span_end, span_max, span_min)) in
                    span_extrema.iter().enumerate()
                {
                    // Find the extremum with largest absolute value in this span
                    let (extremum, is_positive) = if span_max.abs() > span_min.abs() {
                        (*span_max, true)
                    } else {
                        (*span_min, false)
                    };

                    // Only show if significant
                    if extremum.abs() > scale * 0.05 {
                        if let Some(ext_pos) = self.find_extrema_position(
                            &self.data.deflection_diagram,
                            *span_start,
                            *span_end,
                            extremum,
                        ) {
                            let px =
                                x + (ext_pos as f32 / self.data.total_length_ft as f32) * width;
                            let d_ratio = extremum / scale;
                            let py = axis_y + (d_ratio as f32) * plot_height;

                            let marker = Path::circle(Point::new(px, py), 3.0);
                            frame.fill(&marker, color);

                            // Label position: below for positive (downward), above for negative (upward)
                            let label_y = if is_positive { py + 10.0 } else { py - 8.0 };
                            let label = Text {
                                content: format!("{:.3}\"", extremum),
                                position: Point::new(px, label_y),
                                color,
                                size: iced::Pixels(7.0),
                                align_x: iced::alignment::Horizontal::Center.into(),
                                ..Text::default()
                            };
                            frame.fill_text(label);
                        }
                    }
                    let _ = i; // Suppress unused warning
                }
            }
        }

        // Draw support lines at interior supports (for multi-span)
        self.draw_support_lines(
            frame,
            x,
            width,
            y + top_margin,
            y + height - bottom_margin,
            axis_color,
        );

        // Labels - positioned within bounds
        let title = Text {
            content: "Deflection (δ)".to_string(),
            position: Point::new(x + 5.0, y + 3.0),
            color,
            size: iced::Pixels(10.0),
            ..Text::default()
        };
        frame.fill_text(title);

        let max_label = Text {
            content: format!("Max: {:.3} in", self.data.max_deflection_in.abs()),
            position: Point::new(x + 85.0, y + 3.0), // Next to title
            color,
            size: iced::Pixels(9.0),
            ..Text::default()
        };
        frame.fill_text(max_label);
    }
}

impl canvas::Program<Message> for BeamDiagram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let width = bounds.width;
        let height = bounds.height;

        // Layout: divide into 4 sections with padding between them
        let section_padding = 12.0; // Padding between sections to prevent label overlap
        let total_padding = section_padding * 3.0; // 3 gaps between 4 sections
        let section_height = (height - total_padding) / 4.0;
        let margin = 20.0;
        let plot_width = width - 2.0 * margin;

        // Colors
        let beam_color = Color::from_rgb(0.3, 0.3, 0.3);
        let shear_color = Color::from_rgb(0.2, 0.5, 0.8);
        let moment_color = Color::from_rgb(0.8, 0.4, 0.2);
        let defl_color = Color::from_rgb(0.2, 0.7, 0.3);
        let axis_color = Color::from_rgb(0.5, 0.5, 0.5);

        // Calculate section Y positions with padding
        let section1_y = 0.0;
        let section2_y = section_height + section_padding;
        let section3_y = (section_height + section_padding) * 2.0;
        let section4_y = (section_height + section_padding) * 3.0;

        // Section 1: Beam schematic with loads
        self.draw_beam_schematic(
            &mut frame,
            margin,
            section1_y,
            plot_width,
            section_height,
            beam_color,
        );

        // Section 2: Shear diagram
        self.draw_shear_diagram(
            &mut frame,
            margin,
            section2_y,
            plot_width,
            section_height,
            shear_color,
            axis_color,
        );

        // Section 3: Moment diagram
        self.draw_moment_diagram(
            &mut frame,
            margin,
            section3_y,
            plot_width,
            section_height,
            moment_color,
            axis_color,
        );

        // Section 4: Deflection diagram
        self.draw_deflection_diagram(
            &mut frame,
            margin,
            section4_y,
            plot_width,
            section_height,
            defl_color,
            axis_color,
        );

        vec![frame.into_geometry()]
    }
}
