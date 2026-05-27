use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, checkbox, column, container, row, text, text_input},
};

use crate::models::{connection_form::ConnectionForm, message::Message};
use crate::ui::theme::{
    ACCENT, BG_BASE, BG_SURFACE, BORDER, DANGER, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn connection_modal(form: &ConnectionForm) -> Element<'_, Message> {
    let title = if form.editing_id.is_some() {
        "Editar conexión"
    } else {
        "Nueva conexión"
    };

    let fields = column![
        field("Nombre", &form.name, Message::ConnectionFormNameChanged),
        field("Host", &form.host, Message::ConnectionFormHostChanged),
        field("Puerto", &form.port, Message::ConnectionFormPortChanged),
        field(
            "Usuario",
            &form.username,
            Message::ConnectionFormUsernameChanged
        ),
        field_secret(
            "Contraseña",
            &form.password,
            Message::ConnectionFormPasswordChanged
        ),
        checkbox(form.active_mode)
            .label("Modo activo (desactivado = pasivo)")
            .on_toggle(Message::ConnectionFormModeChanged)
            .text_size(12)
            .style(checkbox_style),
        checkbox(form.use_ftps)
            .label("FTPS explícito (AUTH TLS)")
            .on_toggle(Message::ConnectionFormUseFtpsChanged)
            .text_size(12)
            .style(checkbox_style),
        checkbox(form.accept_invalid_certs)
            .label("Aceptar certificados inválidos")
            .on_toggle_maybe(if form.use_ftps {
                Some(Message::ConnectionFormAcceptInvalidCertsChanged)
            } else {
                None
            })
            .text_size(12)
            .style(checkbox_style),
    ]
    .spacing(10);

    let mut actions = row![
        button(text("Cancelar").size(12).color(TEXT_MUTED))
            .on_press(Message::ConnectionFormCancel)
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
        button(text("Guardar").size(12).color(BG_BASE))
            .on_press(Message::ConnectionFormSave)
            .padding([6, 14])
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

    if form.editing_id.is_some() {
        actions = actions.push(
            button(text("Eliminar").size(12).color(DANGER))
                .on_press(Message::ConnectionFormDelete)
                .padding([6, 14])
                .style(|_, _| button::Style {
                    background: None,
                    border: Border {
                        color: DANGER,
                        width: 0.5,
                        radius: 5.0.into(),
                    },
                    text_color: DANGER,
                    ..button::Style::default()
                }),
        );
    }

    let mut card_body = column![text(title).size(15).color(TEXT_PRIMARY)].spacing(14);

    if let Some(err) = &form.error {
        card_body = card_body.push(text(err).size(11).color(DANGER));
    }

    card_body = card_body.push(fields).push(actions);

    let card = container(card_body.padding(20).max_width(360)).style(|_| container::Style {
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

fn field<'a>(
    label: &'a str,
    value: &'a str,
    on_change: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(11).color(TEXT_DIM),
        text_input("", value)
            .on_input(on_change)
            .padding(8)
            .size(13)
            .style(|_, status| text_input::Style {
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
            }),
    ]
    .spacing(4)
    .into()
}

fn field_secret<'a>(
    label: &'a str,
    value: &'a str,
    on_change: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(11).color(TEXT_DIM),
        text_input("", value)
            .on_input(on_change)
            .padding(8)
            .size(13)
            .secure(true)
            .style(|_, status| text_input::Style {
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
            }),
    ]
    .spacing(4)
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
