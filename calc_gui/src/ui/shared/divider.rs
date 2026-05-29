//! Resizable Panel Divider
//!
//! A draggable vertical divider that allows users to resize adjacent panels.

use iced::widget::{container, mouse_area, rule};
use iced::{mouse, Element, Length};

use crate::{DividerType, Message};

/// Create a draggable vertical divider between panels
///
/// The divider includes 15px padding on each side of a 1px vertical line,
/// matching the layout: panel[15px]|[15px]panel
pub fn view_divider(divider_type: DividerType, is_dragging: bool) -> Element<'static, Message> {
    // The actual visible line (1px wide)
    let line = rule::vertical(1);

    // Container with padding to create the 15px gaps on each side
    // Total width: 15px (left gap) + 1px (line) + 15px (right gap) = 31px
    let divider_content = container(line)
        .padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 14.0,
        })
        .height(Length::Fill);

    // Style the container based on dragging state
    let styled_content = container(divider_content).style(move |theme: &iced::Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: if is_dragging {
                Some(palette.primary.weak.color.into())
            } else {
                None
            },
            ..Default::default()
        }
    });

    // Use mouse_area to capture press events immediately (not on click completion)
    // This is crucial for drag behavior - we need to know when the button goes DOWN
    let interactive = mouse_area(styled_content)
        .on_press(Message::DividerDragStart(divider_type, 0.0))
        .on_release(Message::DividerDragEnd)
        .interaction(mouse::Interaction::ResizingHorizontally);

    interactive.into()
}
