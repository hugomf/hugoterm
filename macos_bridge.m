#import <gtk/gtk.h>
#import <gdk/macos/gdkmacos.h>
#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>

// Private CoreGraphics API types
typedef uint32_t CGSConnectionID;
typedef uint32_t CGSWindowID;

// Function pointer types for private APIs
typedef CGSConnectionID (*CGSDefaultConnectionForThreadFunc)(void);
typedef int32_t (*CGSSetWindowBackgroundBlurRadiusFunc)(CGSConnectionID, CGSWindowID, uint32_t);

// Global function pointers
static CGSDefaultConnectionForThreadFunc pCGSDefaultConnectionForThread = NULL;
static CGSSetWindowBackgroundBlurRadiusFunc pCGSSetWindowBackgroundBlurRadius = NULL;
static CGSConnectionID connection_id = 0;

// Initialize blur API (must be called once before using blur)
void init_blur_api(void) {
    if (pCGSSetWindowBackgroundBlurRadius != NULL) {
        return; // Already initialized
    }
    
    void* handle = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_LAZY);
    if (!handle) {
        fprintf(stderr, "Failed to load CoreGraphics framework\n");
        return;
    }
    
    pCGSDefaultConnectionForThread = (CGSDefaultConnectionForThreadFunc)dlsym(handle, "CGSDefaultConnectionForThread");
    pCGSSetWindowBackgroundBlurRadius = (CGSSetWindowBackgroundBlurRadiusFunc)dlsym(handle, "CGSSetWindowBackgroundBlurRadius");
    
    if (pCGSDefaultConnectionForThread && pCGSSetWindowBackgroundBlurRadius) {
        connection_id = pCGSDefaultConnectionForThread();
        printf("✅ Blur API initialized (connection: %u)\n", connection_id);
    } else {
        fprintf(stderr, "❌ Failed to load CGS functions\n");
    }
}

// Original function: set window with colored background and opacity
void set_window_opacity(void* gtk_window_ptr, double opacity, double red, double green, double blue) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            return;
        }
        
        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        NSColor* backgroundColor = [NSColor colorWithRed:red green:green blue:blue alpha:opacity];
        
        // Window must be non-opaque for content transparency
        [ns_window setOpaque:NO];
        
        // Keep titlebar normal (opaque with default color)
        [ns_window setTitlebarAppearsTransparent:NO];
        
        // Window background should be clear so titlebar stays default
        [ns_window setBackgroundColor:[NSColor clearColor]];
        
        // Apply tint only to content view (below titlebar)
        NSView* contentView = [ns_window contentView];
        [contentView setWantsLayer:YES];
        [[contentView layer] setOpaque:NO];
        [[contentView layer] setBackgroundColor:[backgroundColor CGColor]];
        
        [ns_window display];
        [ns_window invalidateShadow];
    }
}

// Set window opacity and blur in one call
// opacity: 0.0 to 1.0 (0.0 = fully transparent, 1.0 = fully opaque)
// blur_amount: 0.0 to 1.0 (0.0 = no blur, 1.0 = maximum blur)
int set_opacity_and_blur(void* gtk_window_ptr, double opacity, double blur_amount) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            return -1;
        }
        
        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        
        // Set window transparency
        [ns_window setOpaque:NO];
        [ns_window setTitlebarAppearsTransparent:NO];
        [ns_window setBackgroundColor:[NSColor clearColor]];
        
        // Always show shadow/border regardless of opacity
        [ns_window setHasShadow:YES];
        
        // Set content view opacity (using a semi-transparent black/white)
        NSView* contentView = [ns_window contentView];
        [contentView setWantsLayer:YES];
        [[contentView layer] setOpaque:NO];
        
        // Use opacity to create the background (you can change the color here)
        NSColor* backgroundColor = [NSColor colorWithWhite:0.0 alpha:1.0 - opacity];
        [[contentView layer] setBackgroundColor:[backgroundColor CGColor]];
        
        // Apply blur if requested and API is available
        // Convert blur_amount (0.0-1.0) to radius (0-100)
        uint32_t blur_radius = (uint32_t)(blur_amount * 100.0);
        if (blur_radius > 0 && connection_id != 0 && pCGSSetWindowBackgroundBlurRadius) {
            NSInteger window_number = [ns_window windowNumber];
            pCGSSetWindowBackgroundBlurRadius(connection_id, (CGSWindowID)window_number, blur_radius);
        }
        
        [ns_window display];
        [ns_window invalidateShadow];
        
        return 0;
    }
}

// Set content view background with color and opacity
void set_content_background(void* gtk_window_ptr, double opacity, double red, double green, double blue) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            return;
        }
        
        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        NSColor* backgroundColor = [NSColor colorWithRed:red green:green blue:blue alpha:opacity];
        
        NSView* contentView = [ns_window contentView];
        [contentView setWantsLayer:YES];
        [[contentView layer] setOpaque:NO];
        [[contentView layer] setBackgroundColor:[backgroundColor CGColor]];
        
        [ns_window display];
    }
}

// Set window transparency level (0.0 = fully transparent, 1.0 = fully opaque)
void set_window_alpha(void* gtk_window_ptr, double alpha) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            return;
        }
        
        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        
        // Enable transparency if alpha < 1.0
        if (alpha < 1.0) {
            [ns_window setOpaque:NO];
            [ns_window setBackgroundColor:[NSColor clearColor]];
        }
        
        [ns_window setAlphaValue:alpha];
        [ns_window display];
    }
}

// Apply blur to window content (requires window to be transparent)
int set_blur(void* gtk_window_ptr, uint32_t radius) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        
        if (!gtk_window) {
            fprintf(stderr, "❌ NULL GTK window provided\n");
            return -1;
        }

        if (connection_id == 0 || !pCGSSetWindowBackgroundBlurRadius) {
            fprintf(stderr, "❌ Blur API not initialized. Call init_blur_api() first\n");
            return -1;
        }

        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        if (!surface || !GDK_IS_MACOS_SURFACE(surface)) {
            fprintf(stderr, "❌ Invalid macOS surface\n");
            return -1;
        }

        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        if (!ns_window) {
            fprintf(stderr, "❌ Failed to get NSWindow\n");
            return -1;
        }

        // Ensure window is transparent for blur to work
        [ns_window setOpaque:NO];
        if (radius > 0) {
            [ns_window setBackgroundColor:[NSColor clearColor]];
        }
        [ns_window setHasShadow:YES];

        NSInteger window_number = [ns_window windowNumber];
        int32_t result = pCGSSetWindowBackgroundBlurRadius(connection_id, (CGSWindowID)window_number, radius);
        
        [ns_window invalidateShadow];
        
        return result;
    }
}