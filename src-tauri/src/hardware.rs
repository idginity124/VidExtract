use serde::Serialize;
use sysinfo::System;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub dedicated_memory: Option<u64>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub total_memory: u64,
    pub available_memory: u64,
    pub gpus: Vec<GpuInfo>,
    pub tier: String,
    pub recommended_model: String,
    pub threads: usize,
    pub gpu_acceleration: bool,
}
const GIB: u64 = 1024 * 1024 * 1024;
fn policy(total: u64, available: u64, cores: usize) -> (&'static str, &'static str, usize) {
    if total < 6 * GIB || available < 2 * GIB || cores <= 2 {
        ("efficient", "tiny", 1)
    } else if total < 12 * GIB || available < 5 * GIB || cores < 6 {
        ("balanced", "base", cores.saturating_sub(1).clamp(1, 3))
    } else {
        ("quality", "small", (cores / 2).clamp(2, 8))
    }
}
fn snapshot() -> DeviceProfile {
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_list(sysinfo::CpuRefreshKind::everything());
    let physical = System::physical_core_count().unwrap_or_else(|| system.cpus().len().max(1));
    let total = system.total_memory();
    let available = system.available_memory();
    let allowed_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(physical)
        .min(physical)
        .max(1);
    let (tier, model, threads) = policy(total, available, allowed_cores);
    DeviceProfile {
        cpu_name: system
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        physical_cores: physical,
        logical_cores: system.cpus().len(),
        total_memory: total,
        available_memory: available,
        gpus: Vec::new(),
        tier: tier.into(),
        recommended_model: model.into(),
        threads,
        gpu_acceleration: false,
    }
}
pub async fn memory_profile() -> Result<DeviceProfile, String> {
    tokio::task::spawn_blocking(snapshot)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn inspect_device() -> Result<DeviceProfile, String> {
    let mut profile = memory_profile().await?;
    #[cfg(target_os = "linux")]
    {
        let mut cmd = crate::process::command(std::path::Path::new("lspci"));
        cmd.args(["-D", "-mm"]);
        let pci = crate::process::checked(cmd, 8).await.unwrap_or_default();
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name
                    .strip_prefix("card")
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                {
                    continue;
                }
                let device = entry.path().join("device");
                let pci_id = std::fs::canonicalize(&device)
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
                let line = pci_id
                    .as_deref()
                    .and_then(|id| pci.lines().find(|line| line.starts_with(id)));
                let label = line
                    .map(|line| {
                        line.split('"')
                            .skip(1)
                            .step_by(2)
                            .skip(1)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("GPU {name}"));
                let vram = std::fs::read_to_string(device.join("mem_info_vram_total"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .filter(|n| *n > 0);
                profile.gpus.push(GpuInfo {
                    name: label,
                    dedicated_memory: vram,
                });
            }
        }
    }
    #[cfg(windows)]
    {
        profile.gpus = tokio::task::spawn_blocking(|| {
            use windows::Win32::Graphics::Dxgi::{
                CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
            };
            let mut gpus = Vec::new();
            // DXGI reports 64-bit dedicated memory; WMI AdapterRAM truncates GPUs above 4 GB.
            unsafe {
                if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
                    let mut i = 0;
                    while let Ok(adapter) = factory.EnumAdapters1(i) {
                        i += 1;
                        if let Ok(desc) = adapter.GetDesc1() {
                            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                                continue;
                            }
                            let length = desc
                                .Description
                                .iter()
                                .position(|c| *c == 0)
                                .unwrap_or(desc.Description.len());
                            gpus.push(GpuInfo {
                                name: String::from_utf16_lossy(&desc.Description[..length]),
                                dedicated_memory: Some(desc.DedicatedVideoMemory as u64)
                                    .filter(|n| *n > 0),
                            });
                        }
                    }
                }
            }
            gpus
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(profile)
}
pub fn minimum_memory(model: &str) -> u64 {
    match model {
        "tiny" => 700 * 1024 * 1024,
        "base" => 1100 * 1024 * 1024,
        _ => 2200 * 1024 * 1024,
    }
}
/// Ask the bundled inference backend, rather than inferring Vulkan support from a GPU name.
pub async fn vulkan_available(app: &tauri::AppHandle) -> bool {
    let executable = crate::tool(app, "whisper-cli-vulkan");
    if !executable.is_file() {
        return false;
    }
    let mut command = crate::process::command(&executable);
    command.arg("--vidextract-probe");
    crate::process::checked(command, 15)
        .await
        .is_ok_and(|output| {
            output
                .lines()
                .any(|line| line.starts_with("VIDEXTRACT_GPU:"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_recommendations_use_available_memory_and_cpu() {
        assert_eq!(policy(4 * GIB, 2 * GIB, 4), ("efficient", "tiny", 1));
        assert_eq!(policy(32 * GIB, GIB, 16), ("efficient", "tiny", 1));
        assert_eq!(policy(8 * GIB, 4 * GIB, 4), ("balanced", "base", 3));
        assert_eq!(policy(16 * GIB, 8 * GIB, 8), ("quality", "small", 4));
        assert_eq!(policy(64 * GIB, 40 * GIB, 64).2, 8);
    }
}
