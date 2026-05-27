use std::collections::HashSet;

use iced::{
    Alignment, Border, Element, Length,
    widget::{button, container, row, text},
};

use crate::models::{
    connection::ConnectionStatus, ftp_entry::FtpEntry, message::Message, panel::PanelKind,
    settings::AppSettings, sort::SortSpec,
};
use crate::ui::theme::{ACCENT, BG_SURFACE, BORDER_SUBTLE, TEXT_MUTED};
use crate::ui::transfer_bar::format_bytes;

pub fn status_bar(
    focused: PanelKind,
    entries: &[FtpEntry],
    selected: &HashSet<String>,
    sort: SortSpec,
    filter: &str,
    settings: &AppSettings,
    remote_status: &ConnectionStatus,
) -> Element<'static, Message> {
    let visible = crate::models::sort::apply_view(entries, sort, filter, settings);
    let total = visible.len();
    let selected_count = selected.len();

    let total_bytes: u64 = visible
        .iter()
        .filter(|e| !e.is_dir)
        .filter_map(|e| e.size)
        .sum();

    let panel_label = match focused {
        PanelKind::Local => "local",
        PanelKind::Remote => "remoto",
    };

    let selection_text = if selected_count > 0 {
        format!("{selected_count} seleccionados · ")
    } else {
        String::new()
    };

    let label = format!(
        "{panel_label}: {selection_text}{total} items · {}",
        format_bytes(total_bytes)
    );

    let is_connected = matches!(remote_status, ConnectionStatus::Connected);

    container(
        row![
            text(label).size(10).color(TEXT_MUTED),
            iced::widget::Space::new().width(Length::Fill),
            if is_connected {
                button(text("sincronizar").size(10).color(ACCENT))
                    .on_press(Message::OpenSyncPanel)
                    .padding([2, 8])
                    .style(|_, _| button::Style {
                        background: None,
                        border: Border {
                            color: BORDER_SUBTLE,
                            width: 0.5,
                            radius: 4.0.into(),
                        },
                        text_color: ACCENT,
                        ..button::Style::default()
                    })
            } else {
                button(text("sincronizar").size(10).color(TEXT_MUTED)).padding([2, 8])
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([4, 14])
    .height(26)
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
