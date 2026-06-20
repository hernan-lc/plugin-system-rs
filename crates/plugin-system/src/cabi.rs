//! C-ABI ("c-flat") plugin loader.
//!
//! Plugins written in any language that can produce a C-ABI compatible shared
//! library can opt into the flat ABI by placing a `*.manifest.json` file next
//! to the library that contains `"abi": "c-flat"`. The host then loads the
//! library and dispatches via exported C functions instead of the Rust trait
//! object vtable, eliminating the language interop blocker.
//!
//! Required exports (all `extern "C"` unless noted):
//!
//! ```c
//! void*        plugin_<prefix>_create(void);                          // returns an opaque plugin context
//! void         plugin_<prefix>_destroy(void* ctx);
//! const char*  plugin_<prefix>_metadata_json(void);                    // heap-allocated, freed with plugin_<prefix>_free_string
//! void         plugin_<prefix>_free_string(char* s);
//! void         plugin_<prefix>_on_load(void* ctx);
//! void         plugin_<prefix>_on_unload(void* ctx);
//! // Returns 0 on success, writes a heap-allocated JSON result to *out.
//! // The host frees it with plugin_<prefix>_free_string.
//! int          plugin_<prefix>_handle_command(void* ctx, const char* method, const char* args_json, char** out);
//! ```
//!
//! Unprefixed names (`plugin_create`, `plugin_destroy`, …) are also accepted
//! for plugins that only export a single instance.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use crate::context::PluginContext;
use crate::traits::{Plugin, PluginMetadata};
use serde::{Deserialize, Serialize};

/// Plugin manifest with C-ABI extension fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAbiManifest {
    #[serde(flatten)]
    pub base: super::manifest::Manifest,
    /// Set to `"c-flat"` to enable C-ABI dispatch.
    #[serde(default)]
    pub abi: Option<String>,
    /// Optional set of interface ids the plugin implements.
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// Optional pre-load configuration JSON.
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

pub fn is_cabi_manifest(value: &serde_json::Value) -> bool {
    value
        .get("abi")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("c-flat") || s.eq_ignore_ascii_case("c_abi"))
        .unwrap_or(false)
}

// Raw C function pointer types
type PluginCreateFn = unsafe extern "C" fn() -> *mut c_void;
type PluginDestroyFn = unsafe extern "C" fn(*mut c_void);
type PluginMetadataJsonFn = unsafe extern "C" fn() -> *mut c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut c_char);
type PluginOnLoadFn = unsafe extern "C" fn(*mut c_void);
type PluginOnUnloadFn = unsafe extern "C" fn(*mut c_void);
type PluginHandleCommandFn =
    unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, *mut *mut c_char) -> c_int;

/// Runtime representation of a C-ABI plugin loaded into the host.
pub struct CAbiPlugin {
    /// Library is owned by `CAbiPlugin` and unloaded when `self` drops, *after*
    /// `plugin_destroy` has been called on the context. The field is never
    /// read; its purpose is to extend the lifetime of the loaded library so
    /// the function pointers below stay valid.
    #[allow(dead_code)]
    lib: libloading::Library,
    ctx: *mut c_void,
    metadata: PluginMetadata,
    on_load: Option<PluginOnLoadFn>,
    on_unload: Option<PluginOnUnloadFn>,
    handle_command: PluginHandleCommandFn,
    free_string: PluginFreeStringFn,
    destroy: PluginDestroyFn,
    interface_ids: Vec<String>,
}

// Safety: the C-ABI guarantees the host never calls two methods on the same
// instance concurrently; the trait says `Send + Sync` and we uphold that.
unsafe impl Send for CAbiPlugin {}
unsafe impl Sync for CAbiPlugin {}

impl CAbiPlugin {
    /// Build a C-ABI plugin from a freshly-loaded library. The library is
    /// owned by `self` and unloaded when `self` is dropped *after* `destroy`
    /// has been called.
    pub fn from_library(
        lib: libloading::Library,
        manifest: &CAbiManifest,
        prefix: Option<&str>,
    ) -> Result<Self, String> {
        let prefixed = |base: &str| -> String {
            match prefix {
                Some(p) => format!("plugin_{p}_{base}"),
                None => format!("plugin_{base}"),
            }
        };

        let create: PluginCreateFn = lookup(&lib, &prefixed("create"), "plugin_create")?;
        let destroy: PluginDestroyFn = lookup(&lib, &prefixed("destroy"), "plugin_destroy")?;
        let metadata_json: PluginMetadataJsonFn =
            lookup(&lib, &prefixed("metadata_json"), "plugin_metadata_json")?;
        let free_string: PluginFreeStringFn =
            lookup(&lib, &prefixed("free_string"), "plugin_free_string")?;
        let handle_command: PluginHandleCommandFn =
            lookup(&lib, &prefixed("handle_command"), "plugin_handle_command")?;
        let on_load: Option<PluginOnLoadFn> =
            try_lookup(&lib, &prefixed("on_load")).or_else(|| try_lookup(&lib, "plugin_on_load"));
        let on_unload: Option<PluginOnUnloadFn> = try_lookup(&lib, &prefixed("on_unload"))
            .or_else(|| try_lookup(&lib, "plugin_on_unload"));

        let metadata = unsafe { read_and_free_json(metadata_json, free_string) }
            .map_err(|e| format!("reading metadata: {e}"))?;

        // Instantiate the plugin.
        // SAFETY: `create` returns a heap-allocated opaque pointer; the plugin
        // owns its lifetime and `destroy` will reclaim it.
        let ctx = unsafe { create() };

        Ok(Self {
            lib,
            ctx,
            metadata,
            on_load,
            on_unload,
            handle_command,
            free_string,
            destroy,
            interface_ids: manifest.interfaces.clone(),
        })
    }

