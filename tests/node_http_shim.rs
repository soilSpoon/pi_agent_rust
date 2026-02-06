//! Unit tests for the node:http and node:https shims (bd-1av0.8).
//!
//! Tests verify that `http.request`, `http.get`, `https.request`, `https.get`
//! return `ClientRequest` objects with the correct API surface (`write`, `end`,
//! `on`, `abort`, `destroy`), that `STATUS_CODES` and `METHODS` are exported,
//! and that `createServer` throws as expected. Network tests verify error
//! handling when no `pi.http()` hostcall is available.

mod common;

use pi::extensions::{
    ExtensionEventName, ExtensionManager, JsExtensionLoadSpec, JsExtensionRuntimeHandle,
};
use pi::extensions_js::PiJsRuntimeConfig;
use pi::tools::ToolRegistry;
use std::sync::Arc;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn load_ext(harness: &common::TestHarness, source: &str) -> ExtensionManager {
    let cwd = harness.temp_dir().to_path_buf();
    let ext_entry_path = harness.create_file("extensions/http_test.mjs", source.as_bytes());
    let spec = JsExtensionLoadSpec::from_entry_path(&ext_entry_path).expect("load spec");

    let manager = ExtensionManager::new();
    let tools = Arc::new(ToolRegistry::new(&[], &cwd, None));
    let js_config = PiJsRuntimeConfig {
        cwd: cwd.display().to_string(),
        ..Default::default()
    };

    let runtime = common::run_async({
        let manager = manager.clone();
        let tools = Arc::clone(&tools);
        async move {
            JsExtensionRuntimeHandle::start(js_config, tools, manager)
                .await
                .expect("start js runtime")
        }
    });
    manager.set_js_runtime(runtime);

    common::run_async({
        let manager = manager.clone();
        async move {
            manager
                .load_js_extensions(vec![spec])
                .await
                .expect("load extension");
        }
    });

    manager
}

fn http_ext_source(js_expr: &str) -> String {
    format!(
        r#"
import http from "node:http";

export default function activate(pi) {{
  pi.on("agent_start", (event, ctx) => {{
    let result;
    try {{
      result = String({js_expr});
    }} catch (e) {{
      result = "ERROR:" + e.message;
    }}
    return {{ result }};
  }});
}}
"#
    )
}

fn eval_http(js_expr: &str) -> String {
    let harness = common::TestHarness::new("http_shim");
    let source = http_ext_source(js_expr);
    let mgr = load_ext(&harness, &source);

    let response = common::run_async(async move {
        mgr.dispatch_event_with_response(ExtensionEventName::AgentStart, None, 10000)
            .await
            .expect("dispatch agent_start")
    });

    response
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_else(|| "NO_RESPONSE".to_string())
}

// ─── STATUS_CODES export ────────────────────────────────────────────────────

#[test]
fn status_codes_exported() {
    let result = eval_http(r"http.STATUS_CODES[200]");
    assert_eq!(result, "OK");
}

#[test]
fn status_codes_404() {
    let result = eval_http(r"http.STATUS_CODES[404]");
    assert_eq!(result, "Not Found");
}

// ─── METHODS export ─────────────────────────────────────────────────────────

#[test]
fn methods_includes_standard() {
    let result = eval_http(
        r"http.METHODS.includes('GET') && http.METHODS.includes('POST') && http.METHODS.includes('PUT')",
    );
    assert_eq!(result, "true");
}

// ─── createServer throws ────────────────────────────────────────────────────

#[test]
fn create_server_throws() {
    let result = eval_http(r"http.createServer()");
    assert!(
        result.contains("ERROR:"),
        "createServer should throw, got: {result}"
    );
    assert!(result.contains("not available"), "got: {result}");
}

// ─── request returns ClientRequest ──────────────────────────────────────────

#[test]
fn request_returns_object_with_write() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return typeof req.write === 'function';
    })()",
    );
    assert_eq!(result, "true");
}

#[test]
fn request_returns_object_with_end() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return typeof req.end === 'function';
    })()",
    );
    assert_eq!(result, "true");
}

#[test]
fn request_returns_object_with_on() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return typeof req.on === 'function';
    })()",
    );
    assert_eq!(result, "true");
}

#[test]
fn request_returns_object_with_abort() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return typeof req.abort === 'function';
    })()",
    );
    assert_eq!(result, "true");
}

