use iced::{
    Alignment, Border, Element, Length,
    widget::{container, row, text},
};

use crate::models::{message::Message, transfer::Transfer};
use crate::ui::ftp_log_panel::ftp_log_toggle_btn;
use crate::ui::theme::{ACCENT, BG_SURFACE, BORDER_SUBTLE, TEXT_DIM, TEXT_MUTED};

pub fn transfer_bar(
    transfer: Option<&Transfer>,
    bytes: u64,
    ftp_log_visible: bool,
) -> Element<'_, Message> {
    let status = match transfer {
        None => row![text("Sin transferencias activas").size(11).color(TEXT_DIM),]
            .align_y(Alignment::Center),

        Some(t) => {
            let icon = if t.is_upload { "↑" } else { "↓" };
            let action = if t.is_upload { "subiendo" } else { "bajando" };
            let progress = if bytes > 0 {
                format!(" — {bytes} B")
            } else {
                String::new()
            };

            row![
                text(icon).size(14).color(ACCENT),
                text(format!("{action} {}{progress}", t.filename))
                    .size(11)
                    .color(TEXT_MUTED),
                iced::widget::Space::new().width(Length::Fill),
                text("● ● ●").size(10).color(ACCENT),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        }
    };

    let content = row![ftp_log_toggle_btn(ftp_log_visible), status,]
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
