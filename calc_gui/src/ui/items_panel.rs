//! Items Panel (Left Sidebar)
//!
//! Displays project navigation tree with:
//! - Project Info section (clickable to edit)
//! - Enabled category sections (added via category picker)

use std::collections::HashSet;

use iced::widget::{button, column, container, row, rule, scrollable, text, Column, Space};
use iced::{Alignment, Element, Length, Padding};
use uuid::Uuid;

use calc_core::calculations::CalculationItem;
use calc_core::project::Project;

use crate::{EditorSelection, ItemCategory, ItemSection, Message};

/// Render the items panel (left sidebar)
pub fn view_items_panel<'a>(
    project: &'a Project,
    collapsed_sections: &'a HashSet<ItemSection>,
    enabled_categories: &'a HashSet<ItemCategory>,
    selection: &'a EditorSelection,
    selected_beam_id: Option<Uuid>,
    width: f32,
) -> Element<'a, Message> {
    let mut panel_content: Column<'_, Message> = column![].spacing(2);

    // ===== Project Items Header with + button =====
    let header = row![
        text("Project Items").size(12),
        Space::new().width(Length::Fill),
        button(text("+").size(12))
            .on_press(Message::OpenCategoryPicker)
            .padding(Padding::from([2, 8]))
            .style(button::secondary),
    ]
    .align_y(Alignment::Center)
    .padding(Padding::from([4, 4]));

    panel_content = panel_content.push(header);
    panel_content = panel_content.push(rule::horizontal(1));

    // ===== Project Info Section =====
    let project_expanded = !collapsed_sections.contains(&ItemSection::ProjectInfo);
    let project_selected = matches!(selection, EditorSelection::ProjectInfo);

    // Project Info section header (expand/collapse)
    let project_indicator = if project_expanded { "▼" } else { "▶" };
    let project_header = button(
        row![
            text(project_indicator).size(10),
            Space::new().width(4),
            text("Project Info").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(Message::ToggleSection(ItemSection::ProjectInfo))
    .padding(Padding::from([4, 6]))
    .style(button::text)
    .width(Length::Fill);
    panel_content = panel_content.push(project_header);

    if project_expanded {
        let project_info_content = column![
            text(format!("Eng: {}", project.meta.engineer)).size(10),
            text(format!("Job: {}", project.meta.job_id)).size(10),
            text(format!("Client: {}", project.meta.client)).size(10),
        ]
        .spacing(2);

        // Make clickable to select project info for editing
        let project_btn_style = if project_selected {
            button::primary
        } else {
            button::secondary
        };
        let project_info_btn = button(project_info_content)
            .on_press(Message::SelectProjectInfo)
            .padding(Padding::from([4, 16]))
            .style(project_btn_style)
            .width(Length::Fill);

        panel_content = panel_content.push(project_info_btn);
    }

    panel_content = panel_content.push(rule::horizontal(1));

    // ===== Wood Beams Section (if enabled) =====
    if enabled_categories.contains(&ItemCategory::WoodBeams) {
        let beams_expanded = !collapsed_sections.contains(&ItemSection::WoodBeams);
        let beam_count = project
            .items
            .values()
            .filter(|i| matches!(i, CalculationItem::Beam(_)))
            .count();

        // Section header with expand/collapse and add button
        let beams_indicator = if beams_expanded { "▼" } else { "▶" };
        let beams_header_btn = button(
            row![
                text(beams_indicator).size(10),
                Space::new().width(4),
                text(format!("Wood Beams ({})", beam_count)).size(11),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleSection(ItemSection::WoodBeams))
        .padding(Padding::from([4, 6]))
        .style(button::text)
        .width(Length::Fill);

        let beams_header = row![
            beams_header_btn,
            button(text("+").size(11))
                .on_press(Message::CreateBeam)
                .padding(Padding::from([2, 6]))
                .style(button::secondary),
        ]
        .spacing(2);
        panel_content = panel_content.push(beams_header);

        if beams_expanded {
            let mut beams_list: Column<'_, Message> =
                column![].spacing(2).padding(Padding::from([4, 8]));

            // List beams
            for (id, item) in &project.items {
                if let CalculationItem::Beam(beam) = item {
                    let is_selected = selected_beam_id == Some(*id);
                    let style = if is_selected {
                        button::primary
                    } else {
                        button::secondary
                    };
                    let btn = button(text(&beam.label).size(10))
                        .on_press(Message::SelectBeam(*id))
                        .padding(Padding::from([3, 6]))
                        .style(style)
                        .width(Length::Fill);
                    beams_list = beams_list.push(btn);
                }
            }

            if beam_count == 0 {
                beams_list = beams_list.push(text("(none)").size(10).color([0.5, 0.5, 0.5]));
            }

            panel_content = panel_content.push(beams_list);
        }

        panel_content = panel_content.push(rule::horizontal(1));
    }

    // Hint when no categories are enabled
    if enabled_categories.is_empty() {
        let hint = column![
            Space::new().height(20),
            text("Click + to add").size(10).color([0.5, 0.5, 0.5]),
            text("item categories").size(10).color([0.5, 0.5, 0.5]),
        ]
        .align_x(Alignment::Center)
        .padding(Padding::from([8, 8]));
        panel_content = panel_content.push(hint);
    }

    let panel = container(scrollable(panel_content.padding(4)))
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .style(container::bordered_box)
        .padding(4);

    panel.into()
}
