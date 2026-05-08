use iced::{Theme, color, theme::Palette};

pub const BG_BASE: iced::Color = color!(0x111111); // Main background color
pub const BG_SURFACE: iced::Color = color!(0x161616); // Sidebar, headers
pub const BG_HOVER: iced::Color = color!(0x1a1a1a); // Hover rows
pub const BG_SELECTED: iced::Color = color!(0x1c1c1c); // Selected row

pub const BORDER: iced::Color = color!(0x242424); // Main borders
pub const BORDER_SUBTLE: iced::Color = color!(0x1e1e1e); // Internal borders

pub const TEXT_PRIMARY: iced::Color = color!(0xe2e2e2);
pub const TEXT_MUTED: iced::Color = color!(0x888888);
pub const TEXT_DIM: iced::Color = color!(0x555555);

pub const ACCENT: iced::Color = color!(0xc9a84c); // Honey gold
pub const SUCCESS: iced::Color = color!(0x4caf7d); // Online green
pub const WARNING: iced::Color = color!(0xd4824a); // Warning orange
pub const DANGER: iced::Color = color!(0xe05c5c); // Error red

pub fn miel_theme() -> Theme {
    Theme::custom(
        String::from("miel"),
        Palette {
            background: BG_BASE,
            text: TEXT_PRIMARY,
            primary: ACCENT,
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
        },
    )
}
