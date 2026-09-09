use std::fs;
use std::path::Path;
use crate::config::{AGENT_VERSION, PROTOCOL_COMPATIBILITY_VERSION, UPSTREAM_COMPATIBILITY_COMMIT};
use crate::protocol::types::SystemInformation;

#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub agent_version: String,
    pub protocol_version: String,
    pub upstream_commit: String,
    pub android_release: String,
    pub sdk_level: u32,
    pub kernel_release: String,
    pub architecture: String,
    pub manufacturer: String,
    pub model: String,
    pub board: String,
    pub hardware_platform: String,
    pub cpu_count: u32,
    pub selinux_state: String,
    pub magisk_version: Option<String>,
}

impl DeviceCapabilities {
    /// Perform the comprehensive startup discovery pass.
    pub fn discover() -> Self {
        let getprop = |key: &str| -> String {
            // Check Android system properties via /system/build.prop or default getprop
            Self::read_android_property(key).unwrap_or_default()
        };

        let android_release = getprop("ro.build.version.release");
        let sdk_str = getprop("ro.build.version.sdk");
        let sdk_level = sdk_str.parse().unwrap_or(0);
        let manufacturer = getprop("ro.product.manufacturer");
        let model = getprop("ro.product.model");
        let board = getprop("ro.product.board");
        let hardware = getprop("ro.hardware");

        let kernel_release = fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let arch = std::env::consts::ARCH.to_string();

        let cpu_count = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as u32 };

        let selinux_state = Self::check_selinux();
        let magisk_version = Self::check_magisk();

        Self {
            agent_version: AGENT_VERSION.to_string(),
            protocol_version: PROTOCOL_COMPATIBILITY_VERSION.to_string(),
            upstream_commit: UPSTREAM_COMPATIBILITY_COMMIT.to_string(),
            android_release,
            sdk_level,
            kernel_release,
            architecture: arch,
            manufacturer,
            model,
            board,
            hardware_platform: hardware,
            cpu_count: if cpu_count > 0 { cpu_count } else { 1 },
            selinux_state,
            magisk_version,
        }
    }

    /// Read Android system property using Bionic's __system_property_get on Android,
    /// or fallback to build.prop files on host/Linux.
    fn read_android_property(prop: &str) -> Option<String> {
        #[cfg(target_os = "android")]
        {
            unsafe extern "C" {
                fn __system_property_get(name: *const libc::c_char, value: *mut libc::c_char) -> libc::c_int;
            }


            let prop_c = std::ffi::CString::new(prop).ok()?;
            let mut val_buf = [0u8; 128]; // PROP_VALUE_MAX is 92 on older, 128 on modern
            let len = unsafe {
                __system_property_get(prop_c.as_ptr(), val_buf.as_mut_ptr() as *mut libc::c_char)
            };
            if len > 0 {
                let val_str = std::ffi::CStr::from_bytes_until_nul(&val_buf)
                    .ok()?
                    .to_str()
                    .ok()?;
                if !val_str.is_empty() {
                    return Some(val_str.to_string());
                }
            }
        }

        // Fallback for host simulation or missing property
        for prop_file in [
            "/system/build.prop",
            "/product/build.prop",
            "/vendor/build.prop",
            "/system_ext/build.prop",
        ] {
            if let Ok(content) = fs::read_to_string(prop_file) {
                for line in content.lines() {
                    if let Some(rest) = line.strip_prefix(prop) {
                        if let Some(val) = rest.strip_prefix('=') {
                            return Some(val.trim().to_string());
                        }
                    }
                }
            }
        }

        None
    }


    /// Read SELinux enforcement state.
    fn check_selinux() -> String {
        if let Ok(content) = fs::read_to_string("/sys/fs/selinux/enforce") {
            if content.trim() == "1" {
                "Enforcing".to_string()
            } else {
                "Permissive".to_string()
            }
        } else {
            "Disabled".to_string()
        }
    }

    /// Detect Magisk presence and version.
    fn check_magisk() -> Option<String> {
        for path in ["/data/adb/magisk", "/sbin/magisk", "/product/bin/magisk"] {
            if Path::new(path).exists() {
                return Some("Detected (Magisk Rooted)".to_string());
            }
        }
        None
    }

    /// Convert to Komodo's UI `SystemInformation` payload.
    pub fn to_system_info(&self) -> SystemInformation {
        let name_str = if !self.model.is_empty() {
            format!("{} {}", self.manufacturer, self.model)
        } else {
            "Android Node".to_string()
        };

        let os_str = if !self.android_release.is_empty() {
            format!("Android {} (API {})", self.android_release, self.sdk_level)
        } else {
            "Android (Linux)".to_string()
        };

        let cpu_brand = if !self.hardware_platform.is_empty() {
            format!("{} ({})", self.hardware_platform, self.board)
        } else {
            "ARM64 Heterogeneous CPU".to_string()
        };

        SystemInformation {
            name: Some(name_str),
            os: Some(os_str),
            kernel: Some(self.kernel_release.clone()),
            core_count: Some(self.cpu_count),
            logical_core_count: Some(self.cpu_count),
            host_name: Some(self.model.clone()),
            cpu_brand,
            cpu_arch: self.architecture.clone(),
        }
    }
}
