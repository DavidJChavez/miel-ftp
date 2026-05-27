use std::collections::HashSet;

use iced::{
    Alignment, Border, Element, Length,
    widget::{container, row, text},
};

use crate::models::{
    ftp_entry::FtpEntry, message::Message, panel::PanelKind, settings::AppSettings, sort::SortSpec,
};
use crate::ui::theme::{BG_SURFACE, BORDER_SUBTLE, TEXT_MUTED};
use crate::ui::transfer_bar::format_bytes;

pub fn status_bar(
    focused: PanelKind,
    entries: &[FtpEntry],
    selected: &HashSet<String>,
    sort: SortSpec,
    filter: &str,
    settings: &AppSettings,
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

    container(
        row![text(label).size(10).color(TEXT_MUTED),]
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
