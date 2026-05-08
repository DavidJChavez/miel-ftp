use std::fmt::format;

use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, column, container, row, scrollable, text},
};

use crate::{
    app::{FtpEntry, Message},
    ui::theme::{
        ACCENT, BG_HOVER, BG_SELECTED, BG_SURFACE, BORDER, BORDER_SUBTLE, SUCCESS, TEXT_DIM,
        TEXT_MUTED, TEXT_PRIMARY,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum PanelKind {
    Local,
    Remote,
}

pub fn file_panel<'a>(
    kind: &'a PanelKind,
    path: &'a str,
    entries: &'a [FtpEntry],
    is_connected: bool,
) -> Element<'a, Message> {
    let (icon, label, label_color) = match kind {
        PanelKind::Local => ("💻", "local", TEXT_MUTED),
        PanelKind::Remote => (
            "🖥",
            "remoto",
            if is_connected { SUCCESS } else { TEXT_MUTED },
        ),
    };

    // Dashboard Header
    let header = container(
        row![
            text(icon).size(14),
            text(label).size(12).color(label_color),
            iced::widget::Space::new().width(Length::Fill),
            text(path).size(11).color(TEXT_DIM),
            action_btn(
                "↑",
                match kind {
                    PanelKind::Local => Message::LocalGoUp,
                    PanelKind::Remote => Message::RemoteGoUp,
                }
            ),
            action_btn(
                "↺",
                match kind {
                    PanelKind::Local => Message::LocalRefresh,
                    PanelKind::Remote => Message::RemoteRefresh,
                }
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 12.0,
    })
    .height(36)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(BG_SURFACE)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    // Header columns
    let col_headers = container(
        row![
            text("").size(10).width(20),
            text("nombre").size(10).color(TEXT_DIM).width(Length::Fill),
            text("tamaño").size(10).color(TEXT_DIM).width(80),
            text("modificado").size(10).color(TEXT_DIM).width(90),
        ]
        .spacing(6),
    )
    .padding([4, 12])
    .width(Length::Fill)
    .style(|_| container::Style {
        border: Border {
            color: BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    });

    // Files list
    let file_rows: Vec<Element<Message>> = if entries.is_empty() {
        vec![
            container(
                text(if is_connected || matches!(kind, PanelKind::Local) {
                    "carpeta vacía"
                } else {
                    "sin conexión"
                })
                .size(12)
                .color(TEXT_DIM),
            )
            .padding(20)
            .width(Length::Fill)
            .into(),
        ]
    } else {
        entries.iter().map(|entry| file_row(entry, kind)).collect()
    };

    let list = scrollable(column(file_rows)).height(Length::Fill);

    container(column![header, col_headers, list])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn file_row<'a>(entry: &'a FtpEntry, kind: &'a PanelKind) -> Element<'a, Message> {
    let icon = if entry.is_dir { "📁" } else { "📄" };

    let size_str = match entry.size {
        Some(s) if s >= 1_048_576 => format!("{:.1} MB", s as f64 / 1_048_576.0),
        Some(s) if s >= 1_024 => format!("{:.1} KB", s as f64 / 1_024.0),
        Some(s) => format!("{} B", s),
        None => String::from("-"),
    };

    let modified_str = entry.modified.clone().unwrap_or_else(|| String::from("-"));

    let on_press = if entry.is_dir {
        match kind {
            PanelKind::Local => Message::LocalEntryOpened(entry.clone()),
            PanelKind::Remote => Message::RemoteEntryOpened(entry.clone()),
        }
    } else {
        match kind {
            PanelKind::Local => Message::UploadPressed(entry.clone()),
            PanelKind::Remote => Message::DownloadPressed(entry.clone()),
        }
    };

    button(
        row![
            text(icon).size(13).width(20),
            text(&entry.name)
                .size(12)
                .color(TEXT_PRIMARY)
                .width(Length::Fill),
            text(size_str).size(11).color(TEXT_MUTED).width(80),
            text(modified_str).size(11).color(TEXT_DIM).width(90),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(on_press)
    .width(Length::Fill)
    .padding([5, 12])
    .style(|_, status| button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => BG_SELECTED,
            _ => iced::Color::TRANSPARENT,
        })),
        text_color: TEXT_PRIMARY,
        border: Border::default(),
        ..button::Style::default()
    })
    .into()
}

fn action_btn(label: &str, msg: Message) -> Element<Message> {
    button(text(label).size(14).color(TEXT_MUTED))
        .on_press(msg)
        .style(|_, status| button::Style {
            background: Some(iced::Background::Color(match status {
                button::Status::Hovered => BG_HOVER,
                _ => iced::Color::TRANSPARENT,
            })),
            border: Border {
                radius: 5.0.into(),
                ..Border::default()
            },
            text_color: TEXT_MUTED,
            ..button::Style::default()
        })
        .padding([3, 6])
        .into()
}
