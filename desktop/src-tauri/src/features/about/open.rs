//! The URL is selected by the host allowlist before entering the OS adapter.
pub fn open(url: &str) -> Result<(), &'static str> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("/usr/bin/open")
            .arg(url)
            .status()
            .map_err(|_| "link_unavailable")?;
        if status.success() {
            Ok(())
        } else {
            Err("link_unavailable")
        }
    }
    #[cfg(target_os = "windows")]
    {
        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                window: *mut std::ffi::c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;
        }
        let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let operation = wide("open");
        let file = wide(url);
        let null = std::ptr::null();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                null,
                null,
                1,
            )
        };
        if result > 32 {
            Ok(())
        } else {
            Err("link_unavailable")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        Err("link_unavailable")
    }
}
