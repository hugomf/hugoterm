use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label};

// Declare the external C function directly
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn set_window_opacity(
        gtk_window: *mut std::ffi::c_void, 
        opacity: f64, 
        red: f64, 
        green: f64, 
        blue: f64
    );
}

fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    let red = ((rgb >> 16) & 0xff) as f64 / 255.0;
    let green = ((rgb >> 8) & 0xff) as f64 / 255.0;
    let blue = (rgb & 0xff) as f64 / 255.0;
    
    Some((red, green, blue))
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.transparent")
        .build();

    app.connect_activate(|app| {
        // Apply CSS for transparency BEFORE creating the window
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            "window {
                background-color: transparent;
                background: transparent;
            }
            
            * {
                background-color: transparent;
                background: transparent;
            }
            
            label {
                background-color: transparent;
                background: transparent;
                color: white;
            }"
        );

        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Transparent Test")
            .default_width(800)
            .default_height(600)
            .build();

        let label = Label::new(Some("Hello Transparent World!"));
        window.set_child(Some(&label));

        // Apply macOS native transparency
        #[cfg(target_os = "macos")]
        {
            use std::time::Duration;
            let window_clone = window.clone();
            
            let test_opacity = 0.7;
            let test_color = "#ff0000";
            
            if let Some((red, green, blue)) = hex_to_rgb(test_color) {
                println!("🎡 Converting {} to RGB: ({:.4}, {:.4}, {:.4})", test_color, red, green, blue);

                glib::timeout_add_local(Duration::from_millis(100), move || {
                    unsafe {
                        set_window_opacity(
                            window_clone.as_ptr() as *mut _, 
                            test_opacity, 
                            red, 
                            green, 
                            blue
                        );
                    }
                    glib::ControlFlow::Break
                });
            }
        }

        window.present();
    });

    app.run();
}