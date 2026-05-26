use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, row, text, text_input},
};

use crate::models::{message::Message, prompt::PromptDialog};
use crate::ui::theme::{
    ACCENT, BG_BASE, BG_SURFACE, BORDER, DANGER, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn prompt_modal(dialog: &PromptDialog) -> Element<'_, Message> {
    let (title, body, show_input) = match dialog {
        PromptDialog::Mkdir { value, .. } => (
            "Nueva carpeta",
            column![
                text("Nombre de la carpeta").size(11).color(TEXT_DIM),
                text_input("", value)
                    .on_input(Message::PromptValueChanged)
                    .padding(8)
                    .size(13)
                    .style(input_style),
            ]
            .spacing(4),
            true,
        ),
        PromptDialog::Rename { old, new, .. } => (
            "Renombrar",
            column![
                text(format!("Renombrar «{old}»")).size(11).color(TEXT_DIM),
                text_input("", new)
                    .on_input(Message::PromptValueChanged)
                    .padding(8)
                    .size(13)
                    .style(input_style),
            ]
            .spacing(4),
            true,
        ),
        PromptDialog::ConfirmDelete { names, .. } => {
            let label = if names.len() == 1 {
                format!("¿Eliminar «{}»?", names[0])
            } else {
                format!("¿Eliminar {} elementos?", names.len())
            };
            (
                "Confirmar eliminación",
                column![text(label).size(12).color(TEXT_PRIMARY),].spacing(4),
                false,
            )
        }
    };

    let mut actions = row![
        button(text("Cancelar").size(12).color(TEXT_MUTED))
            .on_press(Message::PromptCancel)
            .padding([6, 14])
            .style(cancel_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if show_input {
        actions = actions.push(
            button(text("Aceptar").size(12).color(BG_BASE))
                .on_press(Message::PromptSubmit)
                .padding([6, 14])
                .style(accept_btn_style),
        );
    } else {
        actions = actions.push(
            button(text("Eliminar").size(12).color(BG_BASE))
                .on_press(Message::PromptSubmit)
                .padding([6, 14])
                .style(delete_btn_style),
        );
    }

    let card_body = column![text(title).size(15).color(TEXT_PRIMARY), body, actions,].spacing(14);

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

fn input_style(_: &iced::Theme, status: text_input::Status) -> text_input::Style {
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

fn cancel_btn_style(_: &iced::Theme, _: button::Status) -> button::Style {
    button::Style {
        background: None,
        border: Border {
            color: BORDER,
            width: 0.5,
            radius: 5.0.into(),
        },
        text_color: TEXT_MUTED,
        ..button::Style::default()
    }
}

fn accept_btn_style(_: &iced::Theme, _: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(ACCENT)),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: BG_BASE,
        ..button::Style::default()
    }
}

fn delete_btn_style(_: &iced::Theme, _: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(DANGER)),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: BG_BASE,
        ..button::Style::default()
    }
}
