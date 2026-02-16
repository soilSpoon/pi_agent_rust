#[path = "../src/hostcall_s3_fifo.rs"]
mod hostcall_s3_fifo;

use hostcall_s3_fifo::{
    S3FifoConfig, S3FifoDecisionKind, S3FifoFallbackReason, S3FifoPolicy, S3FifoTier,
};

#[test]
fn smoke_policy_admits_then_promotes() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig::default());
    let _cfg = policy.config();
    let first = policy.access("ext-smoke", "key-smoke".to_string());
    let second = policy.access("ext-smoke", "key-smoke".to_string());

    assert_eq!(first.kind, S3FifoDecisionKind::AdmitSmall);
    assert_eq!(second.kind, S3FifoDecisionKind::PromoteSmallToMain);
}

#[test]
fn fallback_clear_recovers_from_bypass_mode() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 4,
        small_capacity: 2,
        ghost_capacity: 4,
        max_entries_per_owner: 4,
        fallback_window: 3,
        min_ghost_hits_in_window: 3,
        max_budget_rejections_in_window: 3,
    });

    for idx in 0..3 {
        let _ = policy.access("ext-a", format!("cold-{idx}"));
    }

    assert!(
        policy.telemetry().fallback_reason.is_some(),
        "fallback should trigger after low-signal window"
    );

    let bypass = policy.access("ext-a", "while-fallback".to_string());
    assert_eq!(bypass.kind, S3FifoDecisionKind::FallbackBypass);

    policy.clear_fallback();
    let resumed = policy.access("ext-a", "after-clear".to_string());
    assert_ne!(resumed.kind, S3FifoDecisionKind::FallbackBypass);
}

#[test]
fn owner_budget_rejects_third_unique_live_key() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 8,
        small_capacity: 4,
        ghost_capacity: 8,
        max_entries_per_owner: 2,
        fallback_window: 32,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 32,
    });

    let d1 = policy.access("ext-budget", "k1".to_string());
    let d2 = policy.access("ext-budget", "k2".to_string());
    let d3 = policy.access("ext-budget", "k3".to_string());

    assert_eq!(d1.kind, S3FifoDecisionKind::AdmitSmall);
    assert_eq!(d2.kind, S3FifoDecisionKind::AdmitSmall);
    assert_eq!(d3.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert_eq!(policy.telemetry().budget_rejections_total, 1);
}

#[test]
fn ghost_hit_reentry_admits_from_ghost() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 4,
        small_capacity: 1,
        ghost_capacity: 4,
        max_entries_per_owner: 4,
        fallback_window: 8,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 8,
    });

    let first = policy.access("ext-a", "k1".to_string());
    assert_eq!(first.kind, S3FifoDecisionKind::AdmitSmall);

    let _ = policy.access("ext-b", "k2".to_string());
    let reentry = policy.access("ext-a", "k1".to_string());

    assert_eq!(reentry.kind, S3FifoDecisionKind::AdmitFromGhost);
    assert!(reentry.ghost_hit, "reentry should come from ghost history");
    assert!(
        policy.telemetry().ghost_hits_total >= 1,
        "ghost hit counter should increment on reentry"
    );
}

#[test]
fn telemetry_snapshot_invariants_hold_after_mixed_sequence() {
    let config = S3FifoConfig {
        live_capacity: 6,
        small_capacity: 2,
        ghost_capacity: 8,
        max_entries_per_owner: 3,
        fallback_window: 32,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 32,
    };
    let mut policy = S3FifoPolicy::new(config);

    let sequence = [
        ("ext-a", "k1"),
        ("ext-a", "k1"),
        ("ext-b", "k2"),
        ("ext-c", "k3"),
        ("ext-b", "k4"),
        ("ext-a", "k5"),
        ("ext-c", "k3"),
    ];
    for (owner, key) in sequence {
        let _ = policy.access(owner, key.to_string());
    }

    let telemetry = policy.telemetry();
    let owner_sum: usize = telemetry.owner_live_counts.values().copied().sum();

    assert_eq!(
        telemetry.live_depth,
        telemetry.small_depth + telemetry.main_depth
    );
    assert_eq!(owner_sum, telemetry.live_depth);
    assert!(telemetry.live_depth <= config.live_capacity);
    assert!(telemetry.fallback_reason.is_none());
}

#[test]
fn fallback_bypass_emits_conservative_reason_and_tier_markers() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 4,
        small_capacity: 2,
        ghost_capacity: 4,
        max_entries_per_owner: 4,
        fallback_window: 3,
        min_ghost_hits_in_window: 3,
        max_budget_rejections_in_window: 3,
    });

    for idx in 0..3 {
        let _ = policy.access("ext-a", format!("cold-{idx}"));
    }

    let telemetry_before = policy.telemetry();
    assert_eq!(
        telemetry_before.fallback_reason,
        Some(S3FifoFallbackReason::SignalQualityInsufficient)
    );

    let bypass = policy.access("ext-a", "while-fallback".to_string());
    let telemetry_after = policy.telemetry();
    assert_eq!(bypass.kind, S3FifoDecisionKind::FallbackBypass);
    assert_eq!(bypass.tier, S3FifoTier::Fallback);
    assert_eq!(bypass.fallback_reason, telemetry_after.fallback_reason);
    assert!(!bypass.ghost_hit);
    assert_eq!(telemetry_after.live_depth, telemetry_before.live_depth);
    assert_eq!(telemetry_after.ghost_depth, telemetry_before.ghost_depth);
}

