use oni::memory::{system_memory, estimate_model_memory};

#[test]
fn test_system_memory_returns_nonzero() {
    let report = system_memory();
    assert!(report.total > 0, "total memory should be > 0");
    assert!(report.available > 0, "available memory should be > 0");
    assert!(report.available <= report.total, "available <= total");
}

#[test]
fn test_estimate_model_memory_from_file_size() {
    // 10 GB file × 1.3 multiplier = 13 GB estimated
    let est = estimate_model_memory(10 * 1024 * 1024 * 1024, 1.3);
    assert_eq!(est, 13 * 1024 * 1024 * 1024);
}

#[test]
fn test_estimate_model_memory_default_multiplier() {
    let est = estimate_model_memory(10 * 1024 * 1024 * 1024, 0.0);
    // Should use default 1.3 when multiplier is 0
    assert_eq!(est, 13 * 1024 * 1024 * 1024);
}
