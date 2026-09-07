//! Lightweight hardware detection (Phase 4).
//! Identifies Apple Silicon vs Intel, arch, memory, Metal.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub arch: String,
    pub is_apple_silicon: bool,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub metal_supported: bool,
    pub disk_free_gb: Option<u32>,
}

impl HardwareInfo {
    pub fn probe() -> Self {
        let arch = std::env::consts::ARCH.to_string();
        let is_apple_silicon = arch == "aarch64";
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);
        let memory_gb = probe_memory_gb().unwrap_or(8);
        let metal_supported = is_apple_silicon; // Metal on all AS, best-effort
        let disk_free_gb = probe_disk_free_gb();
        Self {
            arch,
            is_apple_silicon,
            cpu_cores,
            memory_gb,
            metal_supported,
            disk_free_gb,
        }
    }

    pub fn backend(&self) -> String {
        if self.metal_supported {
            "metal".into()
        } else {
            "accelerate".into()
        }
    }
}

fn probe_memory_gb() -> Option<u32> {
    // Use sysctl hw.memsize (bytes)
    let output = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let bytes: u64 = s.trim().parse().ok()?;
    Some((bytes / (1024 * 1024 * 1024)) as u32)
}

fn probe_disk_free_gb() -> Option<u32> {
    // Use df for models dir
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/Library/Application Support/Supertype/models", home);
    let output = std::process::Command::new("df")
        .args(["-g", &path])
        .output()
        .ok()?;
    if !output.status.success() {
        // fallback to root
        let output2 = std::process::Command::new("df")
            .args(["-g", "/"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&output2.stdout);
        // df -g output: ...  ... Available
        for line in s.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Ok(v) = parts[3].parse::<u32>() {
                    return Some(v);
                }
            }
        }
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    for line in s.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(v) = parts[3].parse::<u32>() {
                return Some(v);
            }
        }
    }
    None
}

/// Recommendation based on hardware
pub fn recommend_model_ids(hw: &HardwareInfo) -> Vec<String> {
    if hw.memory_gb >= 16 && hw.is_apple_silicon {
        vec![
            "parakeet-tdt-0.6b".into(),
            "whisper-base".into(),
            "whisper-tiny-q4_0".into(),
        ]
    } else if hw.memory_gb >= 8 {
        vec!["whisper-tiny".into(), "whisper-base".into()]
    } else {
        vec!["whisper-tiny-q4_0".into(), "whisper-tiny".into()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_has_fields() {
        let hw = HardwareInfo::probe();
        assert!(!hw.arch.is_empty());
        assert!(hw.memory_gb >= 1);
        assert!(hw.cpu_cores >= 1);
    }

    #[test]
    fn recommend_tiny_for_low_mem() {
        let hw = HardwareInfo {
            arch: "aarch64".into(),
            is_apple_silicon: true,
            cpu_cores: 8,
            memory_gb: 6,
            metal_supported: true,
            disk_free_gb: Some(20),
        };
        let rec = recommend_model_ids(&hw);
        assert!(rec.contains(&"whisper-tiny-q4_0".to_string()));
    }

    #[test]
    fn recommend_parakeet_for_high_mem() {
        let hw = HardwareInfo {
            arch: "aarch64".into(),
            is_apple_silicon: true,
            cpu_cores: 10,
            memory_gb: 32,
            metal_supported: true,
            disk_free_gb: Some(100),
        };
        let rec = recommend_model_ids(&hw);
        assert!(rec[0] == "parakeet-tdt-0.6b");
    }
}
