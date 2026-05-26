use std::collections::HashSet;
use std::path::PathBuf;

use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, column, container, mouse_area, row, scrollable, text, text_input},
};

use crate::models::{
    ftp_entry::FtpEntry,
    message::Message,
    panel::PanelKind,
    sort::{SortKey, SortSpec, apply_view},
};
use crate::ui::theme::{
    ACCENT, BG_HOVER, BG_SELECTED, BG_SURFACE, BORDER_SUBTLE, SUCCESS, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};

pub fn file_panel<'a>(
    kind: PanelKind,
    path: &'a str,
    entries: &'a [FtpEntry],
    is_connected: bool,
    selected: &'a HashSet<String>,
    sort: SortSpec,
    filter: &'a str,
) -> Element<'a, Message> {
    let (icon, label, label_color) = match kind {
        PanelKind::Local => ("💻", "local", TEXT_MUTED),
        PanelKind::Remote => (
            "🖥",
            "remoto",
            if is_connected { SUCCESS } else { TEXT_MUTED },
        ),
    };

    let has_selection = !selected.is_empty();
    let single_selection = selected.len() == 1;
    let ops_enabled = match kind {
        PanelKind::Local => true,
        PanelKind::Remote => is_connected,
    };

    let header = container(
        row![
            text(icon).size(14),
            text(label).size(12).color(label_color),
            iced::widget::Space::new().width(Length::Fill),
            action_btn("+ carpeta", Message::MkdirPressed(kind), ops_enabled),
            action_btn(
                "renombrar",
                Message::RenamePressed(kind),
                ops_enabled && single_selection,
            ),
            action_btn(
                "eliminar",
                Message::DeletePressed(kind),
                ops_enabled && has_selection,
            ),
            action_btn(
                "↑",
                match kind {
                    PanelKind::Local => Message::LocalGoUp,
                    PanelKind::Remote => Message::RemoteGoUp,
                },
                true,
            ),
            action_btn(
                "↺",
                match kind {
                    PanelKind::Local => Message::LocalRefresh,
                    PanelKind::Remote => Message::RemoteRefresh,
                },
                match kind {
                    PanelKind::Local => true,
                    PanelKind::Remote => is_connected,
                },
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
                    PanelKind::Local => has_selection,
                    PanelKind::Remote => has_selection && is_connected,
                },
            ),
        ]
        .spacing(6)
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

    let breadcrumbs = breadcrumbs_row(kind, path);

    let filter_input = text_input("filtrar…", filter)
        .on_input(match kind {
            PanelKind::Local => Message::LocalFilterChanged,
            PanelKind::Remote => Message::RemoteFilterChanged,
        })
        .padding([4, 8])
        .size(11)
        .width(Length::Fixed(120.0))
        .style(filter_input_style);

    let col_headers = container(
        row![
            text("").size(10).width(20),
            sort_header("name", SortKey::Name, kind, sort, Length::Fill),
            sort_header("size", SortKey::Size, kind, sort, Length::Fixed(80.0)),
            sort_header(
                "modified",
                SortKey::Modified,
                kind,
                sort,
                Length::Fixed(90.0)
            ),
            filter_input,
        ]
        .spacing(6)
        .align_y(Alignment::Center),
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

    let visible = apply_view(entries, sort, filter);

    let file_rows: Vec<Element<Message>> = if visible.is_empty() {
        vec![
            container(
                text(if is_connected || matches!(kind, PanelKind::Local) {
                    if filter.trim().is_empty() {
                        "Carpeta vacía"
                    } else {
                        "Sin coincidencias"
                    }
                } else {
                    "Sin conexión"
                })
                .size(12)
                .color(TEXT_DIM),
            )
            .padding(20)
            .width(Length::Fill)
            .into(),
        ]
    } else {
        visible
            .iter()
            .map(|entry| file_row(entry, kind, selected))
            .collect()
    };

    let list = scrollable(column(file_rows)).height(Length::Fill);

    mouse_area(
        container(column![header, breadcrumbs, col_headers, list,])
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::PanelFocused(kind))
    .into()
}

fn breadcrumbs_row(kind: PanelKind, path: &str) -> Element<'static, Message> {
    let segments = build_breadcrumb_segments(kind, path);

    let mut items: Vec<Element<Message>> = Vec::new();
    for (i, (label, full_path)) in segments.into_iter().enumerate() {
        if i > 0 {
            items.push(text("›").size(10).color(TEXT_DIM).into());
        }
        let msg = match kind {
            PanelKind::Local => Message::LocalCrumbClicked(full_path),
            PanelKind::Remote => Message::RemoteCrumbClicked(full_path),
        };
        items.push(
            button(text(label).size(10).color(TEXT_MUTED))
                .on_press(msg)
                .padding([2, 4])
                .style(|_, status| button::Style {
                    background: Some(iced::Background::Color(match status {
                        button::Status::Hovered => BG_HOVER,
                        _ => iced::Color::TRANSPARENT,
                    })),
                    border: Border::default(),
                    text_color: TEXT_MUTED,
                    ..button::Style::default()
                })
                .into(),
        );
    }

    container(
        scrollable(row(items).spacing(2).align_y(Alignment::Center)).direction(
            scrollable::Direction::Horizontal(scrollable::Scrollbar::default()),
        ),
    )
    .padding([2, 12])
    .width(Length::Fill)
    .into()
}

