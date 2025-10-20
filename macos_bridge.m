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

// Initialize the private API connection
int macos_blur_init(void) {
    void *handle = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_NOW);
    if (!handle) {
        fprintf(stderr, "❌ Failed to open CoreGraphics framework: %s\n", dlerror());
        return -1;
    }

    // Try multiple symbol names for connection
    const char *connection_symbols[] = {
        "CGSDefaultConnectionForThread",
        "CGSMainConnectionID",
        "_CGSDefaultConnection",
        NULL
    };

    for (int i = 0; connection_symbols[i] != NULL; i++) {
        pCGSDefaultConnectionForThread = (CGSDefaultConnectionForThreadFunc)dlsym(handle, connection_symbols[i]);
        if (pCGSDefaultConnectionForThread) {
            connection_id = pCGSDefaultConnectionForThread();
            if (connection_id != 0) {
                printf("✅ Got CGS connection ID: %u using symbol: %s\n", connection_id, connection_symbols[i]);
                break;
            }
        }
    }

    if (connection_id == 0) {
        fprintf(stderr, "❌ Failed to get valid CGS connection ID\n");
        return -1;
    }

    // Load blur radius function
    pCGSSetWindowBackgroundBlurRadius = (CGSSetWindowBackgroundBlurRadiusFunc)dlsym(handle, "CGSSetWindowBackgroundBlurRadius");
    if (!pCGSSetWindowBackgroundBlurRadius) {
        fprintf(stderr, "❌ Failed to load CGSSetWindowBackgroundBlurRadius: %s\n", dlerror());
        return -1;
    }

    printf("✅ Private CGS APIs loaded successfully\n");
    return 0;
}

// Apply blur to a GTK window using the official GTK4 macOS API
int macos_blur_apply_to_gtk_window(GtkWindow *gtk_window, uint32_t radius) {
    if (!gtk_window) {
        fprintf(stderr, "❌ NULL GTK window provided\n");
        return -1;
    }

    if (connection_id == 0 || !pCGSSetWindowBackgroundBlurRadius) {
        fprintf(stderr, "❌ CGS APIs not initialized. Call macos_blur_init() first\n");
        return -1;
    }

    // Get the GdkSurface from GTK window
    GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
    if (!surface) {
        fprintf(stderr, "❌ Failed to get GdkSurface from GTK window\n");
        return -1;
    }

    // Verify we have a macOS surface
    if (!GDK_IS_MACOS_SURFACE(surface)) {
        fprintf(stderr, "❌ Surface is not a GdkMacosSurface\n");
        return -1;
    }

    // Use the official GTK4 macOS API to get NSWindow
    NSWindow *ns_window = (__bridge NSWindow *)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
    if (!ns_window) {
        fprintf(stderr, "❌ Failed to get NSWindow from GdkMacosSurface\n");
        return -1;
    }

    printf("✅ Successfully obtained NSWindow pointer: %p\n", (__bridge void*)ns_window);

    // Configure NSWindow for transparency (required for blur to work)
    [ns_window setOpaque:NO];
    if (radius > 0) {
        [ns_window setBackgroundColor:[NSColor clearColor]];
    }
    [ns_window setHasShadow:YES];

    // Get window number for CGS API
    NSInteger window_number = [ns_window windowNumber];
    
    printf("🔄 Applying blur: window_number=%ld, radius=%u\n", (long)window_number, radius);

    // Apply blur using private CGS API
    int32_t result = pCGSSetWindowBackgroundBlurRadius(connection_id, (CGSWindowID)window_number, radius);
    
    printf("🔄 Blur result: %d\n", result);

    // Force redraw
    [ns_window invalidateShadow];

    return result;
}

// Set titlebar to be opaque
int macos_set_titlebar_opaque(void* gtk_window_ptr) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        if (!gtk_window) {
            fprintf(stderr, "❌ NULL GTK window provided\n");
            return -1;
        }
        
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        if (!surface) {
            fprintf(stderr, "❌ Failed to get GdkSurface from GTK window\n");
            return -2;
        }
        
        // Verify we have a macOS surface
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            fprintf(stderr, "❌ Surface is not a GdkMacosSurface\n");
            return -3;
        }
        
        // Use the official GTK4 macOS API to get NSWindow (same as blur function)
        NSWindow *ns_window = (__bridge NSWindow *)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        if (!ns_window) {
            fprintf(stderr, "❌ Failed to get NSWindow from GdkMacosSurface\n");
            return -4;
        }
        
        printf("✅ Setting titlebar opaque for NSWindow: %p\n", (__bridge void*)ns_window);
        
        // Make title bar opaque
        [ns_window setTitlebarAppearsTransparent:NO];
        
        // Set title bar color to match control panel
        [ns_window setBackgroundColor:[NSColor clearColor]];
        
        printf("✅ Titlebar set to opaque successfully\n");
        
        return 0;
    }
}