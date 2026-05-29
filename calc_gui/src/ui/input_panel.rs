//! Input Panel (Center)
//!
//! Dispatches to the appropriate input editor based on EditorSelection:
//! - ProjectInfo -> input_project_info
//! - Beam -> input_wood_beam
//! - None -> welcome message

use iced::widget::{container, scrollable, text, Column};
use iced::{Element, Length};

use super::{input_project_info, input_wood_beam};
use crate::{App, EditorSelection, Message};

/// Render the input panel based on current selection
///
/// The `ratio` parameter determines the relative size of this panel vs results panel.
/// A ratio of 0.5 means equal sizes, 0.7 means input takes 70% of the space.
pub fn view_input_panel(app: &App, ratio: f32) -> Element<'_, Message> {
    let panel: Column<'_, Message> = match app.selection {
        EditorSelection::ProjectInfo => input_project_info::view(&app.project.meta),
        EditorSelection::None => Column::new().push(
            text("Select an item from the left panel")
                .size(14)
                .color([0.5, 0.5, 0.5]),
        ),
        EditorSelection::Beam(_) => input_wood_beam::view(app),
    };

    // Convert ratio to fill portion (scale to 0-100 for better precision)
    let portion = (ratio * 100.0) as u16;

    container(scrollable(panel.padding(8)))
        .width(Length::FillPortion(portion))
        .style(container::bordered_box)
        .padding(5)
        .into()
}
