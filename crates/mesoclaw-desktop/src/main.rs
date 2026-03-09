#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Fix WebKit DMA-BUF renderer failure on Linux production builds.
    // SAFETY: Called before any threads are spawned, single-threaded context.
    #[cfg(target_os = "linux")]
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    mesoclaw_desktop::run();
}
