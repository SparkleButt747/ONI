use oni::server_manager::EvictionPlan;
use oni_core::types::ModelTier;

#[test]
fn test_eviction_selects_lru_first() {
    let instances = vec![
        (ModelTier::General, 60_000_000_000u64, 5u64), // used 5s ago
        (ModelTier::Fast, 8_000_000_000u64, 1u64),     // used 1s ago
    ];
    let plan = EvictionPlan::select(&instances, 50_000_000_000, ModelTier::Heavy);
    assert_eq!(plan.tiers_to_evict.len(), 1);
    assert_eq!(plan.tiers_to_evict[0], ModelTier::General);
    assert!(plan.will_free >= 50_000_000_000);
}

#[test]
fn test_eviction_never_evicts_target_tier() {
    let instances = vec![(ModelTier::Heavy, 20_000_000_000u64, 10u64)];
    let plan = EvictionPlan::select(&instances, 20_000_000_000, ModelTier::Heavy);
    assert!(plan.tiers_to_evict.is_empty());
}

#[test]
fn test_eviction_skips_embed_tier() {
    let instances = vec![
        (ModelTier::Embed, 300_000_000u64, 100u64),
        (ModelTier::Fast, 8_000_000_000u64, 1u64),
    ];
    let plan = EvictionPlan::select(&instances, 8_000_000_000, ModelTier::Heavy);
    assert_eq!(plan.tiers_to_evict.len(), 1);
    assert_eq!(plan.tiers_to_evict[0], ModelTier::Fast);
}

#[test]
fn test_eviction_multiple_tiers_to_free_enough() {
    let instances = vec![
        (ModelTier::General, 10_000_000_000u64, 10u64), // oldest
        (ModelTier::Fast, 8_000_000_000u64, 5u64),      // middle
        (ModelTier::Medium, 20_000_000_000u64, 1u64),   // newest
    ];
    // Need 15GB — should evict General (10GB, oldest) then Fast (8GB)
    let plan = EvictionPlan::select(&instances, 15_000_000_000, ModelTier::Heavy);
    assert_eq!(plan.tiers_to_evict.len(), 2);
    assert_eq!(plan.tiers_to_evict[0], ModelTier::General);
    assert_eq!(plan.tiers_to_evict[1], ModelTier::Fast);
    assert!(plan.will_free >= 15_000_000_000);
}

#[test]
fn test_eviction_empty_instances() {
    let instances: Vec<(ModelTier, u64, u64)> = vec![];
    let plan = EvictionPlan::select(&instances, 10_000_000_000, ModelTier::Heavy);
    assert!(plan.tiers_to_evict.is_empty());
    assert_eq!(plan.will_free, 0);
}
