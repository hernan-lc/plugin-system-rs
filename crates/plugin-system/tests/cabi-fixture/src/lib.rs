//! A minimal C-ABI ("c-flat") plugin used as a test fixture for the
//! `plugin-system` integration tests.
//!
//! Exports the full set of C functions expected by `plugin_system::cabi`:
//!
//!   - `plugin_create`        -> opaque context
//!   - `plugin_destroy`
//!   - `plugin_metadata_json` -> heap-allocated JSON, freed by `plugin_free_string`
//!   - `plugin_free_string`
//!   - `plugin_on_load`
//!   - `plugin_on_unload`
//!   - `plugin_handle_command` -> returns 0 and writes a heap JSON result on success

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

/// Opaque context. The C-ABI permits any pointer; we use an empty struct.
struct Context;

static METADATA_JSON: &str = r#"{
    "name": "cabi-fixture",
    "version": "1.2.3",
    "authors": ["StreamDeck Test Suite"],
    "dependencies": []
}"#;

/// Allocate a C string. Caller frees with `plugin_free_string`.
fn into_c_string(s: &str) -> *mut c_char {
    CString::new(s).expect("CString::new").into_raw()
}

/// Allocate a fresh plugin context.
///
/// # Safety
/// The host must call `plugin_destroy` exactly once on the returned pointer.
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut c_void {
    Box::into_raw(Box::new(Context)) as *mut c_void
}

/// Reclaim a context previously returned by `plugin_create`.
///
/// # Safety
/// `ctx` must either be null or a pointer previously returned by
/// `plugin_create` that has not yet been destroyed.
#[no_mangle]
pub unsafe extern "C" fn plugin_destroy(ctx: *mut c_void) {
    if !ctx.is_null() {
        let _ = Box::from_raw(ctx as *mut Context);
    }
}

#[no_mangle]
pub extern "C" fn plugin_metadata_json() -> *mut c_char {
    into_c_string(METADATA_JSON)
}

/// Free a string previously returned by this plugin.
///
/// # Safety
/// `s` must either be null or a pointer previously returned by
/// `plugin_metadata_json` or `plugin_handle_command`.
#[no_mangle]
pub unsafe extern "C" fn plugin_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// # Safety
/// `ctx` must be a pointer previously returned by `plugin_create`, or null.
#[no_mangle]
pub unsafe extern "C" fn plugin_on_load(_ctx: *mut c_void) {}

/// # Safety
/// `ctx` must be a pointer previously returned by `plugin_create`, or null.
#[no_mangle]
pub unsafe extern "C" fn plugin_on_unload(_ctx: *mut c_void) {}

/// Implements two commands:
///   - "echo" with args `{"message": "..."}` returns `{"echoed": "..."}`
///   - "add"  with args `{"a": int, "b": int}`     returns `{"sum": int}`
///
/// # Safety
/// - `ctx` must be a pointer previously returned by `plugin_create`, or null.
/// - `method` and `args_json` must be valid NUL-terminated UTF-8 C strings
///   (or null for `args_json`, which is treated as `{}`).
/// - `out` must be a valid pointer to a `*mut c_char` slot; on success the
///   plugin writes a heap-allocated NUL-terminated string there which the
///   host must free with `plugin_free_string`.
#[no_mangle]
pub unsafe extern "C" fn plugin_handle_command(
    _ctx: *mut c_void,
    method: *const c_char,
    args_json: *const c_char,
    out: *mut *mut c_char,
) -> c_int {
    let method = match CStr::from_ptr(method).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let args_str = if args_json.is_null() {
        "{}"
    } else {
        match CStr::from_ptr(args_json).to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        }
    };
    let args: serde_json::Value = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(_) => serde_json::json!({}),
    };

    let result = match method {
        "echo" => {
            let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            serde_json::json!({ "echoed": msg })
        }
        "add" => {
            let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            serde_json::json!({ "sum": a + b })
        }
        _ => return 1,
    };

    let serialized = match serde_json::to_string(&result) {
        Ok(s) => s,
        Err(_) => return 1,
    };
    *out = into_c_string(&serialized);
    0
}
