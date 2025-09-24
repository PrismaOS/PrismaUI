use anyhow::Result;
use gpui::prelude::*;
use gpui::{App, WindowOptions};

mod browser_window;
mod tab_manager;

use browser_window::BrowserWindow;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new().run(|cx| {
        let window_options = WindowOptions {
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };

        cx.open_window(window_options, |cx| BrowserWindow::new(cx))
            .unwrap();
    });

    Ok(())
}