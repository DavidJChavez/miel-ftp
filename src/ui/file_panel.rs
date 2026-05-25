use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, column, container, row, scrollable, text},
};

use crate::{
    app::{FtpEntry, Message},
    ui::theme::{
        BG_HOVER, BG_SELECTED, BG_SURFACE, BORDER_SUBTLE, SUCCESS, TEXT_DIM, TEXT_MUTED,
        TEXT_PRIMARY,
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
    selected: Option<&'a str>,
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
                },
                true
            ),
            action_btn(
                "↺",
                match kind {
                    PanelKind::Local => Message::LocalRefresh,
                    PanelKind::Remote => Message::RemoteRefresh,
                },
                true
            ),
            action_btn(
                match kind {
                    PanelKind::Local => "↑ subir",
                    PanelKind::Remote => "↓ bajar",
                },
                match kind {
                    PanelKind::Local => Message::UploadPressed,
                    PanelKind::Remote => Message::DownloadPressed,
                },
                match kind {
                    PanelKind::Local => selected.is_some(),
                    PanelKind::Remote => selected.is_some() && is_connected,
                }
            )
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
            text("name").size(10).color(TEXT_DIM).width(Length::Fill),
            text("size").size(10).color(TEXT_DIM).width(80),
            text("modified").size(10).color(TEXT_DIM).width(90),
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
                    "Empty folder"
                } else {
                    "Offline"
                })
                .size(12)
                .color(TEXT_DIM),
            )
            .padding(20)
            .width(Length::Fill)
            .into(),
        ]
    } else {
        entries
            .iter()
            .map(|entry| file_row(entry, kind, selected))
            .collect()
    };

    let list = scrollable(column(file_rows)).height(Length::Fill);

    container(column![header, col_headers, list])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn file_row<'a>(
    entry: &'a FtpEntry,
    kind: &'a PanelKind,
    selected: Option<&'a str>,
) -> Element<'a, Message> {
    let is_selected = selected == Some(entry.name.as_str());

    let on_press = if entry.is_dir {
        match kind {
            PanelKind::Local => Message::LocalEntryOpened(entry.clone()),
            PanelKind::Remote => Message::RemoteEntryOpened(entry.clone()),
        }
    } else {
        match kind {
            PanelKind::Local => Message::LocalFileSelected(entry.name.clone()),
            PanelKind::Remote => Message::RemoteFileSelected(entry.name.clone()),
        }
    };

    let bg = if is_selected {
        BG_SELECTED
    } else {
        iced::Color::TRANSPARENT
    };

    let icon = if entry.is_dir { "📁" } else { "📄" };

    let size_str = match entry.size {
        Some(s) if s >= 1_048_576 => format!("{:.1} MB", s as f64 / 1_048_576.0),
        Some(s) if s >= 1_024 => format!("{:.1} KB", s as f64 / 1_024.0),
        Some(s) => format!("{} B", s),
        None => String::from("-"),
    };

    let modified_str = entry.modified.clone().unwrap_or_else(|| String::from("-"));

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
    .style(move |_, status| button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => BG_SELECTED,
            _ => bg,
        })),
        text_color: TEXT_PRIMARY,
        border: Border::default(),
        ..button::Style::default()
    })
    .into()
}

fn action_btn(label: &str, msg: Message, enabled: bool) -> Element<Message> {
    let btn = button(
        text(label)
            .size(12)
            .color(if enabled { TEXT_MUTED } else { TEXT_DIM }),
    )
    .style(move |_, status| button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Hovered if enabled => BG_HOVER,
            _ => iced::Color::TRANSPARENT,
        })),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: if enabled { TEXT_MUTED } else { TEXT_DIM },
        ..button::Style::default()
    })
    .padding([3, 8]);

    if enabled {
        btn.on_press(msg).into()
    } else {
        btn.into()
    }
}
