mod app;
mod config;
mod error;
mod ftp;
mod models;
mod tracing_init;
mod ui;

pub fn main() -> iced::Result {
    tracing_init::init();

    iced::application(app::boot, app::update, app::view)
        .subscription(app::subscription)
        .title("MielFTP")
        .theme(app::theme)
        .run()
}
