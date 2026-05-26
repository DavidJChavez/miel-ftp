use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, row, scrollable, text},
};

use crate::models::{
    message::Message,
    transfer::{TransferEntry, TransferKind, TransferStatus},
};
use crate::ui::theme::{ACCENT, BG_BASE, BORDER_SUBTLE, DANGER, SUCCESS, TEXT_DIM, TEXT_MUTED};
use crate::ui::transfer_bar::format_bytes;

pub fn transfer_queue_panel(transfers: &[TransferEntry]) -> Element<'_, Message> {
    let header = row![
        text("cola de transferencias").size(11).color(TEXT_DIM),
        iced::widget::Space::new().width(Length::Fill),
        button(text("limpiar terminadas").size(10).color(TEXT_MUTED))
            .on_press(Message::ClearCompletedTransfers)
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            })
            .padding([2, 6]),
        button(text("ocultar").size(10).color(ACCENT))
            .on_press(Message::ToggleTransferQueue)
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            })
            .padding([2, 6]),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let rows: Vec<Element<Message>> = if transfers.is_empty() {
        vec![
            text("No hay transferencias en cola.")
                .size(11)
                .color(TEXT_DIM)
                .into(),
        ]
    } else {
        transfers.iter().map(transfer_row).collect()
    };

    let body = scrollable(column(rows).spacing(4)).height(Length::Fill);

    container(column![header, body].spacing(6).padding([8, 12]))
        .height(140)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(BG_BASE)),
            border: Border {
                color: BORDER_SUBTLE,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn transfer_row(entry: &TransferEntry) -> Element<'_, Message> {
    let icon = match entry.kind {
        TransferKind::Upload => "↑",
        TransferKind::Download => "↓",
    };

    let progress = match (entry.percent(), entry.total_bytes) {
        (Some(p), _) => format!("{p:.0}%"),
        (None, Some(total)) => format!(
            "{} / {}",
            format_bytes(entry.transferred_bytes),
            format_bytes(total)
        ),
        _ => format_bytes(entry.transferred_bytes),
    };

    let status_color = match &entry.status {
        TransferStatus::Done => SUCCESS,
        TransferStatus::Failed(_) | TransferStatus::Cancelled => DANGER,
        TransferStatus::Active => ACCENT,
        TransferStatus::Queued => TEXT_MUTED,
    };

    let status_text = match &entry.status {
        TransferStatus::Failed(msg) => format!("fallida: {msg}"),
        _ => entry.status_label().to_string(),
    };

    let mut actions = row![].spacing(6);

    match &entry.status {
        TransferStatus::Queued | TransferStatus::Active => {
            actions = actions.push(
                button(text("cancelar").size(10).color(DANGER))
                    .on_press(Message::CancelTransfer(entry.id))
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .padding([1, 4]),
            );
        }
        TransferStatus::Failed(_) | TransferStatus::Cancelled => {
            actions = actions.push(
                button(text("reintentar").size(10).color(ACCENT))
                    .on_press(Message::RetryTransfer(entry.id))
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .padding([1, 4]),
            );
        }
        TransferStatus::Done => {}
    }

    row![
        text(icon).size(12).color(ACCENT).width(16),
        column![
            text(&entry.filename).size(11).color(TEXT_MUTED),
            text(format!("{status_text} — {progress}"))
                .size(10)
                .color(status_color),
        ]
        .spacing(2)
        .width(Length::Fill),
        actions,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([2, 0])
    .into()
}