fn build_breadcrumb_segments(kind: PanelKind, path: &str) -> Vec<(String, String)> {
    match kind {
        PanelKind::Local => {
            let p = std::path::Path::new(path);
            let mut parts = Vec::new();
            let mut accum = PathBuf::new();
            for comp in p.components() {
                use std::path::Component;
                accum.push(comp.as_os_str());
                let label = match comp {
                    Component::RootDir | Component::Prefix(_) => {
                        comp.as_os_str().to_string_lossy().to_string()
                    }
                    Component::Normal(name) => name.to_string_lossy().to_string(),
                    _ => continue,
                };
                parts.push((label, accum.to_string_lossy().to_string()));
            }
            if parts.is_empty() {
                parts.push((path.to_string(), path.to_string()));
            }
            parts
        }
        PanelKind::Remote => {
            let trimmed = path.trim_end_matches('/');
            if trimmed.is_empty() || trimmed == "/" {
                vec![("/".into(), "/".into())]
            } else {
                let mut parts = vec![("/".into(), "/".into())];
                let mut accum = String::new();
                for seg in trimmed.trim_start_matches('/').split('/') {
                    accum = if accum.is_empty() || accum == "/" {
                        format!("/{seg}")
                    } else {
                        format!("{accum}/{seg}")
                    };
                    parts.push((seg.to_string(), accum.clone()));
                }
                parts
            }
        }
    }
}

fn sort_header(
    label: &str,
    key: SortKey,
    kind: PanelKind,
    sort: SortSpec,
    width: Length,
) -> Element<'static, Message> {
    let active = sort.key == key;
    let arrow = if active {
        match sort.order {
            crate::models::sort::SortOrder::Asc => " ▴",
            crate::models::sort::SortOrder::Desc => " ▾",
        }
    } else {
        ""
    };

    let msg = match kind {
        PanelKind::Local => Message::LocalSortBy(key),
        PanelKind::Remote => Message::RemoteSortBy(key),
    };

    let label_text = format!("{label}{arrow}");
    let accent = active;

    container(
        button(text(label_text).size(10).color(TEXT_DIM))
            .on_press(msg)
            .padding([2, 0])
            .style(move |_, status| button::Style {
                background: Some(iced::Background::Color(match status {
                    button::Status::Hovered => BG_HOVER,
                    _ => iced::Color::TRANSPARENT,
                })),
                border: Border::default(),
                text_color: if accent { ACCENT } else { TEXT_DIM },
                ..button::Style::default()
            }),
    )
    .width(width)
    .into()
}

fn file_row<'a>(
    entry: &'a FtpEntry,
    kind: PanelKind,
    selected: &HashSet<String>,
) -> Element<'a, Message> {
    let is_selected = selected.contains(&entry.name);

    let open_msg = if entry.is_dir {
        match kind {
            PanelKind::Local => Message::LocalEntryOpened(entry.clone()),
            PanelKind::Remote => Message::RemoteEntryOpened(entry.clone()),
        }
    } else {
        match kind {
            PanelKind::Local => Message::LocalFileSelected {
                name: entry.name.clone(),
                shift: false,
                ctrl: false,
            },
            PanelKind::Remote => Message::RemoteFileSelected {
                name: entry.name.clone(),
                shift: false,
                ctrl: false,
            },
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
        Some(s) => format!("{s} B"),
        None => String::from("-"),
    };

    let modified_str = entry.modified.clone().unwrap_or_else(|| String::from("-"));
    let name = entry.name.clone();

    let row_btn = button(
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
    .on_press(open_msg)
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
    });

    mouse_area(row_btn)
        .on_right_press(Message::ContextMenuOpened {
            panel: kind,
            target: name,
        })
        .into()
}

fn action_btn(label: &'static str, msg: Message, enabled: bool) -> Element<'static, Message> {
    let btn = button(
        text(label)
            .size(11)
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
    .padding([3, 6]);

    if enabled {
        btn.on_press(msg).into()
    } else {
        btn.into()
    }
}

fn filter_input_style(_: &iced::Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(BG_SURFACE),
        border: Border {
            color: match status {
                text_input::Status::Focused { .. } => ACCENT,
                _ => BORDER_SUBTLE,
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
