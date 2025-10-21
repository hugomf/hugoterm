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

// Apply blur to a GTK window using the official GTK4 macOS API
// int set_blur(GtkWindow *gtk_window, uint32_t radius) {

//     if (!gtk_window) {
//         fprintf(stderr, "❌ NULL GTK window provided\n");
//         return -1;
//     }

//     if (connection_id == 0 || !pCGSSetWindowBackgroundBlurRadius) {
//         fprintf(stderr, "❌ CGS APIs not initialized. Call macos_blur_init() first\n");
//         return -1;
//     }

//     // Get the GdkSurface from GTK window
//     GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
//     if (!surface) {
//         fprintf(stderr, "❌ Failed to get GdkSurface from GTK window\n");
//         return -1;
//     }

//     // Verify we have a macOS surface
//     if (!GDK_IS_MACOS_SURFACE(surface)) {
//         fprintf(stderr, "❌ Surface is not a GdkMacosSurface\n");
//         return -1;
//     }

//     // Use the official GTK4 macOS API to get NSWindow
//     NSWindow *ns_window = (__bridge NSWindow *)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
//     if (!ns_window) {
//         fprintf(stderr, "❌ Failed to get NSWindow from GdkMacosSurface\n");
//         return -1;
//     }

//     printf("✅ Successfully obtained NSWindow pointer: %p\n", (__bridge void*)ns_window);

//     // Configure NSWindow for transparency (required for blur to work)
//     [ns_window setOpaque:NO];
//     if (radius > 0) {
//         [ns_window setBackgroundColor:[NSColor clearColor]];
//     }
//     [ns_window setHasShadow:YES];

//     // Get window number for CGS API
//     NSInteger window_number = [ns_window windowNumber];
    
//     printf("📄 Applying blur: window_number=%ld, radius=%u\n", (long)window_number, radius);

//     // Apply blur using private CGS API
//     int32_t result = pCGSSetWindowBackgroundBlurRadius(connection_id, (CGSWindowID)window_number, radius);
    
//     printf("📄 Blur result: %d\n", result);

//     // Force redraw
//     [ns_window invalidateShadow];

//     return result;
// }