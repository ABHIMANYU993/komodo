use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use anyhow::Context;

#[derive(Debug, Clone, Default)]
pub struct InterfaceRawCounters {
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errs: u64,
    pub rx_drop: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errs: u64,
    pub tx_drop: u64,
}

#[derive(Debug, Clone)]
pub struct InterfaceRate {
    pub name: String,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub rx_packets_per_sec: f64,
    pub tx_packets_per_sec: f64,
}

pub struct NetworkCollector {
    prev_snapshot: HashMap<String, InterfaceRawCounters>,
    prev_time: Option<Instant>,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            prev_snapshot: HashMap::new(),
            prev_time: None,
        }
    }

    /// Read and parse all network interfaces dynamically from `/proc/net/dev`.
    pub fn read_dev() -> anyhow::Result<HashMap<String, InterfaceRawCounters>> {
        let file = File::open("/proc/net/dev").context("Failed to open /proc/net/dev")?;
        let reader = BufReader::new(file);

        let mut map = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            if !line.contains(':') {
                continue; // Skip headers
            }

            let mut split = line.splitn(2, ':');
            let iface_name = match split.next() {
                Some(name) => name.trim().to_string(),
                None => continue,
            };

            // Skip loopback interface from external stats
            if iface_name == "lo" {
                continue;
            }

            let rest = match split.next() {
                Some(r) => r,
                None => continue,
            };

            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 16 {
                continue;
            }

            let counters = InterfaceRawCounters {
                rx_bytes: parts[0].parse().unwrap_or(0),
                rx_packets: parts[1].parse().unwrap_or(0),
                rx_errs: parts[2].parse().unwrap_or(0),
                rx_drop: parts[3].parse().unwrap_or(0),
                tx_bytes: parts[8].parse().unwrap_or(0),
                tx_packets: parts[9].parse().unwrap_or(0),
                tx_errs: parts[10].parse().unwrap_or(0),
                tx_drop: parts[11].parse().unwrap_or(0),
            };

            map.insert(iface_name, counters);
        }

        Ok(map)
    }

    /// Collect delta-based bandwidth rates across successive calls.
    pub fn collect(&mut self) -> anyhow::Result<Vec<InterfaceRate>> {
        let now = Instant::now();
        let current = Self::read_dev()?;

        let mut rates = Vec::new();

        if let Some(prev_time) = self.prev_time {
            let elapsed_secs = now.duration_since(prev_time).as_secs_f64();
            if elapsed_secs > 0.0 {
                for (name, curr_counters) in &current {
                    if let Some(prev_counters) = self.prev_snapshot.get(name) {
                        let rx_delta = curr_counters.rx_bytes.saturating_sub(prev_counters.rx_bytes);
                        let tx_delta = curr_counters.tx_bytes.saturating_sub(prev_counters.tx_bytes);
                        let rx_pkts = curr_counters.rx_packets.saturating_sub(prev_counters.rx_packets);
                        let tx_pkts = curr_counters.tx_packets.saturating_sub(prev_counters.tx_packets);

                        rates.push(InterfaceRate {
                            name: name.clone(),
                            rx_bytes_per_sec: rx_delta as f64 / elapsed_secs,
                            tx_bytes_per_sec: tx_delta as f64 / elapsed_secs,
                            rx_packets_per_sec: rx_pkts as f64 / elapsed_secs,
                            tx_packets_per_sec: tx_pkts as f64 / elapsed_secs,
                        });
                    } else {
                        // Newly appeared interface
                        rates.push(InterfaceRate {
                            name: name.clone(),
                            rx_bytes_per_sec: 0.0,
                            tx_bytes_per_sec: 0.0,
                            rx_packets_per_sec: 0.0,
                            tx_packets_per_sec: 0.0,
                        });
                    }
                }
            }
        }

        self.prev_snapshot = current;
        self.prev_time = Some(now);

        Ok(rates)
    }
}
