use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, row, scrollable, text},
};

use crate::models::{ftp_log::FtpLog, message::Message};
use crate::ui::theme::{ACCENT, BG_BASE, BG_SURFACE, BORDER, BORDER_SUBTLE, TEXT_DIM, TEXT_MUTED};

pub fn ftp_log_panel(log: &FtpLog) -> Element<'_, Message> {
    let header = row![
        text("log FTP").size(11).color(TEXT_DIM),
        iced::widget::Space::new().width(Length::Fill),
        button(text("limpiar").size(10).color(TEXT_MUTED))
            .on_press(Message::ClearFtpLog)
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            })
            .padding([2, 6]),
        button(text("ocultar").size(10).color(ACCENT))
            .on_press(Message::ToggleFtpLog)
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            })
            .padding([2, 6]),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let lines: Vec<Element<Message>> = if log.lines().is_empty() {
        vec![
            text("Sin actividad FTP registrada.")
                .size(11)
                .color(TEXT_DIM)
                .into(),
        ]
    } else {
        log.lines()
            .iter()
            .map(|line| {
                text(line)
                    .size(10)
                    .color(TEXT_MUTED)
                    .width(Length::Fill)
                    .into()
            })
            .collect()
    };

    let body = scrollable(column(lines).spacing(2)).height(Length::Fill);

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

pub fn ftp_log_toggle_btn(visible: bool) -> Element<'static, Message> {
    let (label, color) = if visible {
        ("ocultar log", ACCENT)
    } else {
        ("log FTP", TEXT_DIM)
    };

    button(text(label).size(10).color(color))
        .on_press(Message::ToggleFtpLog)
        .style(move |_, status| button::Style {
            background: Some(iced::Background::Color(match status {
                button::Status::Hovered => BG_SURFACE,
                _ => iced::Color::TRANSPARENT,
            })),
            border: Border {
                color: BORDER,
                width: 0.5,
                radius: 4.0.into(),
            },
            text_color: color,
            ..button::Style::default()
        })
        .padding([3, 8])
        .into()
}
