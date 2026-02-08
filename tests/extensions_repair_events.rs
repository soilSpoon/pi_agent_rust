//! Unit tests for the auto-repair event logging infrastructure (bd-k5q5.8.1).
//!
//! Tests cover:
//! - `RepairPattern` display formatting
//! - `ExtensionRepairEvent` construction and cloning
//! - `PiJsRuntimeConfig` auto-repair flag
//! - `PiJsTickStats` default repair count
//! - `JsExtensionRuntimeHandle::drain_repair_events` (via channel)

#![allow(clippy::doc_markdown)]

mod common;

use pi::extensions::{ExtensionManager, JsExtensionLoadSpec, JsExtensionRuntimeHandle};
use pi::extensions_js::{
    ExtensionRepairEvent, MonotonicityVerdict, PatchOp, PatchProposal, PiJsRuntimeConfig,
    PiJsTickStats, RepairMode, RepairPattern, RepairRisk, REPAIR_REGISTRY_VERSION, REPAIR_RULES,
};
use pi::tools::ToolRegistry;
use std::sync::Arc;
use std::time::Duration;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_event(pattern: RepairPattern, success: bool) -> ExtensionRepairEvent {
    ExtensionRepairEvent {
        extension_id: "test-ext".to_string(),
        pattern,
        original_error: "module not found: ./dist/index.js".to_string(),
        repair_action: "resolved to ./src/index.ts".to_string(),
        success,
        timestamp_ms: 1_700_000_000_000,
    }
}

fn start_runtime(harness: &common::TestHarness) -> (ExtensionManager, JsExtensionRuntimeHandle) {
    let cwd = harness.temp_dir().to_path_buf();
    let manager = ExtensionManager::new();
    let tools = Arc::new(ToolRegistry::new(&[], &cwd, None));
    let config = PiJsRuntimeConfig {
        cwd: cwd.display().to_string(),
        ..Default::default()
    };

    let handle = common::run_async({
        let manager = manager.clone();
        async move {
            JsExtensionRuntimeHandle::start(config, tools, manager)
                .await
                .expect("start js runtime")
        }
    });
    manager.set_js_runtime(handle.clone());
    (manager, handle)
}

fn start_runtime_with_ext(
    harness: &common::TestHarness,
    source: &str,
) -> (ExtensionManager, JsExtensionRuntimeHandle) {
    let cwd = harness.temp_dir().to_path_buf();
    let ext_path = harness.create_file("extensions/ext.mjs", source.as_bytes());
    let spec = JsExtensionLoadSpec::from_entry_path(&ext_path).expect("load spec");

    let manager = ExtensionManager::new();
    let tools = Arc::new(ToolRegistry::new(&[], &cwd, None));
    let config = PiJsRuntimeConfig {
        cwd: cwd.display().to_string(),
        ..Default::default()
    };

    let handle = common::run_async({
        let manager = manager.clone();
        async move {
            JsExtensionRuntimeHandle::start(config, tools, manager)
                .await
                .expect("start js runtime")
        }
    });
    manager.set_js_runtime(handle.clone());

    common::run_async({
        let manager = manager.clone();
        async move {
            manager
                .load_js_extensions(vec![spec])
                .await
                .expect("load extension");
        }
    });

    (manager, handle)
}

fn shutdown(handle: &JsExtensionRuntimeHandle) {
    let _ = common::run_async({
        let h = handle.clone();
        async move { h.shutdown(Duration::from_millis(500)).await }
    });
}

// ─── RepairPattern display ──────────────────────────────────────────────────

#[test]
fn repair_pattern_display_dist_to_src() {
    assert_eq!(RepairPattern::DistToSrc.to_string(), "dist_to_src");
}

#[test]
fn repair_pattern_display_missing_asset() {
    assert_eq!(RepairPattern::MissingAsset.to_string(), "missing_asset");
}

#[test]
fn repair_pattern_display_monorepo_escape() {
    assert_eq!(RepairPattern::MonorepoEscape.to_string(), "monorepo_escape");
}

#[test]
fn repair_pattern_display_missing_npm_dep() {
    assert_eq!(RepairPattern::MissingNpmDep.to_string(), "missing_npm_dep");
}

