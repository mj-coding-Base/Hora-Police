use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SystemEnvironment {
    pub vcpu_count: usize,
    pub total_ram_mb: u64,
    pub has_ebpf: bool,
    pub has_cgroups_v2: bool,
    pub load_average: (f64, f64, f64),
}

impl SystemEnvironment {
    pub fn detect() -> Result<Self> {
        let vcpu_count = Self::detect_vcpu_count()?;
        let total_ram_mb = Self::detect_ram()?;
        let has_ebpf = Self::check_ebpf();
        let has_cgroups_v2 = Self::check_cgroups_v2();
        let load_average = Self::read_load_average()?;

        Ok(Self {
            vcpu_count,
            total_ram_mb,
            has_ebpf,
            has_cgroups_v2,
            load_average,
        })
    }

    fn detect_vcpu_count() -> Result<usize> {
        // Try nproc first
        if let Ok(output) = std::process::Command::new("nproc")
            .output()
        {
            if let Ok(count_str) = String::from_utf8(output.stdout) {
                if let Ok(count) = count_str.trim().parse::<usize>() {
                    return Ok(count);
                }
            }
        }

        // Fallback to /proc/cpuinfo
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")
            .context("Failed to read /proc/cpuinfo")?;
        
        let count = cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count();

        if count > 0 {
            Ok(count)
        } else {
            // Last resort: count physical CPUs
            Ok(1)
        }
    }

    fn detect_ram() -> Result<u64> {
        let meminfo = fs::read_to_string("/proc/meminfo")
            .context("Failed to read /proc/meminfo")?;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Ok(kb / 1024); // Convert KB to MB
                    }
                }
            }
        }

        Ok(1024) // Default to 1GB if detection fails
    }

    fn check_ebpf() -> bool {
        // Check if /sys/fs/bpf exists and is mountable
        Path::new("/sys/fs/bpf").exists() || 
        // Or check kernel version >= 4.1
        Self::kernel_version_check(4, 1)
    }

    fn check_cgroups_v2() -> bool {
        Path::new("/sys/fs/cgroup/cgroup.controllers").exists()
    }

    fn kernel_version_check(major: u32, minor: u32) -> bool {
        if let Ok(release) = fs::read_to_string("/proc/sys/kernel/osrelease") {
            let parts: Vec<&str> = release.trim().split('.').collect();
            if parts.len() >= 2 {
                if let (Ok(maj), Ok(min)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return maj > major || (maj == major && min >= minor);
                }
            }
        }
        false
    }

    fn read_load_average() -> Result<(f64, f64, f64)> {
        let loadavg = fs::read_to_string("/proc/loadavg")
            .context("Failed to read /proc/loadavg")?;

        let parts: Vec<&str> = loadavg.split_whitespace().collect();
        if parts.len() >= 3 {
            let load1 = parts[0].parse::<f64>()
                .context("Failed to parse load average 1min")?;
            let load5 = parts[1].parse::<f64>()
                .context("Failed to parse load average 5min")?;
            let load15 = parts[2].parse::<f64>()
                .context("Failed to parse load average 15min")?;
            return Ok((load1, load5, load15));
        }

        Ok((0.0, 0.0, 0.0))
    }

    /// Compute auto-tuned CPU threshold based on vCPU count
    /// Formula: max(20.0, 8.0 * (1.0 / vcpu_count) * 100)
    /// This represents a significant portion of one core
    pub fn compute_cpu_threshold(&self, base_threshold: f32, vcpu_override: Option<usize>) -> f32 {
        let vcpu = vcpu_override.unwrap_or(self.vcpu_count);
        
        if vcpu == 0 {
            return base_threshold;
        }

        // Calculate per-vCPU sensitivity
        // On 4 vCPU: 8.0 * (1.0 / 4) * 100 = 200% of one core = 50% system
        // But we want to be conservative, so use: 25% of one core = 6.25% system on 4 vCPU
        let per_core_threshold = 25.0; // 25% of one core
        let system_threshold = (per_core_threshold / vcpu as f32).max(5.0); // Minimum 5%
        
        // Use the higher of base threshold or auto-tuned threshold
        base_threshold.max(system_threshold)
    }

    /// Scale duration with load average
    /// Increase duration if system is under heavy load
    pub fn compute_duration_minutes(&self, base_duration: u64) -> u64 {
        let (load1, _, _) = self.load_average;
        let load_threshold = self.vcpu_count as f64 * 1.5;

        if load1 > load_threshold {
            // Increase duration by 50% if under heavy load
            ((base_duration as f64 * 1.5) as u64).max(base_duration)
        } else {
            base_duration
        }
    }

    /// Determine if sampling interval should be increased
    pub fn should_adapt_sampling(&self) -> bool {
        let (load1, _, _) = self.load_average;
        let load_threshold = self.vcpu_count as f64 * 2.0;
        load1 > load_threshold
    }

    /// Get recommended polling interval based on load
    pub fn compute_polling_interval_ms(&self, base_interval_ms: u64) -> u64 {
        if self.should_adapt_sampling() {
            // Increase by 50% if under heavy load
            ((base_interval_ms as f64 * 1.5) as u64).max(base_interval_ms)
        } else {
            base_interval_ms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_threshold_calculation() {
        let env = SystemEnvironment {
            vcpu_count: 4,
            total_ram_mb: 8192,
            has_ebpf: false,
            has_cgroups_v2: false,
            load_average: (1.0, 1.0, 1.0),
        };

        let threshold = env.compute_cpu_threshold(20.0, None);
        // On 4 vCPU: 25% / 4 = 6.25%, but we use max(20.0, 6.25) = 20.0
        assert!(threshold >= 5.0);
    }
}

