use iced::{
    Alignment, Border, Element, Length,
    widget::{container, row, text},
};

use crate::{
    app::{Message, Transfer},
    ui::theme::{ACCENT, BG_SURFACE, BORDER_SUBTLE, TEXT_DIM, TEXT_MUTED},
};

pub fn transfer_bar(transfer: Option<&Transfer>) -> Element<Message> {
    let content = match transfer {
        None => row![text("Sin transferencias activas").size(11).color(TEXT_DIM),]
            .align_y(Alignment::Center),

        Some(t) => {
            let icon = if t.is_upload { "↑" } else { "↓" };
            let action = if t.is_upload { "subiendo" } else { "bajando" };

            row![
                text(icon).size(14).color(ACCENT),
                text(format!("{} {}", action, t.filename))
                    .size(11)
                    .color(TEXT_MUTED),
                iced::widget::Space::new().width(Length::Fill),
                // Simple spinner - three animated dots
                text("● ● ●").size(10).color(ACCENT),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        }
    };

    container(content)
        .padding([0, 14])
        .height(28)
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
