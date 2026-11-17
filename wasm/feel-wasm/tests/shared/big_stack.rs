//! The thread that the deepest requests need.

/// Runs `body` on a thread with a big stack natively, and directly on wasm32. The nesting and
/// call-depth limits are sized for the release wasm build in a browser worker; unoptimized native
/// test builds use much bigger stack frames than that.
pub fn with_big_stack(body: impl FnOnce() + Send + 'static) {
  #[cfg(not(target_arch = "wasm32"))]
  if let Err(panic) = std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(body).unwrap().join() {
    std::panic::resume_unwind(panic);
  }
  #[cfg(target_arch = "wasm32")]
  body();
}
