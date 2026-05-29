//! Input view for Wood Beam editor
//!
//! Displays inputs organized into three tabs:
//! - Description: Beam properties (label, span, width, depth)
//! - Member Selection: Material, NDS factors, section deductions
//! - Loads: Load table with multiple discrete loads

use iced::widget::{
    button, checkbox, column, pick_list, row, rule, text, text_input, Column, Row, Space,
};
use iced::{Alignment, Element, Length, Padding};

use calc_core::calculations::continuous_beam::SupportType;
use calc_core::loads::LoadType;
use calc_core::materials::{
    GlulamLayup, GlulamStressClass, LumberSize, LvlGrade, PlyCount, PslGrade, WoodGrade,
    WoodSpecies,
};
use calc_core::nds_factors::{
    FlatUse, Incising, LoadDuration, RepetitiveMember, Temperature, WetService,
};
use calc_core::section_deductions::NotchLocation;

use crate::{App, DistributionType, InputTab, MaterialType, Message};

/// Render the beam editor with tabbed interface
pub fn view(app: &App) -> Column<'_, Message> {
    let editing_label = if app.selected_beam_id().is_some() {
        "Edit Beam"
    } else {
        "New Beam"
    };

    // Tab bar
    let tab_bar = view_tab_bar(app.selected_input_tab);

    // Tab content based on selection
    let tab_content: Element<'_, Message> = match app.selected_input_tab {
        InputTab::Description => view_description_tab(app, editing_label),
        InputTab::MemberSelection => view_member_tab(app),
        InputTab::Loads => view_loads_tab(app),
    };

    // Only show Delete button for existing beams (always visible regardless of tab)
    let action_buttons = if app.selected_beam_id().is_some() {
        row![button("Delete Beam")
            .on_press(Message::DeleteSelectedBeam)
            .padding(Padding::from([6, 12])),]
        .spacing(6)
    } else {
        row![].spacing(6)
    };

    column![
        tab_bar,
        rule::horizontal(1),
        Space::new().height(8),
        tab_content,
        Space::new().height(15),
        action_buttons,
    ]
}

/// Render the tab bar
fn view_tab_bar(selected: InputTab) -> Element<'static, Message> {
    let mut tabs = Row::new().spacing(4);

    for tab in InputTab::ALL {
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
                .on_press(Message::SelectInputTab(tab))
                .padding(Padding::from([6, 16]))
                .style(button::text)
        };

        tabs = tabs.push(btn);
    }

    tabs.into()
}

/// Description tab: beam properties (label, span, dimensions)
fn view_description_tab<'a>(app: &'a App, editing_label: &'a str) -> Element<'a, Message> {
    // Section dimensions - show size dropdown for sawn lumber
    let section_inputs: Element<'_, Message> =
        if app.selected_material_type == MaterialType::SawnLumber {
            view_sawn_lumber_section(app)
        } else {
            // Engineered wood uses manual width/depth inputs
            column![
                labeled_input("Width (in):", &app.width_in, Message::WidthChanged),
                labeled_input("Depth (in):", &app.depth_in, Message::DepthChanged),
            ]
            .spacing(6)
            .into()
        };

    // Span configuration section
    let span_section: Element<'_, Message> = if app.multi_span_mode {
        view_span_table(app)
    } else {
        labeled_input("Span (ft):", &app.span_ft, Message::SpanChanged)
    };

    let multi_span_toggle = checkbox(app.multi_span_mode)
        .label("Multi-span beam")
        .on_toggle(|_| Message::ToggleMultiSpanMode)
        .text_size(11);

    column![
        text(editing_label).size(14),
        Space::new().height(8),
        labeled_input("Label:", &app.beam_label, Message::BeamLabelChanged),
        Space::new().height(4),
        multi_span_toggle,
        Space::new().height(4),
        span_section,
        Space::new().height(4),
        section_inputs,
    ]
    .spacing(6)
    .into()
}

/// Member Selection tab: material, NDS factors, section deductions
fn view_member_tab(app: &App) -> Element<'_, Message> {
    let material_section = view_material_section(app);
    let adjustment_factors_section = view_adjustment_factors(app);
    let section_deductions_section = view_section_deductions(app);

    column![
        material_section,
        Space::new().height(10),
        adjustment_factors_section,
        Space::new().height(10),
        section_deductions_section,
    ]
    .into()
}

/// Loads tab: load table
fn view_loads_tab(app: &App) -> Element<'_, Message> {
    view_load_table(app)
}

