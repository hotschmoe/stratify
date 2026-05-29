//! Results Panel (Right Side)
//!
//! Dispatches to the appropriate results view based on EditorSelection:
//! - ProjectInfo -> result_project_info (project summary)
//! - Beam with results -> result_wood_beam (calculation results + diagrams)
//! - Error -> error display

use iced::widget::{column, container, scrollable, text, Column, Space};
use iced::{Element, Length};

use super::{result_project_info, result_wood_beam};
use crate::{App, Message};

/// Render the results panel based on current selection and calculation state
///
/// The `input_ratio` parameter is the ratio used by the input panel.
/// This panel uses the complementary ratio (1 - input_ratio).
pub fn view_results_panel(app: &App, input_ratio: f32) -> Element<'_, Message> {
    let content: Column<'_, Message> = if let Some(ref error) = app.error_message {
        // Show error message
        column![
            text("Error").size(14),
            Space::new().height(8),
            text(error).size(12).color([0.8, 0.2, 0.2]),
        ]
    } else if let (Some(ref input), Some(ref result)) = (&app.calc_input, &app.result) {
        // Show beam calculation results with tabs
        result_wood_beam::view(input, result, app.selected_results_tab)
    } else {
        // Show project summary
        result_project_info::view(&app.project)
    };

    // Use complementary ratio (scale to 0-100 for better precision)
    let portion = ((1.0 - input_ratio) * 100.0) as u16;

    let panel = container(scrollable(content.padding(8)))
        .width(Length::FillPortion(portion))
        .style(container::bordered_box)
        .padding(5);

    panel.into()
}