#[test]
fn repair_pattern_display_export_shape() {
    assert_eq!(RepairPattern::ExportShape.to_string(), "export_shape");
}

// ─── RepairPattern equality and copy ────────────────────────────────────────

#[test]
fn repair_pattern_eq_and_copy() {
    let a = RepairPattern::DistToSrc;
    let b = a; // Copy
    assert_eq!(a, b);
    assert_ne!(RepairPattern::DistToSrc, RepairPattern::MissingAsset);
}

// ─── ExtensionRepairEvent construction ──────────────────────────────────────

#[test]
fn repair_event_fields_accessible() {
    let ev = make_event(RepairPattern::DistToSrc, true);
    assert_eq!(ev.extension_id, "test-ext");
    assert_eq!(ev.pattern, RepairPattern::DistToSrc);
    assert!(ev.success);
    assert_eq!(ev.timestamp_ms, 1_700_000_000_000);
}

#[test]
fn repair_event_clone() {
    let ev = make_event(RepairPattern::MissingAsset, false);
    let ev2 = ev.clone();
    assert_eq!(ev.extension_id, ev2.extension_id);
    assert_eq!(ev.pattern, ev2.pattern);
    assert_eq!(ev.success, ev2.success);
}

// ─── PiJsRuntimeConfig repair_mode ──────────────────────────────────────────

#[test]
fn config_repair_mode_defaults_to_auto_safe() {
    let config = PiJsRuntimeConfig::default();
    assert_eq!(config.repair_mode, pi::extensions_js::RepairMode::AutoSafe);
    assert!(config.auto_repair_enabled());
}

#[test]
fn config_repair_mode_off_disables_repair() {
    let config = PiJsRuntimeConfig {
        repair_mode: pi::extensions_js::RepairMode::Off,
        ..Default::default()
    };
    assert!(!config.auto_repair_enabled());
}

#[test]
fn config_repair_mode_suggest_does_not_apply() {
    let config = PiJsRuntimeConfig {
        repair_mode: pi::extensions_js::RepairMode::Suggest,
        ..Default::default()
    };
    assert!(!config.auto_repair_enabled());
    assert!(config.repair_mode.is_active());
}

#[test]
fn config_repair_mode_auto_strict_enables_aggressive() {
    let config = PiJsRuntimeConfig {
        repair_mode: pi::extensions_js::RepairMode::AutoStrict,
        ..Default::default()
    };
    assert!(config.auto_repair_enabled());
    assert!(config.repair_mode.allows_aggressive());
}

// ─── PiJsTickStats default ──────────────────────────────────────────────────

#[test]
fn tick_stats_default_has_zero_repairs() {
    let stats = PiJsTickStats::default();
    assert_eq!(stats.repairs_total, 0);
}

// ─── JsExtensionRuntimeHandle drain_repair_events ───────────────────────────

#[test]
fn handle_drain_repair_events_empty_on_fresh_runtime() {
    let harness = common::TestHarness::new("repair_drain_empty");
    let (_manager, handle) = start_runtime(&harness);

    let events = common::run_async({
        let h = handle.clone();
        async move { h.drain_repair_events().await }
    });
    assert!(events.is_empty());

    shutdown(&handle);
}

#[test]
fn handle_drain_repair_events_after_clean_extension_load() {
    let harness = common::TestHarness::new("repair_drain_clean");
    let (_manager, handle) = start_runtime_with_ext(
        &harness,
        r#"
        export default function activate(pi) {
            pi.registerTool({
                name: "noop",
                description: "does nothing",
                parameters: { type: "object", properties: {} },
                execute: async () => ({ content: [{ type: "text", text: "ok" }] }),
            });
        }
        "#,
    );

    // A well-behaved extension should produce zero repair events.
    let events = common::run_async({
        let h = handle.clone();
        async move { h.drain_repair_events().await }
    });
    assert!(
        events.is_empty(),
        "expected no repairs, got {}",
        events.len()
    );

    shutdown(&handle);
}

// ─── All patterns constructible ──────────────────────────────────────────────

