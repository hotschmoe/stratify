//! Results view for Wood Beam calculations
//!
//! Displays results organized into two tabs:
//! - Results: Pass/Fail, demand values, capacity checks, reactions
//! - Diagrams: Beam schematic, shear, moment, deflection diagrams

use iced::widget::{button, column, rule, text, Canvas, Column, Row, Space};
use iced::{Element, Length, Padding};

use calc_core::calculations::continuous_beam::{ContinuousBeamInput, ContinuousBeamResult};
use calc_core::nds_factors::nds_ref;

use super::shared::diagrams::{BeamDiagram, BeamDiagramData};
use crate::{Message, ResultsTab};

/// Render the beam calculation results with tabbed interface
pub fn view<'a>(
    input: &'a ContinuousBeamInput,
    result: &'a ContinuousBeamResult,
    selected_tab: ResultsTab,
) -> Column<'a, Message> {
    // Tab bar
    let tab_bar = view_tab_bar(selected_tab);

    // Tab content based on selection
    let tab_content: Element<'_, Message> = match selected_tab {
        ResultsTab::Results => view_results_tab(input, result),
        ResultsTab::Diagrams => view_diagrams_tab(input, result),
    };

    column![
        tab_bar,
        rule::horizontal(1),
        Space::new().height(8),
        tab_content,
    ]
}

/// Render the tab bar
fn view_tab_bar(selected: ResultsTab) -> Element<'static, Message> {
    let mut tabs = Row::new().spacing(4);

    for tab in ResultsTab::ALL {
        let is_selected = tab == selected;
        let label = text(tab.to_string()).size(11);

        let btn = if is_selected {
            // Selected tab style - more prominent
            button(label)
                .padding(Padding::from([6, 16]))
                .style(button::secondary)
        } else {
            // Unselected tab style
            button(label)
                .on_press(Message::SelectResultsTab(tab))
                .padding(Padding::from([6, 16]))
                .style(button::text)
        };

        tabs = tabs.push(btn);
    }

    tabs.into()
}

/// Results tab: numerical calculation results
fn view_results_tab<'a>(
    input: &'a ContinuousBeamInput,
    result: &'a ContinuousBeamResult,
) -> Element<'a, Message> {
    view_calculation_results(input, result).into()
}

/// Diagrams tab: visual diagrams
fn view_diagrams_tab<'a>(
    input: &'a ContinuousBeamInput,
    result: &'a ContinuousBeamResult,
) -> Element<'a, Message> {
    let diagram_data = BeamDiagramData::from_calc(input, result);
    let diagram = BeamDiagram::new(diagram_data);

    let canvas_widget: Element<'_, Message> = Canvas::new(diagram)
        .width(Length::Fill)
        .height(Length::Fixed(400.0))
        .into();

    column![
        text("Beam Diagrams").size(14),
        Space::new().height(8),
        canvas_widget,
    ]
    .into()
}

