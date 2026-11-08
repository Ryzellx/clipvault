pub fn platform_name() -> &'static str {
    #[cfg(windows)]
    {
        "Windows"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "Unsupported"
    }
}

pub fn clipboard_backend_note() -> &'static str {
    #[cfg(windows)]
    {
        "Native clipboard viewer chain (Win32). Captures text and images."
    }
    #[cfg(target_os = "linux")]
    {
        "X11 clipboard polling via arboard. On Wayland, persistent clipboard access is unavailable: the compositor only grants clipboard content while ClipVault owns the selection, so history capture is limited and ClipVault says so instead of pretending otherwise."
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "Clipboard monitoring is not supported on this platform."
    }
}

pub fn wayland_limited() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("DISPLAY").is_err()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub fn active_window_process() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        None
    }
    #[cfg(windows)]
    {
        None
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}
