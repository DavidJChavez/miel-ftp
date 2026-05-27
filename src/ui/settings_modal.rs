use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};

use crate::models::{message::Message, settings::AppSettings};
use crate::ui::theme::{
    ACCENT, BG_BASE, BG_SURFACE, BORDER, DANGER, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn settings_modal<'a>(
    settings: &'a AppSettings,
    custom_draft: &'a str,
    bandwidth_draft: &'a str,
) -> Element<'a, Message> {
    let hide_system = checkbox(settings.hide_system_files)
        .label("Ocultar archivos del sistema")
        .on_toggle(Message::SettingsHideSystemToggled)
        .text_size(12)
        .style(checkbox_style);

    let show_dotfiles = checkbox(settings.show_dotfiles)
        .label("Mostrar archivos ocultos (.)")
        .on_toggle(Message::SettingsShowDotfilesToggled)
        .text_size(12)
        .style(checkbox_style);

    let custom_rows: Vec<Element<Message>> = settings
        .custom_hidden
        .iter()
        .map(|name| {
            row![
                text(name).size(11).color(TEXT_PRIMARY).width(Length::Fill),
                button(text("×").size(12).color(DANGER))
                    .on_press(Message::SettingsCustomRemoved(name.clone()))
                    .padding([2, 6])
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    }),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        })
        .collect();

    let custom_list = if custom_rows.is_empty() {
        column![text("Sin nombres personalizados").size(11).color(TEXT_DIM),]
    } else {
        column(custom_rows).spacing(4)
    };

    let add_row = row![
        text_input("nombre exacto…", custom_draft)
            .on_input(Message::SettingsCustomDraftChanged)
            .padding([6, 8])
            .size(12)
            .width(Length::Fill)
            .style(text_input_style),
        button(text("+").size(14).color(BG_BASE))
            .on_press(Message::SettingsCustomAdded(custom_draft.to_string()))
            .padding([6, 12])
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(ACCENT)),
                border: Border {
                    radius: 5.0.into(),
                    ..Border::default()
                },
                text_color: BG_BASE,
                ..button::Style::default()
            }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let card_body = column![
        text("Preferencias").size(15).color(TEXT_PRIMARY),
        text("Filtros globales").size(12).color(TEXT_MUTED),
        text("Oculta entradas en ambos paneles sin volver a listar.")
            .size(11)
            .color(TEXT_DIM),
        hide_system,
        show_dotfiles,
        text("Nombres adicionales a ocultar")
            .size(11)
            .color(TEXT_DIM),
        scrollable(custom_list).height(Length::Fixed(80.0)),
        add_row,
        text("Transferencias").size(12).color(TEXT_MUTED),
        column![
            text("Límite de ancho de banda (KB/s, 0 = sin límite)")
                .size(11)
                .color(TEXT_DIM),
            text_input("0", bandwidth_draft)
                .on_input(Message::SettingsBandwidthDraftChanged)
                .padding([6, 8])
                .size(12)
                .width(Length::Fill)
                .style(text_input_style),
        ]
        .spacing(4),
        row![
            iced::widget::Space::new().width(Length::Fill),
            button(text("Cerrar").size(12).color(TEXT_MUTED))
                .on_press(Message::ToggleSettingsModal)
                .padding([6, 14])
                .style(|_, _| button::Style {
                    background: None,
                    border: Border {
                        color: BORDER,
                        width: 0.5,
                        radius: 5.0.into(),
                    },
                    text_color: TEXT_MUTED,
                    ..button::Style::default()
                }),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10);

    let card = container(card_body.padding(20).max_width(380)).style(|_| container::Style {
        background: Some(iced::Background::Color(BG_SURFACE)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    });

    container(
        container(card)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(Padding::ZERO)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color { a: 0.6, ..BG_BASE })),
                ..container::Style::default()
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn checkbox_style(_theme: &iced::Theme, status: checkbox::Status) -> checkbox::Style {
    checkbox::Style {
        background: iced::Background::Color(BG_BASE),
        icon_color: BG_BASE,
        border: Border {
            color: match status {
                checkbox::Status::Active { .. } => ACCENT,
                checkbox::Status::Hovered { .. } => ACCENT,
                _ => BORDER,
            },
            width: 1.0,
            radius: 3.0.into(),
        },
        text_color: Some(TEXT_PRIMARY),
    }
}

fn text_input_style(_: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(BG_BASE),
        border: Border {
            color: match status {
                text_input::Status::Focused { .. } => ACCENT,
                _ => BORDER,
            },
            width: 1.0,
            radius: 5.0.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_DIM,
        value: TEXT_PRIMARY,
        selection: ACCENT,
    }
}

pub fn settings_toggle_btn(active: bool) -> Element<'static, Message> {
    let (label, color) = if active {
        ("filtros ✓", ACCENT)
    } else {
        ("filtros", crate::ui::theme::TEXT_DIM)
    };

    button(text(label).size(10).color(color))
        .on_press(Message::ToggleSettingsModal)
        .style(move |_, status| button::Style {
            background: Some(iced::Background::Color(match status {
                button::Status::Hovered => BG_SURFACE,
                _ => iced::Color::TRANSPARENT,
            })),
            border: Border {
                color: BORDER,
                width: 0.5,
                radius: 4.0.into(),
            },
            text_color: color,
            ..button::Style::default()
        })
        .padding([3, 8])
        .into()
}