/// Render sawn lumber section with size dropdown
fn view_sawn_lumber_section(app: &App) -> Element<'_, Message> {
    let designation_text = if app.selected_ply_count == PlyCount::Single {
        app.selected_lumber_size.display_name().to_string()
    } else {
        format!(
            "{}{}",
            app.selected_ply_count.prefix(),
            app.selected_lumber_size.display_name()
        )
    };

    column![
        row![
            text("Size:").size(11).width(Length::Fixed(80.0)),
            pick_list(
                &LumberSize::ALL[..],
                Some(app.selected_lumber_size),
                Message::LumberSizeSelected
            )
            .width(Length::Fixed(80.0))
            .text_size(11),
            Space::new().width(8),
            text("Plies:").size(11),
            Space::new().width(4),
            pick_list(
                &PlyCount::ALL[..],
                Some(app.selected_ply_count),
                Message::PlyCountSelected
            )
            .width(Length::Fixed(110.0))
            .text_size(11),
        ]
        .align_y(Alignment::Center),
        row![
            text("Actual:")
                .size(10)
                .width(Length::Fixed(80.0))
                .color([0.5, 0.5, 0.5]),
            text(format!(
                "{} = {}\" x {}\"",
                designation_text, app.width_in, app.depth_in
            ))
            .size(10)
            .color([0.5, 0.5, 0.5]),
        ]
        .align_y(Alignment::Center),
        // Custom size inputs (only shown if custom is selected)
        if app.selected_lumber_size.is_custom() {
            column![
                labeled_input("Width (in):", &app.width_in, Message::WidthChanged),
                labeled_input("Depth (in):", &app.depth_in, Message::DepthChanged),
            ]
            .spacing(6)
        } else {
            column![]
        },
    ]
    .spacing(4)
    .into()
}

