use iced::{
    Alignment, Border, Element, Length,
    widget::{button, container, progress_bar, row, text},
};

use crate::models::{message::Message, transfer::TransferEntry};
use crate::ui::ftp_log_panel::ftp_log_toggle_btn;
use crate::ui::icons;
use crate::ui::theme::{ACCENT, BG_SURFACE, BORDER_SUBTLE, TEXT_DIM, TEXT_MUTED};

pub fn transfer_bar(
    active: Option<&TransferEntry>,
    queued_count: usize,
    queue_visible: bool,
    ftp_log_visible: bool,
) -> Element<'_, Message> {
    let status = match active {
        None => row![text("Sin transferencias activas").size(11).color(TEXT_DIM),]
            .align_y(Alignment::Center),

        Some(t) => {
            let action = match t.kind {
                crate::models::transfer::TransferKind::Upload => "subiendo",
                crate::models::transfer::TransferKind::Download => "bajando",
            };

            let progress_text = match (t.percent(), t.total_bytes) {
                (Some(p), _) => format!("{action} {} — {p:.0}%", t.filename),
                (None, Some(total)) if t.transferred_bytes > 0 => format!(
                    "{action} {} — {} / {}",
                    t.filename,
                    format_bytes(t.transferred_bytes),
                    format_bytes(total)
                ),
                _ => format!(
                    "{action} {} — {}",
                    t.filename,
                    format_bytes(t.transferred_bytes)
                ),
            };

            let bar = match t.total_bytes {
                Some(total) if total > 0 => {
                    progress_bar(0.0..=total as f32, t.transferred_bytes as f32).into()
                }
                _ => progress_bar(0.0..=100.0, 0.0).into(),
            };

            let mut row_items = vec![
                icons::transfer_kind(t.kind, 14),
                column_pair(progress_text, bar),
            ];

            row_items.push(
                button(icons::close(11))
                    .on_press(Message::CancelTransfer(t.id))
                    .padding([2, 6])
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .into(),
            );

            row(row_items).spacing(8).align_y(Alignment::Center)
        }
    };

    let queue_label = if queued_count > 0 {
        format!("cola ({queued_count})")
    } else {
        String::from("cola")
    };

    let queue_color = if queue_visible { ACCENT } else { TEXT_DIM };

    let content = row![
        ftp_log_toggle_btn(ftp_log_visible),
        queue_toggle_btn(&queue_label, queue_color, queue_visible),
        status,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(content)
        .padding([6, 14])
        .height(32)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(BG_SURFACE)),
            border: Border {
                color: BORDER_SUBTLE,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn column_pair(label: String, bar: Element<'_, Message>) -> Element<'_, Message> {
    use iced::widget::column;
    column![text(label).size(11).color(TEXT_MUTED), bar,]
        .spacing(4)
        .width(Length::Fill)
        .into()
}

fn queue_toggle_btn(label: &str, color: iced::Color, visible: bool) -> Element<'static, Message> {
    let action_label = if visible {
        format!("ocultar {label}")
    } else {
        label.to_string()
    };

    button(text(action_label).size(10).color(color))
        .on_press(Message::ToggleTransferQueue)
        .style(move |_, status| button::Style {
            background: Some(iced::Background::Color(match status {
                button::Status::Hovered => BG_SURFACE,
                _ => iced::Color::TRANSPARENT,
            })),
            border: Border {
                color: BORDER_SUBTLE,
                width: 0.5,
                radius: 4.0.into(),
            },
            text_color: color,
            ..button::Style::default()
        })
        .padding([3, 8])
        .into()
}

pub fn format_bytes(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    } else if n >= 1_024 {
        format!("{:.1} KB", n as f64 / 1_024.0)
    } else {
        format!("{n} B")
    }
}
