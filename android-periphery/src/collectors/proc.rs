use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::time::Instant;
use crate::protocol::types::SystemProcess;

#[derive(Clone)]
struct ProcessMeta {
    name: String,
    exe: String,
    cmd: Vec<String>,
    start_time: f64,
}

pub struct ProcessCollector {
    prev_cpu_times: HashMap<u32, u64>,
    meta_cache: HashMap<u32, ProcessMeta>,
    prev_time: Option<Instant>,
    clock_ticks_per_sec: f64,
    page_size_kb: f64,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        let ticks = if clock_ticks > 0 { clock_ticks as f64 } else { 100.0 };

        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        let page_kb = if page_size > 0 { (page_size as f64) / 1024.0 } else { 4.0 };

        Self {
            prev_cpu_times: HashMap::new(),
            meta_cache: HashMap::new(),
            prev_time: None,
            clock_ticks_per_sec: ticks,
            page_size_kb: page_kb,
        }
    }

    /// Enumerate all processes dynamically from `/proc/[pid]/`.
    /// Caches static metadata (exe, cmd, start_time) per PID to minimize system calls and CPU load.
    pub fn collect(&mut self) -> Vec<SystemProcess> {
        let now = Instant::now();
        let elapsed_secs = self.prev_time.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(1.0);

        let mut current_cpu_times = HashMap::new();
        let mut current_pids = HashSet::new();
        let mut processes = Vec::new();

        let Ok(entries) = fs::read_dir("/proc") else {
            return processes;
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            let Ok(pid) = name_str.parse::<u32>() else {
                continue; // Not a numeric PID directory
            };

            current_pids.insert(pid);
            let proc_path = entry.path();

            // Read /proc/[pid]/stat
            let stat_path = proc_path.join("stat");
            let Ok(stat_content) = fs::read_to_string(&stat_path) else {
                continue; // Process disappeared (ENOENT) - normal race condition
            };

            // /proc/[pid]/stat format:
            // pid (comm) state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime ... rss ...
            let Some(open_paren) = stat_content.find('(') else { continue };
            let Some(close_paren) = stat_content.rfind(')') else { continue };

            let comm = stat_content[open_paren + 1..close_paren].to_string();
            let rest = &stat_content[close_paren + 2..];
            let fields: Vec<&str> = rest.split_whitespace().collect();

            if fields.len() < 22 {
                continue;
            }

            // utime is field 11 (0-indexed after comm), stime is field 12
            let utime: u64 = fields[11].parse().unwrap_or(0);
            let stime: u64 = fields[12].parse().unwrap_or(0);
            let total_proc_ticks = utime + stime;

            current_cpu_times.insert(pid, total_proc_ticks);

            // Calculate CPU percentage (core percentage)
            let cpu_perc = if let Some(&prev_ticks) = self.prev_cpu_times.get(&pid) {
                let tick_delta = total_proc_ticks.saturating_sub(prev_ticks) as f64;
                if elapsed_secs > 0.0 && self.clock_ticks_per_sec > 0.0 {
                    let cpu_time_secs = tick_delta / self.clock_ticks_per_sec;
                    ((cpu_time_secs / elapsed_secs) * 100.0) as f32
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // RSS is field 21 (in pages)
            let rss_pages: f64 = fields[21].parse().unwrap_or(0.0);
            let mem_mb = (rss_pages * self.page_size_kb) / 1024.0;

            // Reuse cached metadata (cmdline, exe, start_time) to avoid re-reading 1000+ files per second
            let meta = if let Some(cached) = self.meta_cache.get(&pid) {
                cached.clone()
            } else {
                // Start time is field 19 (in clock ticks since boot)
                let start_ticks: f64 = fields[19].parse().unwrap_or(0.0);
                let start_time_secs = start_ticks / self.clock_ticks_per_sec;

                // Read /proc/[pid]/cmdline (NUL-separated)
                let cmd_path = proc_path.join("cmdline");
                let cmd = if let Ok(mut f) = File::open(&cmd_path) {
                    let mut buf = Vec::new();
                    let _ = f.read_to_end(&mut buf);
                    buf.split(|&b| b == 0)
                        .filter(|s| !s.is_empty())
                        .map(|s| String::from_utf8_lossy(s).to_string())
                        .collect()
                } else {
                    vec![comm.clone()]
                };

                // Read /proc/[pid]/exe symlink
                let exe_path = proc_path.join("exe");
                let exe = fs::read_link(&exe_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let new_meta = ProcessMeta {
                    name: comm,
                    exe,
                    cmd,
                    start_time: start_time_secs,
                };
                self.meta_cache.insert(pid, new_meta.clone());
                new_meta
            };

            processes.push(SystemProcess {
                pid,
                name: meta.name,
                exe: meta.exe,
                cmd: meta.cmd,
                start_time: meta.start_time,
                cpu_perc,
                mem_mb,
                disk_read_kb: 0.0,
                disk_write_kb: 0.0,
            });
        }

        self.prev_cpu_times = current_cpu_times;
        self.meta_cache.retain(|pid, _| current_pids.contains(pid));
        self.prev_time = Some(now);

        processes
    }

    /// Terminate process safely via signal (SIGTERM = 15, SIGKILL = 9).
    pub fn kill_process(pid: u32, signal: i32) -> anyhow::Result<()> {
        let res = unsafe { libc::kill(pid as libc::pid_t, signal) };
        if res == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to send signal {signal} to PID {pid}: errno {}", std::io::Error::last_os_error()))
        }
    }
}