#[test]
fn request_returns_object_with_destroy() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return typeof req.destroy === 'function';
    })()",
    );
    assert_eq!(result, "true");
}

// ─── get auto-ends request ──────────────────────────────────────────────────

#[test]
fn get_auto_ends() {
    let result = eval_http(
        r"(() => {
        const req = http.get({ hostname: 'example.com', path: '/' });
        return req._ended;
    })()",
    );
    assert_eq!(result, "true");
}

// ─── request method ─────────────────────────────────────────────────────────

#[test]
fn request_method_defaults_to_get() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        return req.method;
    })()",
    );
    assert_eq!(result, "GET");
}

#[test]
fn request_method_can_be_set() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', method: 'POST', path: '/' });
        return req.method;
    })()",
    );
    assert_eq!(result, "POST");
}

// ─── request path ───────────────────────────────────────────────────────────

#[test]
fn request_path_from_options() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/api/v1' });
        return req.path;
    })()",
    );
    assert_eq!(result, "/api/v1");
}

// ─── ClientRequest.write accumulates body ───────────────────────────────────

#[test]
fn write_accumulates_body() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        req.write('part1');
        req.write('part2');
        return req._body.join('');
    })()",
    );
    assert_eq!(result, "part1part2");
}

// ─── Import styles ──────────────────────────────────────────────────────────

#[test]
fn named_import_works() {
    let harness = common::TestHarness::new("http_named_import");
    let source = r#"
import { request, STATUS_CODES } from "node:http";

export default function activate(pi) {
  pi.on("agent_start", (event, ctx) => {
    return { result: typeof request + ":" + STATUS_CODES[200] };
  });
}
"#;
    let mgr = load_ext(&harness, source);
    let response = common::run_async(async move {
        mgr.dispatch_event_with_response(ExtensionEventName::AgentStart, None, 10000)
            .await
            .expect("dispatch")
    });
    let result = response
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_default();
    assert_eq!(result, "function:OK");
}

#[test]
fn bare_http_import_works() {
    let harness = common::TestHarness::new("http_bare_import");
    let source = r#"
import http from "http";

export default function activate(pi) {
  pi.on("agent_start", (event, ctx) => {
    return { result: typeof http.request };
  });
}
"#;
    let mgr = load_ext(&harness, source);
    let response = common::run_async(async move {
        mgr.dispatch_event_with_response(ExtensionEventName::AgentStart, None, 10000)
            .await
            .expect("dispatch")
    });
    let result = response
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_default();
    assert_eq!(result, "function");
}

// ─── HTTPS module ───────────────────────────────────────────────────────────

#[test]
fn https_request_exists() {
    let harness = common::TestHarness::new("https_import");
    let source = r#"
import https from "node:https";

export default function activate(pi) {
  pi.on("agent_start", (event, ctx) => {
    return { result: typeof https.request + ":" + typeof https.get };
  });
}
"#;
    let mgr = load_ext(&harness, source);
    let response = common::run_async(async move {
        mgr.dispatch_event_with_response(ExtensionEventName::AgentStart, None, 10000)
            .await
            .expect("dispatch")
    });
    let result = response
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_default();
    assert_eq!(result, "function:function");
}

#[test]
fn https_create_server_throws() {
    let harness = common::TestHarness::new("https_server");
    let source = r#"
import https from "node:https";

export default function activate(pi) {
  pi.on("agent_start", (event, ctx) => {
    try { https.createServer(); return { result: "no-throw" }; }
    catch(e) { return { result: "threw:" + e.message }; }
  });
}
"#;
    let mgr = load_ext(&harness, source);
    let response = common::run_async(async move {
        mgr.dispatch_event_with_response(ExtensionEventName::AgentStart, None, 10000)
            .await
            .expect("dispatch")
    });
    let result = response
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_default();
    assert!(
        result.starts_with("threw:"),
        "createServer should throw, got: {result}"
    );
}

// ─── Error on request without hostcall ──────────────────────────────────────

#[test]
fn request_emits_error_without_hostcall() {
    let result = eval_http(
        r"(() => {
        const req = http.request({ hostname: 'example.com', path: '/' });
        let errorMsg = 'none';
        req.on('error', (err) => { errorMsg = err.message; });
        req.end();
        return errorMsg;
    })()",
    );
    // Should get an error since pi.http() isn't wired up in unit tests
    assert!(result != "none", "expected error event, got: {result}");
}