#[test]
fn all_patterns_constructible() {
    let patterns = [
        RepairPattern::DistToSrc,
        RepairPattern::MissingAsset,
        RepairPattern::MonorepoEscape,
        RepairPattern::MissingNpmDep,
        RepairPattern::ExportShape,
        RepairPattern::ManifestNormalization,
        RepairPattern::ApiMigration,
    ];

    for (i, pattern) in patterns.iter().enumerate() {
        let ev = ExtensionRepairEvent {
            extension_id: format!("ext-{i}"),
            pattern: *pattern,
            original_error: "err".to_string(),
            repair_action: "fix".to_string(),
            success: true,
            timestamp_ms: 1_000 + i as u64,
        };
        assert_eq!(ev.extension_id, format!("ext-{i}"));
        assert_eq!(ev.pattern, patterns[i]);
    }
}

// ─── RepairPattern hash ─────────────────────────────────────────────────────

#[test]
fn repair_pattern_usable_as_hash_key() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(RepairPattern::DistToSrc);
    set.insert(RepairPattern::MissingAsset);
    set.insert(RepairPattern::DistToSrc); // duplicate
    assert_eq!(set.len(), 2);
}

// ─── Pattern 1: dist/ → src/ fallback (bd-k5q5.8.2) ────────────────────────

#[test]
fn dist_to_src_fallback_resolves_when_src_exists() {
    let harness = common::TestHarness::new("dist_to_src_resolve");

    // Create the extension entry that imports from ./dist/extension.js
    // (which doesn't exist), but ./src/extension.ts does.
    harness.create_file(
        "extensions/src/extension.ts",
        br#"
        export function hello() { return "from src"; }
        "#,
    );

    // The entry point re-exports from ./dist/extension.js (missing build output).
    let (_manager, handle) = start_runtime_with_ext(
        &harness,
        r#"
        import { hello } from "./src/extension.ts";
        export default function activate(pi) {
            pi.registerTool({
                name: "hello",
                description: "test",
                parameters: { type: "object", properties: {} },
                execute: async () => ({
                    content: [{ type: "text", text: hello() }],
                }),
            });
        }
        "#,
    );

    // Verify the extension loaded (it uses a direct src import, no repair needed).
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(tools.iter().any(|t| t.name == "hello"));

    shutdown(&handle);
}

#[test]
fn dist_to_src_fallback_loads_extension_with_dist_import() {
    let harness = common::TestHarness::new("dist_to_src_import");

    // Create src/lib.ts (the source file that dist/lib.js would have been).
    harness.create_file(
        "extensions/src/lib.ts",
        br#"
        export const greeting = "hello from src";
        "#,
    );

    // Entry point imports from ./dist/lib.js which doesn't exist.
    // The auto-repair should fall back to ./src/lib.ts.
    let (_manager, handle) = start_runtime_with_ext(
        &harness,
        r#"
        import { greeting } from "./dist/lib.js";
        export default function activate(pi) {
            pi.registerTool({
                name: "greet",
                description: "test",
                parameters: { type: "object", properties: {} },
                execute: async () => ({
                    content: [{ type: "text", text: greeting }],
                }),
            });
        }
        "#,
    );

    // Verify the extension loaded successfully via the fallback.
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(
        tools.iter().any(|t| t.name == "greet"),
        "extension should have loaded via dist→src fallback"
    );

    shutdown(&handle);
}

#[test]
fn dist_to_src_fallback_no_effect_when_dist_exists() {
    let harness = common::TestHarness::new("dist_to_src_no_fallback");

    // Create BOTH dist/lib.js and src/lib.ts.
    harness.create_file(
        "extensions/dist/lib.js",
        br#"
        export const greeting = "from dist";
        "#,
    );
    harness.create_file(
        "extensions/src/lib.ts",
        br#"
        export const greeting = "from src";
        "#,
    );

    // Entry point imports from ./dist/lib.js which DOES exist.
    let (_manager, handle) = start_runtime_with_ext(
        &harness,
        r#"
        import { greeting } from "./dist/lib.js";
        export default function activate(pi) {
            pi.registerTool({
                name: "greet",
                description: "test",
                parameters: { type: "object", properties: {} },
                execute: async () => ({
                    content: [{ type: "text", text: greeting }],
                }),
            });
        }
        "#,
    );

    // Should load from dist/ (no fallback needed).
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(tools.iter().any(|t| t.name == "greet"));

    shutdown(&handle);
}

