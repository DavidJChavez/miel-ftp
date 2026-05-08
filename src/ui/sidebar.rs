use iced::{
    Alignment, Border, Element, Length,
    widget::{self, button, column, container, row, scrollable, text},
};

use crate::{
    app::{Connection, ConnectionStatus, Message},
    ui::theme::{
        ACCENT, BG_HOVER, BG_SELECTED, BG_SURFACE, BORDER, BORDER_SUBTLE, DANGER, SUCCESS,
        TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
    },
};

pub fn sidebar<'a>(
    connections: &'a [Connection],
    selected_id: Option<uuid::Uuid>,
    remote_status: &'a ConnectionStatus,
) -> Element<'a, Message> {
    // Header
    let header = container(
        row![
            text("conexiones")
                .size(11)
                .color(TEXT_DIM)
                .style(|_| iced::widget::text::Style::default()),
            iced::widget::Space::new().width(Length::Fill),
            button(text("+").size(15).color(TEXT_MUTED))
                .on_press(Message::AddConnectionPressed)
                .style(|_, _| button::Style {
                    background: None,
                    border: Border {
                        color: BORDER,
                        width: 0.5,
                        radius: 5.0.into(),
                    },
                    text_color: TEXT_MUTED,
                    ..button::Style::default()
                })
                .padding([2, 8]),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 12.0,
        right: 14.0,
        bottom: 8.0,
        left: 14.0,
    })
    .width(Length::Fill);

    // Connections list
    let conn_list = scrollable(column(
        connections
            .iter()
            .map(|conn| conn_item(conn, selected_id, remote_status)),
    ))
    .height(Length::Fill);

    // Footer
    let footer = container(
        button(
            row![
                text("⚙").size(14).color(TEXT_DIM),
                text("preferencias").size(12).color(TEXT_DIM),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::AddConnectionPressed) // TODO: Message::OpenSettings
        .style(|_, _| button::Style {
            background: None,
            ..button::Style::default()
        })
        .width(Length::Fill)
        .padding([6, 2]),
    )
    .padding([10, 14])
    .width(Length::Fill)
    .style(|_| container::Style {
        border: Border {
            color: BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    // Main wrapper
    container(column![header, conn_list, footer])
        .width(220)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(BG_SURFACE)),
            border: Border {
                color: BORDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn conn_item<'a>(
    conn: &'a Connection,
    selected_id: Option<uuid::Uuid>,
    remote_status: &'a ConnectionStatus,
) -> Element<'a, Message> {
    let is_selected = selected_id == Some(conn.id);

    // Status dot
    let dot_color = if is_selected {
        match remote_status {
            ConnectionStatus::Connected => SUCCESS,
            ConnectionStatus::Connecting => ACCENT,
            ConnectionStatus::Error(_) => DANGER,
            ConnectionStatus::Disconnected => TEXT_DIM,
        }
    } else {
        TEXT_DIM
    };

    let dot =
        container(iced::widget::Space::new().width(7).height(7)).style(move |_| container::Style {
            background: Some(iced::Background::Color(dot_color)),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let info = column![
        text(&conn.name).size(13).color(TEXT_PRIMARY),
        text(&conn.host).size(11).color(TEXT_DIM),
    ]
    .spacing(2);

    let bg = if is_selected {
        BG_SELECTED
    } else {
        iced::Color::TRANSPARENT
    };
    let border_color = if is_selected {
        ACCENT
    } else {
        iced::Color::TRANSPARENT
    };

    button(
        row![dot, info]
            .spacing(9)
            .align_y(Alignment::Center)
            .width(Length::Fill),
    )
    .on_press(Message::ConnectionSelected(conn.id))
    .width(Length::Fill)
    .padding([8, 14])
    .style(move |_, status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => BG_SELECTED,
            _ => bg,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border {
                color: border_color,
                width: if is_selected { 2.0 } else { 0.0 },
                radius: 0.0.into(),
            },
            text_color: TEXT_PRIMARY,
            ..button::Style::default()
        }
    })
    .into()
}
