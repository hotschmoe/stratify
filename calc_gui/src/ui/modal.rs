//! Modal dialog component
//!
//! Provides a reusable modal overlay system for confirmation dialogs,
//! alerts, and other popup interactions.

use iced::widget::{button, column, container, row, scrollable, text, Column, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::{ItemCategory, Message};

/// Types of modal dialogs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalType {
    /// Prompt to save unsaved changes before an action
    UnsavedChanges {
        /// The action that triggered this modal (for display)
        action: PendingAction,
    },
    /// Category picker for adding new item categories to the project
    CategoryPicker,
}

/// Actions that can be pending while a modal is shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    /// User wants to create a new project
    NewProject,
    /// User wants to open an existing project
    OpenProject,
}

impl std::fmt::Display for PendingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PendingAction::NewProject => write!(f, "create a new project"),
            PendingAction::OpenProject => write!(f, "open another project"),
        }
    }
}

/// Render a modal backdrop (semi-transparent overlay that catches clicks)
pub fn view_backdrop() -> Element<'static, Message> {
    button(Space::new())
        .on_press(Message::ModalCancel)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_, _| {
            iced::widget::button::Style::default()
                .with_background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
        })
        .into()
}

/// Render a modal dialog based on its type
pub fn view_modal(modal_type: &ModalType) -> Element<'_, Message> {
    match modal_type {
        ModalType::UnsavedChanges { action } => view_unsaved_changes_modal(*action),
        ModalType::CategoryPicker => view_category_picker_modal(),
    }
}

/// Render the "Save current progress?" modal
fn view_unsaved_changes_modal(action: PendingAction) -> Element<'static, Message> {
    let title = text("Save Changes?").size(18);

    let description = text(format!(
        "You have unsaved changes. Would you like to save before you {}?",
        action
    ))
    .size(12);

    let buttons = row![
        button(text("Don't Save").size(11))
            .on_press(Message::ModalDontSave)
            .padding(Padding::from([6, 16]))
            .style(button::secondary),
        Space::new().width(8),
        button(text("Cancel").size(11))
            .on_press(Message::ModalCancel)
            .padding(Padding::from([6, 16]))
            .style(button::secondary),
        Space::new().width(8),
        button(text("Save").size(11))
            .on_press(Message::ModalSave)
            .padding(Padding::from([6, 16]))
            .style(button::primary),
    ]
    .align_y(Alignment::Center);

    let content = column![
        title,
        Space::new().height(12),
        description,
        Space::new().height(20),
        buttons,
    ]
    .width(Length::Fixed(400.0));

    let modal_box = container(content)
        .padding(20)
        .style(container::bordered_box);

    // Center the modal in the screen
    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

/// Category groups for tab organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryGroup {
    Beams,
    Posts,
    Foundations,
    Retaining,
    Misc,
}

impl CategoryGroup {
    pub const ALL: &'static [CategoryGroup] = &[
        CategoryGroup::Beams,
        CategoryGroup::Posts,
        CategoryGroup::Foundations,
        CategoryGroup::Retaining,
        CategoryGroup::Misc,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            CategoryGroup::Beams => "Beams",
            CategoryGroup::Posts => "Posts",
            CategoryGroup::Foundations => "Foundations",
            CategoryGroup::Retaining => "Retaining",
            CategoryGroup::Misc => "Misc",
        }
    }

    /// Get categories belonging to this group
    pub fn categories(&self) -> Vec<(ItemCategory, bool)> {
        // Returns (category, is_implemented) pairs
        match self {
            CategoryGroup::Beams => vec![
                (ItemCategory::WoodBeams, true),
                // (ItemCategory::SteelBeams, false),      // Future
                // (ItemCategory::ContinuousBeams, false), // Future
            ],
            CategoryGroup::Posts => vec![
                // (ItemCategory::WoodColumns, false),     // Future
                // (ItemCategory::SteelColumns, false),    // Future
            ],
            CategoryGroup::Foundations => vec![
                // (ItemCategory::SpreadFootings, false),  // Future
                // (ItemCategory::CombinedFootings, false),// Future
            ],
            CategoryGroup::Retaining => vec![
                (ItemCategory::CantileverRetainingWalls, false),
                // (ItemCategory::GravityWalls, false),    // Future
            ],
            CategoryGroup::Misc => vec![
                // (ItemCategory::Connections, false),     // Future
            ],
        }
    }

    /// Check if group has any categories (for future filtering)
    #[allow(dead_code)]
    pub fn has_categories(&self) -> bool {
        !self.categories().is_empty()
    }
}

/// Render the category picker modal with tabbed sections
fn view_category_picker_modal() -> Element<'static, Message> {
    let title = text("Add Item Category").size(18);

    let description = text("Select a category to add to your project:").size(12);

    // Build tab-style sections with category lists
    let mut sections: Column<'_, Message> = column![].spacing(12);

    for group in CategoryGroup::ALL {
        let categories = group.categories();
        if categories.is_empty() {
            continue;
        }

        // Section header (tab-like styling)
        let section_header = container(text(group.name()).size(11))
            .padding(Padding::from([4, 8]))
            .style(|_theme: &iced::Theme| container::Style {
                text_color: Some(iced::Color::from_rgb(0.4, 0.4, 0.4)),
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.95, 0.95, 0.95,
                ))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.85, 0.85, 0.85),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: false,
            })
            .width(Length::Fill);

        sections = sections.push(section_header);

        // Category items within section
        let mut category_buttons: Column<'_, Message> =
            column![].spacing(4).padding(Padding::from([4, 0]));

        for (category, is_implemented) in categories {
            let label = if is_implemented {
                text(category.display_name()).size(12)
            } else {
                text(format!("{} (coming soon)", category.display_name()))
                    .size(12)
                    .color([0.6, 0.6, 0.6])
            };

            let btn = if is_implemented {
                button(row![label].align_y(Alignment::Center).width(Length::Fill))
                    .on_press(Message::AddCategory(category))
                    .padding(Padding::from([8, 12]))
                    .style(button::secondary)
                    .width(Length::Fill)
            } else {
                button(row![label].align_y(Alignment::Center).width(Length::Fill))
                    .padding(Padding::from([8, 12]))
                    .style(button::secondary)
                    .width(Length::Fill)
                // No on_press for unimplemented categories
            };

            category_buttons = category_buttons.push(btn);
        }

        sections = sections.push(category_buttons);
    }

    let cancel_btn = button(text("Cancel").size(11))
        .on_press(Message::CloseCategoryPicker)
        .padding(Padding::from([6, 16]))
        .style(button::secondary);

    let content = column![
        title,
        Space::new().height(12),
        description,
        Space::new().height(16),
        scrollable(sections).height(Length::Fixed(250.0)),
        Space::new().height(16),
        container(cancel_btn)
            .align_x(iced::alignment::Horizontal::Right)
            .width(Length::Fill),
    ]
    .width(Length::Fixed(350.0));

    let modal_box = container(content)
        .padding(20)
        .style(container::bordered_box);

    // Center the modal in the screen
    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
