//! # feel-wasm
//!
//! The dsntk FEEL engine (the local fork, pure-Rust `dashu` decimal128 backend) exposed through a tiny
//! JSON-in / JSON-out ABI, built for `wasm32-wasip1` as a reactor module.
//!
//! The binding specification is `docs/CONTRACT.md` (§1–3). In short:
//!
//! * [`feel_alloc`] `(len) -> ptr` — allocate a request buffer;
//! * [`feel_call`] `(ptr, len) -> (resp_ptr << 32) | resp_len` — takes ownership of the request
//!   buffer and returns a newly allocated UTF-8 JSON response;
//! * [`feel_free`] `(ptr, len)` — frees a buffer from `feel_alloc` (if it was never passed to
//!   `feel_call`) or a response returned by `feel_call`.
//!
//! All recoverable problems (bad JSON, syntax errors, bad context, ...) are reported as
//! `{"ok": false, "error": {...}}` responses and the instance stays healthy.
//!
//! **Panics trap.** The release profile uses `panic = "abort"` (and `wasm32-wasip1` has no
//! unwinding anyway), so a Rust panic inside dsntk becomes a wasm `unreachable` trap. The host
//! must then discard the instance and create a new one; it must never call into a trapped
//! instance again. On native targets (tests) panics are caught and reported as
//! `{"ok": false, "error": {"kind": "internal"}}`.
//!
//! The pure-Rust entry points [`call`] / [`call_bytes`] implement the same protocol without the
//! raw-memory ABI. The ABI exports themselves exist only on wasm32: the packed `u64` return value
//! of `feel_call` can not hold a 64-bit pointer.

mod display;
mod engine;
mod input;

pub use engine::{call, call_bytes};

/// Allocates `len` bytes in linear memory and returns the pointer.
///
/// The buffer must be released either by passing it to [`feel_call`] (which takes ownership)
/// or with [`feel_free`]`(ptr, len)`.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn feel_alloc(len: usize) -> *mut u8 {
  let buffer: Box<[u8]> = vec![0u8; len].into_boxed_slice();
  Box::into_raw(buffer) as *mut u8
}

/// Frees a buffer returned by [`feel_alloc`] or a response returned by [`feel_call`].
///
/// # Safety
/// `ptr`/`len` must describe exactly one live buffer obtained from this module.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn feel_free(ptr: *mut u8, len: usize) {
  if ptr.is_null() {
    return;
  }
  // SAFETY: the caller guarantees (ptr, len) is a buffer allocated as Box<[u8]> by this module.
  drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
}

/// Processes the UTF-8 JSON request at (`ptr`, `len`), taking ownership of that buffer.
///
/// Returns `(resp_ptr << 32) | resp_len`; the caller must `feel_free(resp_ptr, resp_len)`.
/// Pointers and lengths are 32-bit on wasm32, so both halves fit.
///
/// # Safety
/// (`ptr`, `len`) must be a buffer obtained from [`feel_alloc`]`(len)` and not freed yet.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn feel_call(ptr: *mut u8, len: usize) -> u64 {
  let request: Box<[u8]> = if ptr.is_null() {
    Box::new([])
  } else {
    // SAFETY: the caller hands over a buffer allocated by `feel_alloc(len)`.
    unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) }
  };
  let response = engine::call_bytes(&request);
  drop(request);
  let response: Box<[u8]> = response.into_bytes().into_boxed_slice();
  let resp_len = response.len() as u32;
  let resp_ptr = Box::into_raw(response) as *mut u8 as usize as u32;
  ((resp_ptr as u64) << 32) | resp_len as u64
}