/// Render calculation results text
fn view_calculation_results<'a>(
    input: &'a ContinuousBeamInput,
    result: &'a ContinuousBeamResult,
) -> Column<'a, Message> {
    let pass_fail = if result.passes() {
        text("DESIGN ADEQUATE").size(16).color([0.2, 0.6, 0.2])
    } else {
        text("DESIGN INADEQUATE").size(16).color([0.8, 0.2, 0.2])
    };

    let governing = text(format!("Governing: {}", result.governing_condition)).size(11);

    // Get first span result for detailed stress checks (single-span case)
    let span_result = result.span_results.first();

    let (bending_status, bending_unity, actual_fb, allowable_fb) = match span_result {
        Some(sr) => {
            let status = if sr.bending_unity <= 1.0 {
                "OK"
            } else {
                "FAIL"
            };
            (
                status,
                sr.bending_unity,
                sr.actual_fb_psi,
                sr.allowable_fb_psi,
            )
        }
        None => ("N/A", 0.0, 0.0, 0.0),
    };

    let (shear_status, shear_unity, actual_fv, allowable_fv) = match span_result {
        Some(sr) => {
            let status = if sr.shear_unity <= 1.0 { "OK" } else { "FAIL" };
            (
                status,
                sr.shear_unity,
                sr.actual_fv_psi,
                sr.allowable_fv_psi,
            )
        }
        None => ("N/A", 0.0, 0.0, 0.0),
    };

    let (defl_status, defl_unity) = match span_result {
        Some(sr) => {
            let status = if sr.deflection_unity <= 1.0 {
                "OK"
            } else {
                "FAIL"
            };
            (status, sr.deflection_unity)
        }
        None => ("N/A", 0.0),
    };

    // Build reactions display string (R_1, R_2, R_3, etc.)
    let reactions_str = result
        .reactions
        .iter()
        .enumerate()
        .map(|(i, r)| format!("R_{} = {:.0} lb", i + 1, r))
        .collect::<Vec<_>>()
        .join(", ");

    // Get section properties from input
    let (section_modulus, moment_inertia) = input
        .spans
        .first()
        .map(|span| (span.section_modulus_in3(), span.moment_of_inertia_in4()))
        .unwrap_or((0.0, 0.0));

    column![
        text("Calculation Results").size(14),
        Space::new().height(8),
        pass_fail,
        governing,
        Space::new().height(12),
        text("Load Summary").size(12),
        text(format!("Governing Combo: {}", result.governing_combination)).size(11),
        Space::new().height(12),
        text("Demand").size(12),
        text(format!(
            "Max Moment: {:.0} ft-lb",
            result.max_positive_moment_ftlb
        ))
        .size(11),
        text(format!("Max Shear: {:.0} lb", result.max_shear_lb)).size(11),
        text(format!(
            "Max Deflection: {:.3} in",
            result.max_deflection_in
        ))
        .size(11),
        Space::new().height(12),
        text("Capacity Checks").size(12),
        text(format!(
            "Bending: {:.0}/{:.0} psi = {:.2} [{}] ({})",
            actual_fb,
            allowable_fb,
            bending_unity,
            bending_status,
            nds_ref::BENDING
        ))
        .size(11),
        text(format!(
            "Shear: {:.0}/{:.0} psi = {:.2} [{}] ({})",
            actual_fv,
            allowable_fv,
            shear_unity,
            shear_status,
            nds_ref::SHEAR
        ))
        .size(11),
        text(format!(
            "Deflection: {:.2} [{}] ({})",
            defl_unity,
            defl_status,
            nds_ref::DEFLECTION
        ))
        .size(11),
        Space::new().height(12),
        text("Section Properties").size(12),
        text(format!("Section Modulus (S): {:.2} in³", section_modulus)).size(11),
        text(format!("Moment of Inertia (I): {:.2} in⁴", moment_inertia)).size(11),
        Space::new().height(12),
        text("Support Reactions").size(12),
        text(format!("Max: {}", reactions_str)).size(11),
        text(format!("  ({})", result.governing_combination)).size(10),
        view_min_reactions(result),
    ]
}

/// Render minimum reactions section with uplift warning
fn view_min_reactions<'a>(result: &'a ContinuousBeamResult) -> Element<'a, Message> {
    // Build min reactions display string
    let min_reactions_str = result
        .min_reactions
        .iter()
        .enumerate()
        .map(|(i, r)| format!("R_{} = {:.0} lb", i + 1, r))
        .collect::<Vec<_>>()
        .join(", ");

    // Check for uplift at any reaction
    let has_uplift = result.min_reactions.iter().any(|&r| r < 0.0);

    let min_reactions_text = text(format!("Min: {}", min_reactions_str)).size(11);
    let combo_text = text(format!("  ({})", result.min_reaction_combination)).size(10);

    if has_uplift {
        let uplift_warning = text("UPLIFT - Hold-downs required!")
            .size(11)
            .color([0.9, 0.5, 0.0]);

        // Find the worst uplift value and its location. `total_cmp` is a total order
        // over f64 (NaN-safe); reactions come from numeric calculation and shouldn't
        // be NaN, but a total order avoids the panic if one ever sneaks through.
        let (worst_idx, worst_value) = result
            .min_reactions
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, v)| (i + 1, *v))
            .unwrap_or((1, 0.0));

        let uplift_detail = text(format!(
            "Max uplift at R_{}: {:.0} lb",
            worst_idx,
            worst_value.abs()
        ))
        .size(10)
        .color([0.9, 0.5, 0.0]);

        column![
            min_reactions_text,
            combo_text,
            Space::new().height(4),
            uplift_warning,
            uplift_detail,
        ]
        .spacing(2)
        .into()
    } else {
        column![min_reactions_text, combo_text].spacing(2).into()
    }
}
