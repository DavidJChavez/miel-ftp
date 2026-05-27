use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, row, scrollable, text, text_input},
};

use crate::models::{
    connection::{Connection, ConnectionStatus},
    message::Message,
    quickconnect::QuickconnectForm,
};
use crate::ui::theme::{
    ACCENT, BG_BASE, BG_HOVER, BG_SELECTED, BG_SURFACE, BORDER, BORDER_SUBTLE, DANGER, SUCCESS,
    TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn sidebar<'a>(
    connections: &'a [Connection],
    selected_id: Option<uuid::Uuid>,
    remote_status: &'a ConnectionStatus,
    quickconnect_expanded: bool,
    quickconnect: &'a QuickconnectForm,
    bookmarks: &'a [String],
    is_connected: bool,
) -> Element<'a, Message> {
    let header = container(
        row![
            text("conexiones")
                .size(11)
                .color(TEXT_DIM)
                .style(|_| iced::widget::text::Style::default()),
            iced::widget::Space::new().width(Length::Fill),
            button(text("⚡").size(13).color(if quickconnect_expanded {
                ACCENT
            } else {
                TEXT_MUTED
            }))
            .on_press(Message::ToggleQuickconnect)
            .padding([2, 6])
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            }),
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

    let mut top = column![header].spacing(6);

    if quickconnect_expanded {
        top = top.push(quickconnect_panel(quickconnect));
    }

    let conn_list = scrollable(column(
        connections
            .iter()
            .map(|conn| conn_item(conn, selected_id, remote_status)),
    ))
    .height(Length::Fill);

    let mut footer_col = column![].spacing(4);

    if is_connected {
        footer_col = footer_col.push(
            button(text("desconectar").size(10).color(TEXT_DIM))
                .on_press(Message::Disconnected)
                .style(|_, _| button::Style {
                    background: None,
                    ..button::Style::default()
                })
                .padding([2, 2]),
        );
        footer_col = footer_col.push(
            button(text("★ marcar ruta").size(10).color(ACCENT))
                .on_press(Message::BookmarkAdd)
                .style(|_, _| button::Style {
                    background: None,
                    ..button::Style::default()
                })
                .padding([2, 2]),
        );
    }

    if !bookmarks.is_empty() {
        footer_col = footer_col.push(text("marcadores").size(10).color(TEXT_DIM));
        for path in bookmarks {
            let path_clone = path.clone();
            let display = truncate_path(path, 24);
            footer_col = footer_col.push(
                row![
                    button(text(display).size(10).color(TEXT_MUTED))
                        .on_press(Message::BookmarkNavigate(path_clone))
                        .style(|_, _| button::Style {
                            background: None,
                            ..button::Style::default()
                        })
                        .padding([2, 0]),
                    button(text("×").size(10).color(DANGER))
                        .on_press(Message::BookmarkRemove(path.clone()))
                        .padding([0, 4])
                        .style(|_, _| button::Style {
                            background: None,
                            ..button::Style::default()
                        }),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }
    }

    if let Some(id) = selected_id {
        footer_col = footer_col
            .push(
                button(text("conectar").size(11).color(ACCENT))
                    .on_press(Message::ConnectPressed)
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .padding([4, 2]),
            )
            .push(
                button(text("editar").size(11).color(TEXT_DIM))
                    .on_press(Message::EditConnectionPressed(id))
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .padding([4, 2]),
            );
    }

    footer_col = footer_col.push(
        button(
            row![
                text("⚙").size(14).color(TEXT_DIM),
                text("preferencias").size(12).color(TEXT_DIM),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleSettingsModal)
        .style(|_, _| button::Style {
            background: None,
            ..button::Style::default()
        })
        .width(Length::Fill)
        .padding([6, 2]),
    );

    let footer = container(footer_col)
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

    container(column![top, conn_list, footer])
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

fn quickconnect_panel(form: &QuickconnectForm) -> Element<'_, Message> {
    let mut col = column![
        row![
            text_input("host", &form.host)
                .on_input(Message::QuickconnectHostChanged)
                .padding([4, 6])
                .size(11)
                .width(Length::Fill)
                .style(text_input_style),
            text_input("puerto", &form.port)
                .on_input(Message::QuickconnectPortChanged)
                .padding([4, 6])
                .size(11)
                .width(Length::Fixed(48.0))
                .style(text_input_style),
        ]
        .spacing(4),
        text_input("usuario", &form.username)
            .on_input(Message::QuickconnectUsernameChanged)
            .padding([4, 6])
            .size(11)
            .width(Length::Fill)
            .style(text_input_style),
        text_input("contraseña", &form.password)
            .on_input(Message::QuickconnectPasswordChanged)
            .padding([4, 6])
            .size(11)
            .secure(true)
            .width(Length::Fill)
            .style(text_input_style),
        button(text("conectar").size(11).color(BG_BASE))
            .on_press(Message::QuickconnectConnect)
            .padding([4, 10])
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(ACCENT)),
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                text_color: BG_BASE,
                ..button::Style::default()
            }),
    ]
    .spacing(4)
    .padding(iced::Padding {
        top: 0.0,
        right: 14.0,
        bottom: 8.0,
        left: 14.0,
    });

    if let Some(err) = &form.error {
        col = col.push(text(err).size(10).color(DANGER));
    }

    col.into()
}

fn conn_item<'a>(
    conn: &'a Connection,
    selected_id: Option<uuid::Uuid>,
    remote_status: &'a ConnectionStatus,
) -> Element<'a, Message> {
    let is_selected = selected_id == Some(conn.id);

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
        text(format!("{} · {}", conn.host, conn.protocol.label()))
            .size(11)
            .color(TEXT_DIM),
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

fn truncate_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        path.to_string()
    } else {
        format!(
            "…{}",
            path.chars()
                .skip(path.chars().count().saturating_sub(max - 1))
                .collect::<String>()
        )
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
            radius: 4.0.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_DIM,
        value: TEXT_PRIMARY,
        selection: ACCENT,
    }
}