// ─── Safety boundary: repair_mode gating (bd-k5q5.9.1.2) ────────────────────

/// Start a runtime with a specific `RepairMode`, attempt to load the extension,
/// and return the result (may be `Err` if the extension fails to load).
fn try_start_runtime_with_mode(
    harness: &common::TestHarness,
    source: &str,
    mode: RepairMode,
) -> (
    ExtensionManager,
    JsExtensionRuntimeHandle,
    Result<(), String>,
) {
    let cwd = harness.temp_dir().to_path_buf();
    let ext_path = harness.create_file("extensions/ext.mjs", source.as_bytes());
    let spec = JsExtensionLoadSpec::from_entry_path(&ext_path).expect("load spec");

    let manager = ExtensionManager::new();
    let tools = Arc::new(ToolRegistry::new(&[], &cwd, None));
    let config = PiJsRuntimeConfig {
        cwd: cwd.display().to_string(),
        repair_mode: mode,
        ..Default::default()
    };

    let handle = common::run_async({
        let manager = manager.clone();
        async move {
            JsExtensionRuntimeHandle::start(config, tools, manager)
                .await
                .expect("start js runtime")
        }
    });
    manager.set_js_runtime(handle.clone());

    let load_result = common::run_async({
        let manager = manager.clone();
        async move {
            manager
                .load_js_extensions(vec![spec])
                .await
                .map_err(|e| e.to_string())
        }
    });

    (manager, handle, load_result)
}

/// Source code that imports from `./dist/lib.js` (which won't exist).
const DIST_IMPORT_SOURCE: &str = r#"
    import { greeting } from "./dist/lib.js";
    export default function activate(pi) {
        pi.registerTool({
            name: "greet",
            description: "test",
            parameters: { type: "object", properties: {} },
            execute: async () => ({
                content: [{ type: "text", text: greeting }],
            }),
        });
    }
"#;

#[test]
fn repair_off_prevents_dist_to_src_fallback() {
    let harness = common::TestHarness::new("repair_off_no_fallback");

    // Create src/lib.ts but NOT dist/lib.js.
    harness.create_file(
        "extensions/src/lib.ts",
        br#"export const greeting = "from src";"#,
    );

    let (_manager, handle, _load_result) =
        try_start_runtime_with_mode(&harness, DIST_IMPORT_SOURCE, RepairMode::Off);

    // With Off mode the fallback should NOT fire → no "greet" tool registered.
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(
        !tools.iter().any(|t| t.name == "greet"),
        "Off mode should not apply dist→src fallback"
    );

    shutdown(&handle);
}

#[test]
fn repair_suggest_does_not_apply_fallback() {
    let harness = common::TestHarness::new("repair_suggest_no_apply");

    // Create src/lib.ts but NOT dist/lib.js.
    harness.create_file(
        "extensions/src/lib.ts",
        br#"export const greeting = "from src";"#,
    );

    let (_manager, handle, _load_result) =
        try_start_runtime_with_mode(&harness, DIST_IMPORT_SOURCE, RepairMode::Suggest);

    // Suggest mode should log but NOT apply → no "greet" tool registered.
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(
        !tools.iter().any(|t| t.name == "greet"),
        "Suggest mode should not apply dist→src fallback"
    );

    shutdown(&handle);
}

#[test]
fn repair_auto_safe_applies_dist_to_src_fallback() {
    let harness = common::TestHarness::new("repair_auto_safe_applies");

    // Create src/lib.ts but NOT dist/lib.js.
    harness.create_file(
        "extensions/src/lib.ts",
        br#"export const greeting = "from src";"#,
    );

    let (_manager, handle, _load_result) =
        try_start_runtime_with_mode(&harness, DIST_IMPORT_SOURCE, RepairMode::AutoSafe);

    // AutoSafe should apply the fallback → "greet" tool registered.
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(
        tools.iter().any(|t| t.name == "greet"),
        "AutoSafe mode should apply dist→src fallback"
    );

    shutdown(&handle);
}

