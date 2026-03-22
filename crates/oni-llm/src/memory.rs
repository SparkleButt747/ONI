use sysinfo::System;

/// Snapshot of system memory state.
pub struct MemoryReport {
    /// Total physical memory in bytes.
    pub total: u64,
    /// Available memory in bytes (free + reclaimable).
    pub available: u64,
}

/// Query current system memory.
pub fn system_memory() -> MemoryReport {
    let mut sys = System::new();
    sys.refresh_memory();
    MemoryReport {
        total: sys.total_memory(),
        available: sys.available_memory(),
    }
}

/// Estimate runtime memory for a model given its GGUF file size.
/// `multiplier` accounts for KV cache, activations, and Metal overhead.
/// If multiplier is <= 0, uses default 1.3.
pub fn estimate_model_memory(gguf_file_size: u64, multiplier: f64) -> u64 {
    let mult = if multiplier <= 0.0 { 1.3 } else { multiplier };
    (gguf_file_size as f64 * mult) as u64
}

/// Read the file size of a GGUF model file.
pub fn gguf_file_size(path: &std::path::Path) -> std::io::Result<u64> {
    std::fs::metadata(path).map(|m| m.len())
}
