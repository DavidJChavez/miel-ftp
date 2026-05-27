use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{button, column, container, pick_list, row, scrollable, text},
};

use crate::models::{
    message::Message,
    sync::{SyncAction, SyncDiff, SyncState},
};
use crate::ui::theme::{ACCENT, BG_BASE, BG_SURFACE, BORDER, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

pub fn sync_panel(state: &SyncState) -> Element<'_, Message> {
    let title = if state.analyzing {
        "Comparando directorios…"
    } else {
        "Sincronizar directorios"
    };

    let body = if state.analyzing {
        column![
            text("Analizando diferencias local ↔ remoto…")
                .size(12)
                .color(TEXT_DIM),
        ]
    } else if state.entries.is_empty() {
        column![
            text("No hay diferencias entre los directorios actuales.")
                .size(12)
                .color(TEXT_DIM),
        ]
    } else {
        let rows: Vec<Element<Message>> =
            state.entries.iter().map(|entry| sync_row(entry)).collect();
        column![scrollable(column(rows).spacing(4)).height(Length::Fixed(280.0)),]
    };

    let card_body = column![
        text(title).size(15).color(TEXT_PRIMARY),
        body,
        row![
            iced::widget::Space::new().width(Length::Fill),
            button(text("Cancelar").size(12).color(TEXT_MUTED))
                .on_press(Message::CloseSyncPanel)
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
            if state.analyzing {
                button(text("…").size(12).color(TEXT_DIM)).padding([6, 14])
            } else {
                button(text("Aplicar").size(12).color(BG_BASE))
                    .on_press(Message::SyncApply)
                    .padding([6, 14])
                    .style(|_, _| button::Style {
                        background: Some(iced::Background::Color(ACCENT)),
                        border: Border {
                            radius: 5.0.into(),
                            ..Border::default()
                        },
                        text_color: BG_BASE,
                        ..button::Style::default()
                    })
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12);

    let card = container(card_body.padding(20).max_width(520)).style(|_| container::Style {
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

fn sync_row(entry: &crate::models::sync::SyncEntry) -> Element<'_, Message> {
    let diff_label = match entry.diff {
        SyncDiff::OnlyLocal => "solo local",
        SyncDiff::OnlyRemote => "solo remoto",
        SyncDiff::Both { local_newer: true } => "ambos (local más nuevo)",
        SyncDiff::Both { local_newer: false } => "ambos (remoto más nuevo)",
    };

    let actions: Vec<SyncAction> = SyncAction::all().to_vec();
    let selected = Some(entry.action);

    row![
        column![
            text(&entry.name).size(12).color(TEXT_PRIMARY),
            text(diff_label).size(10).color(TEXT_DIM),
        ]
        .spacing(2)
        .width(Length::Fill),
        pick_list(actions, selected, move |action| {
            Message::SyncEntryActionChanged(entry.name.clone(), action)
        })
        .placeholder("acción")
        .text_size(11)
        .padding([4, 8])
        .width(Length::Fixed(140.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
