//! Input view for Project Information
//!
//! Displays fields for Engineer, Job ID, and Client.

use iced::widget::{column, row, text, text_input, Column, Space};
use iced::{Alignment, Element, Length};

use calc_core::project::ProjectMetadata;

use crate::Message;

/// Render the project info editor
pub fn view(meta: &ProjectMetadata) -> Column<'_, Message> {
    column![
        text("Project Information").size(14),
        Space::new().height(8),
        labeled_input("Engineer:", &meta.engineer, Message::EngineerNameChanged),
        labeled_input("Job ID:", &meta.job_id, Message::JobIdChanged),
        labeled_input("Client:", &meta.client, Message::ClientChanged),
        Space::new().height(20),
        text("Select a beam from the left panel to edit,")
            .size(11)
            .color([0.5, 0.5, 0.5]),
        text("or click '+' to create a new beam.")
            .size(11)
            .color([0.5, 0.5, 0.5]),
    ]
    .spacing(6)
}

/// Helper to create a labeled text input
fn labeled_input<'a>(
    label: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(11).width(Length::Fixed(80.0)),
        text_input("", value)
            .on_input(on_change)
            .width(Length::Fill)
            .padding(4)
            .size(11),
    ]
    .align_y(Alignment::Center)
    .into()
}
