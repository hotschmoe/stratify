//! Toolbar component
//!
//! Contains file operations (New, Open, Save, Save As, Export PDF) and settings dropdown.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::Message;
#[cfg(not(target_arch = "wasm32"))]
use crate::UpdateStatus;

/// Render the application header with title (borrowed)
#[allow(dead_code)]
pub fn view_header(window_title: &str) -> Element<'_, Message> {
    row![
        text("Stratify").size(28),
        Space::new().width(Length::Fill),
        text(window_title).size(14),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Render the application header with title (owned)
pub fn view_header_owned(window_title: String) -> Element<'static, Message> {
    row![
        text("Stratify").size(28),
        Space::new().width(Length::Fill),
        text(window_title).size(14),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Render the toolbar with file operations and settings dropdown
pub fn view_toolbar(settings_menu_open: bool) -> Element<'static, Message> {
    let file_buttons = row![
        button(text("New").size(11))
            .on_press(Message::NewProject)
            .padding(Padding::from([4, 8]))
            .style(button::secondary),
        button(text("Open").size(11))
            .on_press(Message::OpenProject)
            .padding(Padding::from([4, 8]))
            .style(button::secondary),
        button(text("Save").size(11))
            .on_press(Message::SaveProject)
            .padding(Padding::from([4, 8]))
            .style(button::secondary),
        button(text("Save As").size(11))
            .on_press(Message::SaveProjectAs)
            .padding(Padding::from([4, 8]))
            .style(button::secondary),
        button(text("Export PDF").size(11))
            .on_press(Message::ExportPdf)
            .padding(Padding::from([4, 8]))
            .style(button::primary),
    ]
    .spacing(4);

    // Settings button with dropdown indicator
    let settings_button_text = if settings_menu_open {
        "Settings ▲"
    } else {
        "Settings ▼"
    };
    let settings_button = button(text(settings_button_text).size(11))
        .on_press(Message::ToggleSettingsMenu)
        .padding(Padding::from([4, 8]))
        .style(if settings_menu_open {
            button::primary
        } else {
            button::secondary
        });

    row![
        file_buttons,
        Space::new().width(Length::Fill),
        settings_button,
    ]
    .padding(Padding::from([4, 0]))
    .align_y(Alignment::Center)
    .into()
}

/// Render the settings dropdown menu (native version with update checking)
#[cfg(not(target_arch = "wasm32"))]
pub fn view_settings_menu(
    dark_mode: bool,
    update_status: &UpdateStatus,
) -> Element<'static, Message> {
    let theme_label = if dark_mode { "Light Mode" } else { "Dark Mode" };

    // Build update button based on status
    let update_button = match update_status {
        UpdateStatus::NotChecked | UpdateStatus::Failed(_) => {
            button(text("Check for Updates").size(10))
                .on_press(Message::CheckForUpdates)
                .padding(Padding::from([4, 12]))
                .width(Length::Fill)
                .style(button::secondary)
        }
        UpdateStatus::Checking => button(text("Checking...").size(10).color([0.5, 0.5, 0.5]))
            .padding(Padding::from([4, 12]))
            .width(Length::Fill)
            .style(button::secondary),
        UpdateStatus::UpToDate => button(text("Up to Date").size(10))
            .on_press(Message::CheckForUpdates)
            .padding(Padding::from([4, 12]))
            .width(Length::Fill)
            .style(button::secondary),
        UpdateStatus::UpdateAvailable {
            version, html_url, ..
        } => button(text(format!("Update to v{}", version)).size(10))
            .on_press(Message::OpenUpdateUrl(html_url.clone()))
            .padding(Padding::from([4, 12]))
            .width(Length::Fill)
            .style(button::success),
    };

    let dropdown_content = column![
        // Dark mode toggle
        button(text(theme_label).size(10))
            .on_press(Message::ToggleDarkMode)
            .padding(Padding::from([4, 12]))
            .width(Length::Fill)
            .style(button::secondary),
        // Update button
        update_button,
    ]
    .spacing(2)
    .width(Length::Fixed(140.0));

    container(dropdown_content)
        .padding(4)
        .style(container::bordered_box)
        .into()
}

/// Render the settings dropdown menu (WASM version - no update checking)
#[cfg(target_arch = "wasm32")]
pub fn view_settings_menu(dark_mode: bool) -> Element<'static, Message> {
    let theme_label = if dark_mode { "Light Mode" } else { "Dark Mode" };

    let dropdown_content = column![
        // Dark mode toggle
        button(text(theme_label).size(10))
            .on_press(Message::ToggleDarkMode)
            .padding(Padding::from([4, 12]))
            .width(Length::Fill)
            .style(button::secondary),
    ]
    .spacing(2)
    .width(Length::Fixed(130.0));

    container(dropdown_content)
        .padding(4)
        .style(container::bordered_box)
        .into()
}
