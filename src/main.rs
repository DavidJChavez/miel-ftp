mod app;
mod config;
mod error;
mod ftp;
mod models;
mod ui;

pub fn main() -> iced::Result {
    iced::application(app::boot, app::update, app::view)
        .title("MielFTP")
        .theme(app::theme)
        .run()
}