#[test]
fn repair_auto_strict_applies_dist_to_src_fallback() {
    let harness = common::TestHarness::new("repair_auto_strict_applies");

    // Create src/lib.ts but NOT dist/lib.js.
    harness.create_file(
        "extensions/src/lib.ts",
        br#"export const greeting = "from src";"#,
    );

    let (_manager, handle, _load_result) =
        try_start_runtime_with_mode(&harness, DIST_IMPORT_SOURCE, RepairMode::AutoStrict);

    // AutoStrict should also apply the fallback → "greet" tool registered.
    let tools = common::run_async({
        let h = handle.clone();
        async move { h.get_registered_tools().await.unwrap() }
    });
    assert!(
        tools.iter().any(|t| t.name == "greet"),
        "AutoStrict mode should apply dist→src fallback"
    );

    shutdown(&handle);
}

// ─── Privilege monotonicity checker (bd-k5q5.9.1.3) ─────────────────────────

use pi::extensions_js::verify_repair_monotonicity;
use std::path::PathBuf;

#[test]
fn monotonicity_safe_when_resolved_within_root() {
    let root = PathBuf::from("/extensions/my-ext");
    let original = PathBuf::from("/extensions/my-ext/dist/lib.js");
    let resolved = PathBuf::from("/extensions/my-ext/src/lib.ts");
    assert_eq!(
        verify_repair_monotonicity(&root, &original, &resolved),
        MonotonicityVerdict::Safe
    );
}

#[test]
fn monotonicity_escapes_root_when_resolved_above() {
    let root = PathBuf::from("/extensions/my-ext");
    let original = PathBuf::from("/extensions/my-ext/dist/lib.js");
    let resolved = PathBuf::from("/extensions/other-ext/src/lib.ts");
    let verdict = verify_repair_monotonicity(&root, &original, &resolved);
    assert!(!verdict.is_safe(), "should detect escape: {verdict:?}");
    assert!(matches!(verdict, MonotonicityVerdict::EscapesRoot { .. }));
}

#[test]
fn monotonicity_escapes_root_when_resolved_to_parent() {
    let root = PathBuf::from("/extensions/my-ext");
    let original = PathBuf::from("/extensions/my-ext/dist/lib.js");
    let resolved = PathBuf::from("/extensions/lib.ts");
    let verdict = verify_repair_monotonicity(&root, &original, &resolved);
    assert!(!verdict.is_safe());
}

#[test]
fn monotonicity_safe_for_nested_subdirectory() {
    let root = PathBuf::from("/extensions/my-ext");
    let original = PathBuf::from("/extensions/my-ext/dist/deep/nested/file.js");
    let resolved = PathBuf::from("/extensions/my-ext/src/deep/nested/file.ts");
    assert_eq!(
        verify_repair_monotonicity(&root, &original, &resolved),
        MonotonicityVerdict::Safe
    );
}

#[test]
fn monotonicity_safe_at_root_boundary() {
    // Resolved path IS the root itself (edge case).
    let root = PathBuf::from("/extensions/my-ext");
    let original = PathBuf::from("/extensions/my-ext/dist/index.js");
    let resolved = PathBuf::from("/extensions/my-ext/index.ts");
    assert_eq!(
        verify_repair_monotonicity(&root, &original, &resolved),
        MonotonicityVerdict::Safe
    );
}

// ─── Repair risk classification (bd-k5q5.9.1.4) ─────────────────────────────

#[test]
fn safe_patterns_have_safe_risk() {
    assert_eq!(RepairPattern::DistToSrc.risk(), RepairRisk::Safe);
    assert_eq!(RepairPattern::MissingAsset.risk(), RepairRisk::Safe);
    assert_eq!(
        RepairPattern::ManifestNormalization.risk(),
        RepairRisk::Safe
    );
}

