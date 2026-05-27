use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, row, text},
};

use crate::models::{message::Message, toast::ToastKind};
use crate::ui::theme::{BG_SURFACE, BORDER, DANGER, SUCCESS, TEXT_MUTED, TEXT_PRIMARY};

pub fn toast_overlay<'a>(toasts: &'a [crate::models::toast::Toast]) -> Element<'a, Message> {
    let visible: Vec<_> = toasts.iter().rev().take(3).collect();

    if visible.is_empty() {
        return container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let cards: Vec<Element<Message>> = visible.into_iter().map(|toast| toast_card(toast)).collect();

    container(
        column(cards)
            .spacing(8)
            .align_x(Alignment::End)
            .width(Length::Fill),
    )
    .padding(iced::Padding {
        top: 0.0,
        right: 16.0,
        bottom: 16.0,
        left: 0.0,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::End)
    .align_y(Alignment::End)
    .into()
}

fn toast_card(toast: &crate::models::toast::Toast) -> Element<'_, Message> {
    let (bg, border) = match toast.kind {
        ToastKind::Info => (BG_SURFACE, BORDER),
        ToastKind::Success => (SUCCESS.scale_alpha(0.2), SUCCESS),
        ToastKind::Error => (DANGER.scale_alpha(0.2), DANGER),
    };

    container(
        row![
            text(&toast.message)
                .size(11)
                .color(TEXT_PRIMARY)
                .width(Length::Fill),
            button(text("×").size(12).color(TEXT_MUTED))
                .on_press(Message::DismissToast(toast.id))
                .padding([0, 4])
                .style(|_, _| button::Style {
                    background: None,
                    ..button::Style::default()
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fixed(320.0))
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}
