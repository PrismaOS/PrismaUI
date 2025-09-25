use gpui::*;
use gpui_component::Root;

mod assets;
mod desktop;
mod window_manager;
mod shell;
mod components;

pub use assets::Assets;
use desktop::Desktop;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        let app = Application::new().with_assets(Assets);

        app.run(move |cx| {
            gpui_component::init(cx);
            cx.activate(true);

            // Check PRISMA_WINDOWED env
            let windowed = std::env::var("PRISMA_WINDOWED").map(|v| v == "true" || v == "1").unwrap_or(false);

            // Set up main OS window
            let mut window_size = size(px(1920.), px(1080.));
            if let Some(display) = cx.primary_display() {
                window_size = display.bounds().size;
            }
            let window_bounds = Bounds::from_corners(Point::default(), Point { x: window_size.width, y: window_size.height });

            cx.spawn(async move |cx| {
                let options = if windowed {
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                        titlebar: None,
                        window_decorations: Some(gpui::WindowDecorations::Client),
                        window_min_size: Some(gpui::Size {
                            width: px(1024.),
                            height: px(768.),
                        }),
                        kind: WindowKind::PopUp,
                        ..Default::default()
                    }
                } else {
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Fullscreen(window_bounds)),
                        titlebar: None,
                        window_decorations: Some(gpui::WindowDecorations::Client),
                        window_min_size: Some(gpui::Size {
                            width: px(1024.),
                            height: px(768.),
                        }),
                        kind: WindowKind::PopUp,
                        ..Default::default()
                    }
                };

                let window = cx
                    .open_window(options, |window, cx| {
                        let desktop = cx.new(|cx| Desktop::new(window, cx));
                        cx.new(|cx| Root::new(desktop.into(), window, cx))
                    })
                    .expect("failed to open main OS window");

                window
                    .update(cx, |_, window, _| {
                        window.activate_window();
                        window.set_window_title("PrismaUI OS");
                    })
                    .expect("failed to update window");

                Ok::<_, anyhow::Error>(())
            })
            .detach();
        });
    });
}