#[test]
fn aggressive_patterns_have_aggressive_risk() {
    assert_eq!(RepairPattern::MonorepoEscape.risk(), RepairRisk::Aggressive);
    assert_eq!(RepairPattern::MissingNpmDep.risk(), RepairRisk::Aggressive);
    assert_eq!(RepairPattern::ExportShape.risk(), RepairRisk::Aggressive);
    assert_eq!(RepairPattern::ApiMigration.risk(), RepairRisk::Aggressive);
}

#[test]
fn safe_patterns_allowed_by_auto_safe() {
    assert!(RepairPattern::DistToSrc.is_allowed_by(RepairMode::AutoSafe));
    assert!(RepairPattern::MissingAsset.is_allowed_by(RepairMode::AutoSafe));
}

#[test]
fn aggressive_patterns_blocked_by_auto_safe() {
    assert!(!RepairPattern::MonorepoEscape.is_allowed_by(RepairMode::AutoSafe));
    assert!(!RepairPattern::MissingNpmDep.is_allowed_by(RepairMode::AutoSafe));
    assert!(!RepairPattern::ExportShape.is_allowed_by(RepairMode::AutoSafe));
}

#[test]
fn aggressive_patterns_allowed_by_auto_strict() {
    assert!(RepairPattern::MonorepoEscape.is_allowed_by(RepairMode::AutoStrict));
    assert!(RepairPattern::MissingNpmDep.is_allowed_by(RepairMode::AutoStrict));
    assert!(RepairPattern::ExportShape.is_allowed_by(RepairMode::AutoStrict));
}

#[test]
fn no_patterns_allowed_by_off() {
    for &pattern in &[
        RepairPattern::DistToSrc,
        RepairPattern::MissingAsset,
        RepairPattern::MonorepoEscape,
        RepairPattern::MissingNpmDep,
        RepairPattern::ExportShape,
        RepairPattern::ManifestNormalization,
        RepairPattern::ApiMigration,
    ] {
        assert!(
            !pattern.is_allowed_by(RepairMode::Off),
            "{pattern} should be blocked in Off mode"
        );
    }
}

#[test]
fn no_patterns_allowed_by_suggest() {
    for &pattern in &[
        RepairPattern::DistToSrc,
        RepairPattern::MissingAsset,
        RepairPattern::MonorepoEscape,
        RepairPattern::MissingNpmDep,
        RepairPattern::ExportShape,
        RepairPattern::ManifestNormalization,
        RepairPattern::ApiMigration,
    ] {
        assert!(
            !pattern.is_allowed_by(RepairMode::Suggest),
            "{pattern} should be blocked in Suggest mode"
        );
    }
}

// ─── Deterministic rule registry (bd-k5q5.9.3.1) ────────────────────────────

use pi::extensions_js::{applicable_rules, rule_by_id};

#[test]
fn registry_has_seven_rules() {
    assert_eq!(REPAIR_RULES.len(), 7);
}

#[test]
fn registry_version_is_set() {
    assert!(!REPAIR_REGISTRY_VERSION.is_empty());
}

#[test]
fn all_rules_have_unique_ids() {
    let mut ids: Vec<&str> = REPAIR_RULES.iter().map(|r| r.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), REPAIR_RULES.len(), "duplicate rule IDs detected");
}

#[test]
fn rule_by_id_finds_existing() {
    let rule = rule_by_id("dist_to_src_v1").expect("should find rule");
    assert_eq!(rule.pattern, RepairPattern::DistToSrc);
    assert_eq!(rule.version, "1.0.0");
}

#[test]
fn rule_by_id_returns_none_for_unknown() {
    assert!(rule_by_id("nonexistent_rule").is_none());
}

#[test]
fn applicable_rules_auto_safe_returns_safe_only() {
    let rules = applicable_rules(RepairMode::AutoSafe);
    assert!(rules.iter().all(|r| r.risk() == RepairRisk::Safe));
    assert_eq!(rules.len(), 3); // DistToSrc + MissingAsset + ManifestNormalization
}

#[test]
fn applicable_rules_auto_strict_returns_all() {
    let rules = applicable_rules(RepairMode::AutoStrict);
    assert_eq!(rules.len(), 7);
}

#[test]
fn applicable_rules_off_returns_empty() {
    let rules = applicable_rules(RepairMode::Off);
    assert!(rules.is_empty());
}

