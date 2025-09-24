use anyhow::Result;
use gpui::prelude::*;
use gpui::{Application, WindowOptions};

mod browser_window;
mod tab_manager;

use browser_window::BrowserWindow;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Application::new();
    app.run(move |cx| {
        let window_options = WindowOptions {
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            cx.new(|cx| BrowserWindow::new(window, cx))
        }).unwrap();
    });

    Ok(())
}