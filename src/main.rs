use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label};

// Declare the external C functions
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn set_window_opacity(
        gtk_window: *mut std::ffi::c_void, 
        opacity: f64, 
        red: f64, 
        green: f64, 
        blue: f64
    );
    
    fn set_opacity_and_blur(
        gtk_window: *mut std::ffi::c_void,
        opacity: f64,
        blur_amount: f64,
        red: f64, 
        green: f64, 
        blue: f64
    ) -> i32;
    
    fn init_blur_api();
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
        // Apply CSS to make GTK widgets transparent and add border
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            "window {
                background-color: transparent;
                background: transparent;
                border: 1px solid rgba(128, 128, 128, 0.3);
                border-radius: 10px;
            }
            
            * {
                background-color: transparent;
                background: transparent;
            }
            
            label {
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

        // Apply macOS transparency and blur
        #[cfg(target_os = "macos")]
        {
            use std::time::Duration;
            let window_clone = window.clone();
            


            // Initialize blur API first
            unsafe {
                init_blur_api();
            }

            let opacity = 0.5;     // 0.0 = fully transparent, 1.0 = fully opaque
            let blur_amount = 0.2;  // 0.0 = no blur, 1.0 = maximum blur
            let tint_color = "#1e1e1e";
            println!("🎨 Setting opacity: {}, blur: {}", opacity, blur_amount);

            if let Some((red, green, blue)) = hex_to_rgb(tint_color) {
                println!("🎡 Converting {} to RGB: ({:.4}, {:.4}, {:.4})", tint_color, red, green, blue);
            
                glib::timeout_add_local(Duration::from_millis(100), move || {
                    unsafe {
                        set_opacity_and_blur(
                            window_clone.as_ptr() as *mut _,
                            opacity,
                            blur_amount,
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