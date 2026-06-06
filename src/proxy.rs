#[allow(unsafe_code)]
mod ffi {
    use std::ffi::c_void;

    #[link(name = "wininet")]
    unsafe extern "system" {
        pub fn InternetSetOptionW(
            hinternet: *mut c_void,
            option: u32,
            buffer: *mut c_void,
            buffer_length: u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    pub const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    pub const INTERNET_OPTION_REFRESH: u32 = 37;
}

use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
};

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

pub struct ProxyGuard {
    original_enable: u32,
    original_server: Option<String>,
    restored: bool,
}

impl ProxyGuard {
    pub fn set(proxy_addr: &str) -> anyhow::Result<Self> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_WRITE)
            .map_err(|e| anyhow::anyhow!("failed to open Internet Settings: {e}"))?;

        let original_enable: u32 = key.get_value("ProxyEnable").unwrap_or(0);
        let original_server: Option<String> = key.get_value("ProxyServer").ok();

        key.set_value("ProxyEnable", &1u32)?;
        key.set_value("ProxyServer", &proxy_addr)?;

        notify_proxy_changed();
        eprintln!("[dae-box] System proxy set to {proxy_addr}");

        Ok(Self {
            original_enable,
            original_server,
            restored: false,
        })
    }

    fn restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_WRITE) {
            let _ = key.set_value("ProxyEnable", &self.original_enable);
            if let Some(ref server) = self.original_server {
                let _ = key.set_value("ProxyServer", server);
            } else {
                let _ = key.delete_value("ProxyServer");
            }
        }

        notify_proxy_changed();
        eprintln!("[dae-box] System proxy restored");
    }
}

impl Drop for ProxyGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

pub fn set_ctrlc_handler() -> anyhow::Result<()> {
    unsafe extern "system" fn ctrl_handler(_ctrl_type: u32) -> i32 {
        1
    }

    let result = unsafe { ffi::SetConsoleCtrlHandler(Some(ctrl_handler), 1) };
    if result == 0 {
        anyhow::bail!("SetConsoleCtrlHandler failed");
    }
    Ok(())
}

fn notify_proxy_changed() {
    unsafe {
        ffi::InternetSetOptionW(
            std::ptr::null_mut(),
            ffi::INTERNET_OPTION_SETTINGS_CHANGED,
            std::ptr::null_mut(),
            0,
        );
        ffi::InternetSetOptionW(
            std::ptr::null_mut(),
            ffi::INTERNET_OPTION_REFRESH,
            std::ptr::null_mut(),
            0,
        );
    }
}
