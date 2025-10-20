
use gtk4::prelude::*;
use gtk4::ApplicationWindow;

// External C functions from our bridge
unsafe extern "C" {
    fn macos_blur_init() -> i32;
    fn macos_blur_apply_to_gtk_window(window: *mut gtk4::ffi::GtkWindow, radius: u32) -> i32;
    fn macos_set_titlebar_opaque(window: *mut gtk4::ffi::GtkWindow) -> i32;
}

/// Apply native macOS blur effect with transparency
pub fn apply_native_blur(window: &ApplicationWindow, opacity: f64, blur_strength: f64) {
    unsafe {
        // Initialize the blur system
        let init_result = macos_blur_init();
        if init_result != 0 {
            eprintln!("❌ Failed to initialize macOS blur system: {}", init_result);
            return;
        }
        println!("✅ macOS blur system initialized");

        // Convert blur_strength (0.0-1.0) to radius (0-100)
        let radius = (blur_strength * 100.0) as u32;
        
        // Apply blur
        let window_ptr = window.as_ptr() as *mut gtk4::ffi::GtkWindow;
        let blur_result = macos_blur_apply_to_gtk_window(window_ptr, radius);
        
        if blur_result == 0 {
            println!("✅ Blur applied: radius={}, opacity={}", radius, opacity);
        } else {
            eprintln!("❌ Failed to apply blur: result={}", blur_result);
        }
        
        // Make titlebar opaque
        let titlebar_result = macos_set_titlebar_opaque(window_ptr);
        if titlebar_result == 0 {
            println!("✅ Titlebar set to opaque");
        } else {
            eprintln!("❌ Failed to set titlebar opaque: result={}", titlebar_result);
        }
    }
}

/// Apply native macOS transparency without blur
pub fn apply_native_transparency(window: &ApplicationWindow, opacity: f64) {
    unsafe {
        // Initialize the system
        let init_result = macos_blur_init();
        if init_result != 0 {
            eprintln!("❌ Failed to initialize macOS blur system: {}", init_result);
            return;
        }
        
        // Apply zero blur (just transparency)
        let window_ptr = window.as_ptr() as *mut gtk4::ffi::GtkWindow;
        let blur_result = macos_blur_apply_to_gtk_window(window_ptr, 0);
        
        if blur_result == 0 {
            println!("✅ Transparency applied: opacity={}", opacity);
        } else {
            eprintln!("❌ Failed to apply transparency: result={}", blur_result);
        }
        
        // Make titlebar opaque
        let titlebar_result = macos_set_titlebar_opaque(window_ptr);
        if titlebar_result == 0 {
            println!("✅ Titlebar set to opaque");
        } else {
            eprintln!("❌ Failed to set titlebar opaque: result={}", titlebar_result);
        }
    }
}