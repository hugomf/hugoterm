fn main() {
    #[cfg(target_os = "macos")]
    {
        // Link against GTK4
        println!("cargo:rustc-link-lib=gtk-4.1");
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        
        // Find GTK4 installation
        let gtk_path = std::process::Command::new("pkg-config")
            .args(["--variable=libdir", "gtk4"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "/opt/homebrew/Cellar/gtk4/4.20.2/lib".to_string());
        
        println!("cargo:rustc-link-search={}", gtk_path);
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        
        // Link macOS frameworks
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");

        // Get GTK4 include paths
        let gtk_includes = std::process::Command::new("pkg-config")
            .args(["--cflags", "gtk4"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        let mut build = cc::Build::new();
        build.file("macos_bridge.m");
        build.flag("-Wall");
        build.flag("-Wextra");
        build.flag("-fobjc-arc");
        build.flag("-fmodules");
        build.flag("-Wno-ambiguous-macro");

        // Parse include paths from pkg-config
        for flag in gtk_includes.split_whitespace() {
            if let Some(path) = flag.strip_prefix("-I") {
                build.include(path);
            }
        }

        // Additional hardcoded includes for reliability
        let extra_includes = [
            "/opt/homebrew/Cellar/gtk4/4.20.2/include/gtk-4.0",
            "/opt/homebrew/Cellar/pango/1.57.0/include/pango-1.0",
            "/opt/homebrew/Cellar/fribidi/1.0.16/include/fribidi",
            "/opt/homebrew/Cellar/harfbuzz/12.1.0/include/harfbuzz",
            "/opt/homebrew/Cellar/graphite2/1.3.14/include",
            "/opt/homebrew/include/gdk-pixbuf-2.0",
            "/opt/homebrew/opt/libtiff/include",
            "/opt/homebrew/opt/zstd/include",
            "/opt/homebrew/Cellar/xz/5.8.1/include",
            "/opt/homebrew/opt/jpeg-turbo/include",
            "/opt/homebrew/Cellar/cairo/1.18.4/include/cairo",
            "/opt/homebrew/Cellar/fontconfig/2.17.1/include",
            "/opt/homebrew/opt/freetype/include/freetype2",
            "/opt/homebrew/opt/libpng/include/libpng16",
            "/opt/homebrew/Cellar/libxext/1.3.6/include",
            "/opt/homebrew/Cellar/xorgproto/2024.1/include",
            "/opt/homebrew/Cellar/libxrender/0.9.12/include",
            "/opt/homebrew/Cellar/libx11/1.8.12/include",
            "/opt/homebrew/Cellar/libxcb/1.17.0/include",
            "/opt/homebrew/Cellar/libxau/1.0.12/include",
            "/opt/homebrew/Cellar/libxdmcp/1.1.5/include",
            "/opt/homebrew/Cellar/pixman/0.46.4/include/pixman-1",
            "/opt/homebrew/Cellar/graphene/1.10.8/include/graphene-1.0",
            "/opt/homebrew/Cellar/graphene/1.10.8/lib/graphene-1.0/include",
            "/opt/homebrew/Cellar/glib/2.86.0/include",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX15.sdk/usr/include/ffi",
            "/opt/homebrew/Cellar/glib/2.86.0/include/glib-2.0",
            "/opt/homebrew/Cellar/glib/2.86.0/lib/glib-2.0/include",
            "/opt/homebrew/opt/gettext/include",
            "/opt/homebrew/Cellar/pcre2/10.46/include",
        ];

        for path in &extra_includes {
            build.include(path);
        }

        build.compile("macos_bridge");

        println!("cargo:rerun-if-changed=macos_bridge.m");
    }
}