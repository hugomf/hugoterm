use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label};

// Declare the external C function directly
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn make_window_transparent(gtk_window: *mut std::ffi::c_void);
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.transparent")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Transparent Test")
            .default_width(800)
            .default_height(600)
            .build();

        let label = Label::new(Some("Hello Transparent World!"));

        // Apply CSS for transparency
        let provider = gtk4::CssProvider::new();
        provider.load_from_data("window { background-color: transparent; }");

        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Apply macOS native transparency
        #[cfg(target_os = "macos")]
        {
            use std::time::Duration;
            let window_clone = window.clone();
            glib::timeout_add_local(Duration::from_millis(100), move || {
                unsafe {
                    make_window_transparent(window_clone.as_ptr() as *mut _);
                }
                glib::ControlFlow::Break
            });
        }

        window.set_child(Some(&label));
        window.present();
    });

    app.run();
}
