use iced::widget::svg::{self, Handle, Svg};
use iced::{Color, Element, Length};

use crate::models::message::Message;
use crate::ui::theme::{ACCENT, DANGER, SUCCESS, TEXT_DIM, TEXT_MUTED};

const FOLDER: &[u8] = include_bytes!("../../assets/icons/folder.svg");
const FILE: &[u8] = include_bytes!("../../assets/icons/file.svg");
const MONITOR: &[u8] = include_bytes!("../../assets/icons/monitor.svg");
const SERVER: &[u8] = include_bytes!("../../assets/icons/server.svg");
const ARROW_UP: &[u8] = include_bytes!("../../assets/icons/arrow-up.svg");
const ARROW_DOWN: &[u8] = include_bytes!("../../assets/icons/arrow-down.svg");
const CORNER_LEFT_UP: &[u8] = include_bytes!("../../assets/icons/corner-left-up.svg");
const REFRESH_CW: &[u8] = include_bytes!("../../assets/icons/refresh-cw.svg");
const X: &[u8] = include_bytes!("../../assets/icons/x.svg");
const CHEVRON_RIGHT: &[u8] = include_bytes!("../../assets/icons/chevron-right.svg");

pub fn icon(bytes: &'static [u8], size: u16, color: Color) -> Element<'static, Message> {
    Svg::new(Handle::from_memory(bytes))
        .width(Length::Fixed(f32::from(size)))
        .height(Length::Fixed(f32::from(size)))
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

pub fn folder(size: u16) -> Element<'static, Message> {
    icon(FOLDER, size, TEXT_MUTED)
}

pub fn file(size: u16) -> Element<'static, Message> {
    icon(FILE, size, TEXT_MUTED)
}

pub fn computer(size: u16) -> Element<'static, Message> {
    icon(MONITOR, size, TEXT_MUTED)
}

pub fn server(size: u16, connected: bool) -> Element<'static, Message> {
    icon(SERVER, size, if connected { SUCCESS } else { TEXT_MUTED })
}

pub fn arrow_up(size: u16, color: Color) -> Element<'static, Message> {
    icon(ARROW_UP, size, color)
}

pub fn arrow_down(size: u16, color: Color) -> Element<'static, Message> {
    icon(ARROW_DOWN, size, color)
}

pub fn nav_up(size: u16) -> Element<'static, Message> {
    icon(CORNER_LEFT_UP, size, TEXT_MUTED)
}

pub fn refresh(size: u16) -> Element<'static, Message> {
    icon(REFRESH_CW, size, TEXT_MUTED)
}

pub fn close(size: u16) -> Element<'static, Message> {
    icon(X, size, DANGER)
}

pub fn chevron_right(size: u16) -> Element<'static, Message> {
    icon(CHEVRON_RIGHT, size, TEXT_DIM)
}

pub fn transfer_kind(
    kind: crate::models::transfer::TransferKind,
    size: u16,
) -> Element<'static, Message> {
    use crate::models::transfer::TransferKind;
    match kind {
        TransferKind::Upload => arrow_up(size, ACCENT),
        TransferKind::Download => arrow_down(size, ACCENT),
    }
}
