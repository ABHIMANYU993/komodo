use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use crate::protocol::types::SingleDiskUsage;

pub struct StorageCollector;

impl StorageCollector {
    /// Discovers real, non-virtual storage mount points and queries filesystem capacity via `statvfs`.
    pub fn collect() -> Vec<SingleDiskUsage> {
        let mut results = Vec::new();
        let mounts = Self::read_mounts();

        // Track visited device names/paths to avoid reporting identical underlying partitions twice (e.g. /data and /storage/emulated/0)
        let mut seen_targets = std::collections::HashSet::new();

        for (_device, mount_point, fs_type) in mounts {
            // Filter out virtual pseudo-filesystems
            if fs_type == "sysfs"
                || fs_type == "proc"
                || fs_type == "devpts"
                || fs_type == "tmpfs"
                || fs_type == "cgroup"
                || fs_type == "cgroup2"
                || fs_type == "pstore"
                || fs_type == "selinuxfs"
                || fs_type == "tracefs"
                || fs_type == "debugfs"
                || fs_type == "bpf"
                || fs_type == "fusectl"
                || fs_type == "overlay"
                || fs_type == "functionfs"
            {
                continue;
            }

            // Only consider meaningful Android mount points (/data, /, /system, /vendor, /product, /mnt/media_rw/*)
            if !mount_point.starts_with("/data")
                && mount_point != "/"
                && !mount_point.starts_with("/system")
                && !mount_point.starts_with("/mnt/media_rw")
            {
                continue;
            }

            // Avoid reporting /storage/emulated/0 when /data is already reported
            if mount_point.starts_with("/storage/emulated") {
                continue;
            }

            if seen_targets.contains(&mount_point) {
                continue;
            }

            if let Ok(stat) = Self::statvfs(&mount_point) {
                if stat.total_gb > 0.05 {
                    // Ignore tiny mounts < 50MB
                    seen_targets.insert(mount_point.clone());
                    results.push(SingleDiskUsage {
                        mount: PathBuf::from(&mount_point),
                        file_system: fs_type,
                        used_gb: stat.used_gb,
                        total_gb: stat.total_gb,
                    });
                }
            }
        }

        // Fallback: If no mounts matched (e.g. non-standard Android OEM), query /data and /
        if results.is_empty() {
            for fallback_path in ["/data", "/"] {
                if let Ok(stat) = Self::statvfs(fallback_path) {
                    results.push(SingleDiskUsage {
                        mount: PathBuf::from(fallback_path),
                        file_system: "ext4/f2fs".to_string(),
                        used_gb: stat.used_gb,
                        total_gb: stat.total_gb,
                    });
                }
            }
        }

        results
    }

    /// Read `/proc/mounts`
    fn read_mounts() -> Vec<(String, String, String)> {
        let mut list = Vec::new();
        let Ok(file) = File::open("/proc/mounts") else {
            return list;
        };
        let reader = BufReader::new(file);

        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                list.push((
                    parts[0].to_string(), // device
                    parts[1].to_string(), // mount point
                    parts[2].to_string(), // filesystem type
                ));
            }
        }

        list
    }

    /// Query `statvfs` for a mount path.
    fn statvfs(path: &str) -> anyhow::Result<StatVfsResult> {
        let path_c = std::ffi::CString::new(path)?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };

        let res = unsafe { libc::statvfs(path_c.as_ptr(), &mut stat) };
        if res != 0 {
            return Err(anyhow::anyhow!("statvfs failed with code {res}"));
        }

        let block_size = stat.f_frsize as f64;
        let total_bytes = stat.f_blocks as f64 * block_size;
        let free_bytes = stat.f_bavail as f64 * block_size;
        let used_bytes = (total_bytes - free_bytes).max(0.0);

        let gb = 1024.0 * 1024.0 * 1024.0;

        Ok(StatVfsResult {
            total_gb: total_bytes / gb,
            used_gb: used_bytes / gb,
        })
    }
}

struct StatVfsResult {
    total_gb: f64,
    used_gb: f64,
}
