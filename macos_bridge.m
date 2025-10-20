#import <AppKit/AppKit.h>
#import <gtk/gtk.h>
#import <gdk/macos/gdkmacos.h>

void make_window_transparent(void* gtk_window_ptr) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        if (GDK_IS_MACOS_SURFACE(surface)) {
            NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
            
            // Make content area transparent for blur
            [ns_window setOpaque:NO];
            [ns_window setBackgroundColor:[NSColor clearColor]];
            
            // Keep title bar completely normal (opaque, system style)
            [ns_window setTitlebarAppearsTransparent:NO];
            
            NSLog(@"✅ Window configured with opaque titlebar, transparent content");
        }
    }
}


