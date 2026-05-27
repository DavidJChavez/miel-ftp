use iced::{
    Alignment, Border, Element, Length,
    widget::{button, column, container, mouse_area, row, text},
};

use crate::models::{context_menu::ContextMenu, message::Message, panel::PanelKind};
use crate::ui::theme::{ACCENT, BG_SURFACE, BORDER, TEXT_MUTED, TEXT_PRIMARY};

pub fn context_menu_overlay<'a>(
    menu: &'a ContextMenu,
    pending_edits: &'a [crate::models::remote_edit::RemoteEdit],
) -> Element<'a, Message> {
    let backdrop = mouse_area(
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::ContextMenuClosed);

    let mut items = vec![
        menu_item("Nueva carpeta", Message::MkdirPressed(menu.panel)),
        menu_item("Renombrar", Message::RenamePressed(menu.panel)),
        menu_item("Eliminar", Message::DeletePressed(menu.panel)),
    ];

    if matches!(menu.panel, PanelKind::Remote) {
        items.push(menu_item(
            "Editar",
            Message::EditRemoteFile(menu.target.clone()),
        ));
        if let Some(edit) = pending_edits.iter().find(|e| e.filename == menu.target) {
            items.push(menu_item(
                "Re-subir edición",
                Message::ReuploadRemoteEdit(edit.temp_path.clone()),
            ));
        }
    }

    items.push(menu_item(
        match menu.panel {
            PanelKind::Local => "Subir",
            PanelKind::Remote => "Bajar",
        },
        match menu.panel {
            PanelKind::Local => Message::UploadPressed,
            PanelKind::Remote => Message::DownloadPressed,
        },
    ));

    let menu_card = container(
        column![
            text(format!("«{}»", menu.target))
                .size(10)
                .color(TEXT_MUTED),
            column(items).spacing(2),
        ]
        .spacing(2)
        .padding(4),
    )
    .style(|_| container::Style {
        background: Some(iced::Background::Color(BG_SURFACE)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    });

    let positioned = container(menu_card).padding(iced::Padding {
        top: 120.0,
        right: 0.0,
        bottom: 0.0,
        left: 240.0,
    });

    container(
        column![backdrop, positioned,]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_item(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(
        row![text(label).size(12).color(TEXT_PRIMARY),]
            .align_y(Alignment::Center)
            .width(Length::Fill),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([6, 12])
    .style(|_, status| button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Hovered => ACCENT.scale_alpha(0.15),
            _ => iced::Color::TRANSPARENT,
        })),
        border: Border::default(),
        text_color: TEXT_MUTED,
        ..button::Style::default()
    })
    .into()
}