#[test]
fn applicable_rules_suggest_returns_empty() {
    let rules = applicable_rules(RepairMode::Suggest);
    assert!(rules.is_empty());
}

#[test]
fn registry_order_is_deterministic() {
    let ids: Vec<&str> = REPAIR_RULES.iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec![
            "dist_to_src_v1",
            "missing_asset_v1",
            "monorepo_escape_v1",
            "missing_npm_dep_v1",
            "export_shape_v1",
            "manifest_schema_v1",
            "api_migration_v1",
        ]
    );
}

// ─── Model patch primitives (bd-k5q5.9.4.1) ─────────────────────────────────

#[test]
fn patch_op_replace_module_path_is_safe() {
    let op = PatchOp::ReplaceModulePath {
        from: "./dist/lib.js".to_string(),
        to: "./src/lib.ts".to_string(),
    };
    assert_eq!(op.risk(), RepairRisk::Safe);
    assert_eq!(op.tag(), "replace_module_path");
}

#[test]
fn patch_op_inject_stub_is_aggressive() {
    let op = PatchOp::InjectStub {
        virtual_path: "/@stubs/missing.js".to_string(),
        source: "export default {};".to_string(),
    };
    assert_eq!(op.risk(), RepairRisk::Aggressive);
    assert_eq!(op.tag(), "inject_stub");
}

#[test]
fn patch_op_add_export_is_aggressive() {
    let op = PatchOp::AddExport {
        module_path: "./index.ts".to_string(),
        export_name: "activate".to_string(),
        export_value: "function activate() {}".to_string(),
    };
    assert_eq!(op.risk(), RepairRisk::Aggressive);
}

#[test]
fn patch_op_rewrite_require_is_safe() {
    let op = PatchOp::RewriteRequire {
        module_path: "./entry.js".to_string(),
        from_specifier: "missing-pkg".to_string(),
        to_specifier: "/@stubs/missing-pkg".to_string(),
    };
    assert_eq!(op.risk(), RepairRisk::Safe);
}

#[test]
fn proposal_max_risk_safe_when_all_safe() {
    let proposal = PatchProposal {
        rule_id: "dist_to_src_v1".to_string(),
        ops: vec![PatchOp::ReplaceModulePath {
            from: "./dist/x.js".to_string(),
            to: "./src/x.ts".to_string(),
        }],
        rationale: "remap path".to_string(),
        confidence: Some(0.95),
    };
    assert_eq!(proposal.max_risk(), RepairRisk::Safe);
    assert!(proposal.is_allowed_by(RepairMode::AutoSafe));
}

#[test]
fn proposal_max_risk_aggressive_when_any_aggressive() {
    let proposal = PatchProposal {
        rule_id: "mixed_v1".to_string(),
        ops: vec![
            PatchOp::ReplaceModulePath {
                from: "./a.js".to_string(),
                to: "./b.ts".to_string(),
            },
            PatchOp::InjectStub {
                virtual_path: "/@stubs/x".to_string(),
                source: "export default {};".to_string(),
            },
        ],
        rationale: "mixed ops".to_string(),
        confidence: None,
    };
    assert_eq!(proposal.max_risk(), RepairRisk::Aggressive);
    assert!(!proposal.is_allowed_by(RepairMode::AutoSafe));
    assert!(proposal.is_allowed_by(RepairMode::AutoStrict));
}

#[test]
fn proposal_blocked_by_off_mode() {
    let proposal = PatchProposal {
        rule_id: "safe_v1".to_string(),
        ops: vec![PatchOp::ReplaceModulePath {
            from: "a".to_string(),
            to: "b".to_string(),
        }],
        rationale: "test".to_string(),
        confidence: None,
    };
    assert!(!proposal.is_allowed_by(RepairMode::Off));
}

#[test]
fn empty_proposal_is_safe() {
    let proposal = PatchProposal {
        rule_id: "empty_v1".to_string(),
        ops: vec![],
        rationale: "no-op".to_string(),
        confidence: None,
    };
    // No aggressive ops → Safe
    assert_eq!(proposal.max_risk(), RepairRisk::Safe);
}