/// Render the span table for multi-span beams
fn view_span_table(app: &App) -> Element<'_, Message> {
    // Header row
    let header = row![
        text("#").size(10).width(Length::Fixed(20.0)),
        text("Length (ft)").size(10).width(Length::Fixed(80.0)),
        text("Left Support").size(10).width(Length::Fixed(90.0)),
        text("").size(10).width(Length::Fixed(30.0)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // Build rows for each span
    let mut span_rows: Column<'_, Message> = column![].spacing(4);

    for (i, span_row) in app.span_table.iter().enumerate() {
        let row_id = span_row.id;
        let span_num = i + 1;

        let num_label = text(format!("{}.", span_num))
            .size(10)
            .width(Length::Fixed(20.0));

        let length_input = text_input("12.0", &span_row.length_ft)
            .on_input(move |s| Message::SpanLengthChanged(row_id, s))
            .width(Length::Fixed(80.0))
            .padding(2)
            .size(10);

        let support_picker = pick_list(
            &SupportType::ALL[..],
            Some(span_row.left_support),
            move |st| Message::SpanLeftSupportChanged(row_id, st),
        )
        .width(Length::Fixed(90.0))
        .text_size(10);

        // Only show delete button if we have more than 1 span
        let delete_btn: Element<'_, Message> = if app.span_table.len() > 1 {
            button(text("X").size(10))
                .on_press(Message::RemoveSpan(row_id))
                .padding(Padding::from([2, 6]))
                .into()
        } else {
            Space::new().width(30).into()
        };

        let span_row_widget: Row<'_, Message> =
            row![num_label, length_input, support_picker, delete_btn,]
                .spacing(4)
                .align_y(Alignment::Center);

        span_rows = span_rows.push(span_row_widget);
    }

    // Right-end support row (after all spans)
    let right_support_row = row![
        text("End").size(10).width(Length::Fixed(20.0)),
        Space::new().width(80),
        pick_list(
            &SupportType::ALL[..],
            Some(app.right_end_support),
            Message::RightEndSupportChanged,
        )
        .width(Length::Fixed(90.0))
        .text_size(10),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let add_span_btn = button(text("+ Add Span").size(10))
        .on_press(Message::AddSpan)
        .padding(Padding::from([4, 8]));

    // Total length display
    let total_length: f64 = app
        .span_table
        .iter()
        .filter_map(|s| s.length_ft.parse::<f64>().ok())
        .sum();

    column![
        text("Spans").size(12),
        Space::new().height(4),
        header,
        rule::horizontal(1),
        span_rows,
        right_support_row,
        Space::new().height(4),
        row![
            add_span_btn,
            Space::new().width(Length::Fill),
            text(format!("Total: {:.1} ft", total_length)).size(10),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(2)
    .into()
}

/// Render the load table
fn view_load_table(app: &App) -> Element<'_, Message> {
    let self_weight_checkbox = checkbox(app.include_self_weight)
        .label("Include self-weight")
        .on_toggle(Message::IncludeSelfWeightToggled)
        .text_size(11);

    // Header row
    let header = row![
        text("Type").size(10).width(Length::Fixed(45.0)),
        text("Dist").size(10).width(Length::Fixed(60.0)),
        text("Mag").size(10).width(Length::Fixed(55.0)),
        text("Position").size(10).width(Length::Fixed(70.0)),
        text("Trib").size(10).width(Length::Fixed(45.0)),
        text("").size(10).width(Length::Fixed(30.0)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // Build rows for each load
    let mut load_rows: Column<'_, Message> = column![].spacing(4);

    for load_row in &app.load_table {
        let row_id = load_row.id;

        let type_picker = pick_list(&LoadType::ALL[..], Some(load_row.load_type), move |lt| {
            Message::LoadTypeChanged(row_id, lt)
        })
        .width(Length::Fixed(45.0))
        .text_size(10);

        let dist_picker = pick_list(
            &DistributionType::ALL[..],
            Some(load_row.distribution),
            move |dt| Message::LoadDistributionChanged(row_id, dt),
        )
        .width(Length::Fixed(60.0))
        .text_size(10);

        let mag_input = text_input("0.0", &load_row.magnitude)
            .on_input(move |s| Message::LoadMagnitudeChanged(row_id, s))
            .width(Length::Fixed(55.0))
            .padding(2)
            .size(10);

        // Position input varies by distribution type
        let pos_widget: Element<'_, Message> = match load_row.distribution {
            DistributionType::UniformFull => text("-").size(10).width(Length::Fixed(70.0)).into(),
            DistributionType::Point => text_input("ft", &load_row.position)
                .on_input(move |s| Message::LoadPositionChanged(row_id, s))
                .width(Length::Fixed(70.0))
                .padding(2)
                .size(10)
                .into(),
            DistributionType::UniformPartial => row![
                text_input("0", &load_row.start_ft)
                    .on_input(move |s| Message::LoadStartChanged(row_id, s))
                    .width(Length::Fixed(32.0))
                    .padding(2)
                    .size(10),
                text("-").size(10),
                text_input("L", &load_row.end_ft)
                    .on_input(move |s| Message::LoadEndChanged(row_id, s))
                    .width(Length::Fixed(32.0))
                    .padding(2)
                    .size(10),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .width(Length::Fixed(70.0))
            .into(),
        };

        let trib_input = text_input("", &load_row.tributary_width)
            .on_input(move |s| Message::LoadTributaryChanged(row_id, s))
            .width(Length::Fixed(45.0))
            .padding(2)
            .size(10);

        let delete_btn = button(text("X").size(10))
            .on_press(Message::RemoveLoad(row_id))
            .padding(Padding::from([2, 6]));

        let load_row_widget: Row<'_, Message> = row![
            type_picker,
            dist_picker,
            mag_input,
            pos_widget,
            trib_input,
            delete_btn,
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        load_rows = load_rows.push(load_row_widget);
    }

    let add_load_btn = button(text("+ Add Load").size(10))
        .on_press(Message::AddLoad)
        .padding(Padding::from([4, 8]));

    column![
        text("Loads").size(14),
        Space::new().height(6),
        self_weight_checkbox,
        Space::new().height(6),
        header,
        rule::horizontal(1),
        load_rows,
        Space::new().height(6),
        add_load_btn,
    ]
    .spacing(2)
    .into()
}

/// Render the material selection section
fn view_material_section(app: &App) -> Column<'_, Message> {
    let material_options: Column<'_, Message> = match app.selected_material_type {
        MaterialType::SawnLumber => column![
            text("Species:").size(11),
            pick_list(
                &WoodSpecies::ALL[..],
                app.selected_species,
                Message::SpeciesSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
            Space::new().height(4),
            text("Grade:").size(11),
            pick_list(
                &WoodGrade::ALL[..],
                app.selected_grade,
                Message::GradeSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
        ]
        .spacing(2),
        MaterialType::Glulam => column![
            text("Stress Class:").size(11),
            pick_list(
                &GlulamStressClass::ALL[..],
                app.selected_glulam_class,
                Message::GlulamClassSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
            Space::new().height(4),
            text("Layup:").size(11),
            pick_list(
                &GlulamLayup::ALL[..],
                app.selected_glulam_layup,
                Message::GlulamLayupSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
        ]
        .spacing(2),
        MaterialType::Lvl => column![
            text("Grade:").size(11),
            pick_list(
                &LvlGrade::ALL[..],
                app.selected_lvl_grade,
                Message::LvlGradeSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
        ]
        .spacing(2),
        MaterialType::Psl => column![
            text("Grade:").size(11),
            pick_list(
                &PslGrade::ALL[..],
                app.selected_psl_grade,
                Message::PslGradeSelected
            )
            .width(Length::Fill)
            .placeholder("Select..."),
        ]
        .spacing(2),
    };

    column![
        text("Material").size(14),
        Space::new().height(8),
        text("Type:").size(11),
        pick_list(
            &MaterialType::ALL[..],
            Some(app.selected_material_type),
            Message::MaterialTypeSelected
        )
        .width(Length::Fill),
        Space::new().height(8),
        material_options,
    ]
    .spacing(2)
}

/// Render NDS adjustment factors section
fn view_adjustment_factors(app: &App) -> Element<'_, Message> {
    // Core factors that are commonly adjusted
    let core_factors = column![
        row![
            text("Load Duration:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &LoadDuration::ALL[..],
                Some(app.selected_load_duration),
                Message::LoadDurationSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        row![
            text("Wet Service:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &WetService::ALL[..],
                Some(app.selected_wet_service),
                Message::WetServiceSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        row![
            text("Repetitive:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &RepetitiveMember::ALL[..],
                Some(app.selected_repetitive_member),
                Message::RepetitiveMemberSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    // Less common factors
    let other_factors = column![
        row![
            text("Temperature:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &Temperature::ALL[..],
                Some(app.selected_temperature),
                Message::TemperatureSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        row![
            text("Incising:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &Incising::ALL[..],
                Some(app.selected_incising),
                Message::IncisingSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        row![
            text("Flat Use:").size(10).width(Length::Fixed(100.0)),
            pick_list(
                &FlatUse::ALL[..],
                Some(app.selected_flat_use),
                Message::FlatUseSelected
            )
            .width(Length::Fill)
            .text_size(10),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    // Beam stability (bracing)
    let bracing = checkbox(app.compression_edge_braced)
        .label("Compression edge braced (C_L = 1.0)")
        .on_toggle(Message::CompressionBracedToggled)
        .text_size(10);

    column![
        text("NDS Adjustment Factors").size(14),
        Space::new().height(6),
        core_factors,
        Space::new().height(6),
        other_factors,
        Space::new().height(6),
        bracing,
    ]
    .spacing(2)
    .into()
}

/// Render section deductions (notches and holes)
fn view_section_deductions(app: &App) -> Element<'_, Message> {
    // Notch location selector
    let notch_row = row![
        text("Notch:").size(10).width(Length::Fixed(60.0)),
        pick_list(
            &NotchLocation::ALL[..],
            Some(app.selected_notch_location),
            Message::NotchLocationSelected
        )
        .width(Length::Fill)
        .text_size(10),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // Notch depth inputs (shown only if notches are selected)
    let notch_depths: Element<'_, Message> = match app.selected_notch_location {
        NotchLocation::None => Space::new().height(0).into(),
        NotchLocation::LeftSupport => row![
            text("Depth (in):").size(10).width(Length::Fixed(60.0)),
            text_input("0.0", &app.notch_depth_left)
                .on_input(Message::NotchDepthLeftChanged)
                .width(Length::Fixed(60.0))
                .padding(2)
                .size(10),
            text("left").size(10).color([0.5, 0.5, 0.5]),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into(),
        NotchLocation::RightSupport => row![
            text("Depth (in):").size(10).width(Length::Fixed(60.0)),
            text_input("0.0", &app.notch_depth_right)
                .on_input(Message::NotchDepthRightChanged)
                .width(Length::Fixed(60.0))
                .padding(2)
                .size(10),
            text("right").size(10).color([0.5, 0.5, 0.5]),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into(),
        NotchLocation::BothSupports => column![
            row![
                text("Left (in):").size(10).width(Length::Fixed(60.0)),
                text_input("0.0", &app.notch_depth_left)
                    .on_input(Message::NotchDepthLeftChanged)
                    .width(Length::Fixed(60.0))
                    .padding(2)
                    .size(10),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            row![
                text("Right (in):").size(10).width(Length::Fixed(60.0)),
                text_input("0.0", &app.notch_depth_right)
                    .on_input(Message::NotchDepthRightChanged)
                    .width(Length::Fixed(60.0))
                    .padding(2)
                    .size(10),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .into(),
    };

    // Holes section
    let holes_row = row![
        text("Holes:").size(10).width(Length::Fixed(60.0)),
        text("Dia (in):").size(10),
        text_input("0.0", &app.hole_diameter)
            .on_input(Message::HoleDiameterChanged)
            .width(Length::Fixed(50.0))
            .padding(2)
            .size(10),
        Space::new().width(8),
        text("Qty:").size(10),
        text_input("0", &app.hole_count)
            .on_input(Message::HoleCountChanged)
            .width(Length::Fixed(40.0))
            .padding(2)
            .size(10),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    column![
        text("Section Deductions").size(14),
        Space::new().height(6),
        notch_row,
        notch_depths,
        Space::new().height(4),
        holes_row,
    ]
    .spacing(4)
    .into()
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
