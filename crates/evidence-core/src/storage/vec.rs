//! Register the `sqlite-vec` extension as a SQLite auto-extension.
//!
//! [`ensure_registered`] runs the FFI registration exactly once per process
//! via [`std::sync::Once`]. Subsequent calls are cheap and idempotent. After
//! it returns, every `rusqlite::Connection` opened in this process
//! transparently has access to the `vec0` virtual-table module.
//!
//! This is the only `unsafe` in the crate. The contract is:
//!
//! - `sqlite3_vec_init` is a valid extension entry point exported by the
//!   linked `sqlite-vec` C library and has the C signature SQLite expects.
//! - `sqlite3_auto_extension` stores the function pointer for the lifetime
//!   of the process. We never deregister.
//! - Both calls are FFI and require `transmute`-shaped fn-pointer coercion.
//!   The pattern is taken verbatim from `sqlite-vec`'s own integration test.

use std::sync::Once;

/// Idempotently register `sqlite-vec` for every connection opened in this
/// process. Call once before opening any connection that needs `vec0`.
pub fn ensure_registered() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(register);
}

#[allow(unsafe_code)]
fn register() {
    use rusqlite::ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension};
    type InitFn = unsafe extern "C" fn(
        *mut sqlite3,
        *mut *mut std::os::raw::c_char,
        *const sqlite3_api_routines,
    ) -> std::os::raw::c_int;

    // SAFETY: `sqlite3_vec_init` is FFI-defined by the `sqlite-vec` crate as
    // an `extern "C" fn()` — its real C signature matches `InitFn` (the
    // SQLite extension entry-point shape). SQLite copies the pointer into a
    // process-global table and calls it once per new connection; the pointer
    // must remain callable for the rest of the process, which it does
    // because the symbol is linked statically. The `transmute` matches the
    // upstream auto-extension example in `sqlite-vec`'s own tests.
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<*const (), InitFn>(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
}