#[test]
fn identical_sequences_yield_identical_decision_and_telemetry_traces() {
    let config = S3FifoConfig {
        live_capacity: 5,
        small_capacity: 2,
        ghost_capacity: 6,
        max_entries_per_owner: 2,
        fallback_window: 8,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 8,
    };
    let mut policy_a = S3FifoPolicy::new(config);
    let mut policy_b = S3FifoPolicy::new(config);

    let sequence = [
        ("ext-a", "k1"),
        ("ext-b", "k2"),
        ("ext-a", "k1"),
        ("ext-c", "k3"),
        ("ext-b", "k4"),
        ("ext-a", "k5"),
        ("ext-c", "k3"),
        ("ext-b", "k2"),
    ];

    let mut decisions_a = Vec::new();
    let mut decisions_b = Vec::new();
    let mut telemetry_a = Vec::new();
    let mut telemetry_b = Vec::new();

    for (owner, key) in sequence {
        decisions_a.push(policy_a.access(owner, key.to_string()));
        telemetry_a.push(policy_a.telemetry());
    }
    for (owner, key) in sequence {
        decisions_b.push(policy_b.access(owner, key.to_string()));
        telemetry_b.push(policy_b.telemetry());
    }

    assert_eq!(decisions_a, decisions_b);
    assert_eq!(telemetry_a, telemetry_b);
}

#[test]
fn fallback_fairness_window_uses_strictly_greater_budget_threshold() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 8,
        small_capacity: 4,
        ghost_capacity: 8,
        max_entries_per_owner: 1,
        fallback_window: 3,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 2,
    });

    let d1 = policy.access("ext-a", "k1".to_string());
    let d2 = policy.access("ext-a", "k2".to_string());
    let d3 = policy.access("ext-a", "k3".to_string());

    assert_eq!(d1.kind, S3FifoDecisionKind::AdmitSmall);
    assert_eq!(d2.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert_eq!(d3.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert_eq!(
        policy.telemetry().fallback_reason,
        None,
        "fallback must not trigger when rejections equal the configured threshold"
    );

    let d4 = policy.access("ext-a", "k4".to_string());
    assert_eq!(d4.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert_eq!(
        policy.telemetry().fallback_reason,
        Some(S3FifoFallbackReason::FairnessInstability)
    );

    let d5 = policy.access("ext-a", "k5".to_string());
    assert_eq!(d5.kind, S3FifoDecisionKind::FallbackBypass);
    assert_eq!(d5.tier, S3FifoTier::Fallback);
    assert_eq!(
        d5.fallback_reason,
        Some(S3FifoFallbackReason::FairnessInstability)
    );
}

#[test]
fn fallback_reason_prefers_signal_quality_when_ghost_hits_are_insufficient() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 8,
        small_capacity: 4,
        ghost_capacity: 8,
        max_entries_per_owner: 1,
        fallback_window: 3,
        min_ghost_hits_in_window: 3,
        max_budget_rejections_in_window: 0,
    });

    let _ = policy.access("ext-a", "k1".to_string());
    let _ = policy.access("ext-a", "k2".to_string());
    let _ = policy.access("ext-a", "k3".to_string());

    assert_eq!(
        policy.telemetry().fallback_reason,
        Some(S3FifoFallbackReason::SignalQualityInsufficient),
        "low ghost-hit signal should take precedence over fairness-instability classification"
    );
}

#[test]
fn clear_fallback_resets_window_and_delays_fairness_retrigger() {
    let mut policy = S3FifoPolicy::new(S3FifoConfig {
        live_capacity: 8,
        small_capacity: 4,
        ghost_capacity: 8,
        max_entries_per_owner: 1,
        fallback_window: 3,
        min_ghost_hits_in_window: 0,
        max_budget_rejections_in_window: 1,
    });

    let _ = policy.access("ext-a", "k1".to_string());
    let _ = policy.access("ext-a", "k2".to_string());
    let _ = policy.access("ext-a", "k3".to_string());
    assert_eq!(
        policy.telemetry().fallback_reason,
        Some(S3FifoFallbackReason::FairnessInstability)
    );

    policy.clear_fallback();
    assert!(policy.telemetry().fallback_reason.is_none());

    let post_clear_1 = policy.access("ext-a", "k4".to_string());
    assert_eq!(post_clear_1.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert!(
        policy.telemetry().fallback_reason.is_none(),
        "one rejection after clear cannot retrigger before the fallback window refills"
    );

    let post_clear_2 = policy.access("ext-a", "k5".to_string());
    assert_eq!(post_clear_2.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert!(policy.telemetry().fallback_reason.is_none());

    let post_clear_3 = policy.access("ext-a", "k6".to_string());
    assert_eq!(post_clear_3.kind, S3FifoDecisionKind::RejectFairnessBudget);
    assert_eq!(
        policy.telemetry().fallback_reason,
        Some(S3FifoFallbackReason::FairnessInstability)
    );
}