    /// The parsed plugin metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
}

impl Plugin for CAbiPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        if let Some(on_load) = self.on_load {
            // SAFETY: caller ensures no concurrent access.
            unsafe { on_load(self.ctx) };
        }
    }

    fn on_unload(&mut self) {
        if let Some(on_unload) = self.on_unload {
            // SAFETY: caller ensures no concurrent access.
            unsafe { on_unload(self.ctx) };
        }
    }

    fn plugin_type_name(&self) -> &'static str {
        "CAbiPlugin"
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        // Each interface id is owned by `self.interface_ids`; leak them into
        // `'static str` so the trait signature is satisfied. The number of
        // ids is small (typically 0-5) so the leak is bounded.
        self.interface_ids
            .iter()
            .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str)
            .collect()
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        None
    }

    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        let args_json = match serde_json::to_string(&args) {
            Ok(s) => s,
            Err(e) => {
                log::error!("C-ABI plugin: failed to serialize args: {e}");
                return None;
            }
        };
        // Build NUL-terminated C strings for the FFI call. CString::new
        // fails only if the input contains an internal NUL byte, which a
        // well-formed method/JSON won't.
        let method_c = match CString::new(method) {
            Ok(s) => s,
            Err(_) => {
                log::error!("C-ABI plugin: method contains NUL byte");
                return None;
            }
        };
        let args_c = match CString::new(args_json) {
            Ok(s) => s,
            Err(_) => {
                log::error!("C-ABI plugin: args JSON contains NUL byte");
                return None;
            }
        };
        let mut result_ptr: *mut c_char = std::ptr::null_mut();
        // SAFETY: `method_c` and `args_c` are valid NUL-terminated C strings
        // for the duration of the call. The plugin must set `*result_out` to
        // a heap-allocated NUL-terminated string on success, or leave it null.
        let rc = unsafe {
            (self.handle_command)(
                self.ctx,
                method_c.as_ptr(),
                args_c.as_ptr(),
                &mut result_ptr,
            )
        };
        if rc != 0 || result_ptr.is_null() {
            if rc != 0 {
                log::error!("C-ABI plugin: handle_command returned {rc}");
            }
            return None;
        }
        // SAFETY: the plugin guarantees `result_ptr` is a heap-allocated
        // NUL-terminated string it allocated. We must free it with the
        // plugin's `free_string` export.
        let cstr = unsafe { CStr::from_ptr(result_ptr) };
        let json = match cstr.to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                log::error!("C-ABI plugin: result is not valid UTF-8: {e}");
                unsafe { (self.free_string)(result_ptr) };
                return None;
            }
        };
        unsafe { (self.free_string)(result_ptr) };
        serde_json::from_str(&json).ok()
    }
}

impl Drop for CAbiPlugin {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            // SAFETY: we own the context and the library is still loaded.
            unsafe { (self.destroy)(self.ctx) };
        }
    }
}

fn lookup<T: Copy>(lib: &libloading::Library, primary: &str, fallback: &str) -> Result<T, String> {
    if let Some(sym) = try_lookup(lib, primary) {
        return Ok(sym);
    }
    try_lookup(lib, fallback)
        .ok_or_else(|| format!("required symbol `{primary}` (or `{fallback}`) not found in plugin"))
}

fn try_lookup<T: Copy>(lib: &libloading::Library, name: &str) -> Option<T> {
    // SAFETY: the symbol exists if the closure returns Some.
    unsafe { lib.get::<T>(name.as_bytes()) }.ok().map(|s| *s)
}

unsafe fn read_and_free_json(
    metadata_json: PluginMetadataJsonFn,
    free_string: PluginFreeStringFn,
) -> Result<PluginMetadata, String> {
    let ptr = metadata_json();
    if ptr.is_null() {
        return Err("plugin_metadata_json returned NULL".into());
    }
    let cstr = CStr::from_ptr(ptr);
    let json = cstr
        .to_str()
        .map_err(|e| format!("metadata is not valid UTF-8: {e}"))?
        .to_string();
    free_string(ptr);
    serde_json::from_str(&json).map_err(|e| format!("metadata is not valid JSON: {e}"))
}
