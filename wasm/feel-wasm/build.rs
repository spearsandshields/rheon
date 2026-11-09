//! Two jobs:
//!
//! 1. **Engine version.** The engine crates live in this repository, so the version is the
//!    repository commit (`RHEON_COMMIT` from the environment, else `.cargo_vcs_info.json` of a
//!    packaged crate, else `git rev-parse HEAD` with `-dirty` when the tree has uncommitted
//!    changes) plus the upstream dsntk-rs base commit from this crate's `Cargo.toml`
//!    (`[package.metadata.rheon] upstream-commit`). Both reach the crate as `RHEON_COMMIT` /
//!    `RHEON_UPSTREAM_COMMIT` (`env!`), so the `info` op reports exactly what was built.
//!
//! 2. **WASI reactor link** (docs/CONTRACT.md §1, `wasm32-wasip1` only). Stable rustc has no
//!    `-Z wasi-exec-model=reactor`, and without it wasm-ld wraps every export in a "command
//!    export" that runs the C destructors (`__wasm_call_dtors` from Rust's self-contained
//!    wasi-libc) after *each* call. Linking Rust's own `crt1-reactor.o` provides `_initialize`
//!    (constructors run once, by the host) and turns the module into a proper reactor without
//!    per-call wrappers.
//!
//!    It also raises the shadow stack from Rust's default 1 MiB to 8 MiB: dsntk's parser and
//!    evaluator are recursive, and with 1 MiB a chain of ~2000 `1+1+...` terms overflowed. A
//!    stack overflow is a trap (the stack sits first in linear memory, so it runs off address 0).
//!
//!    Linear memory is capped at 512 MiB (`--max-memory`): a runaway expression (e.g. a string
//!    doubled 40 times) then fails its allocation early (a trap → `crash`) instead of growing the
//!    host process to ~4 GiB first.
//!
//!    On wasm32-wasip1 a missing `crt1-reactor.o` is a hard build error: without it the module
//!    would silently lose `_initialize` (CONTRACT §1) and the 8 MiB stack.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Shadow stack size of the wasm module (bytes).
const STACK_SIZE: usize = 8 * 1024 * 1024;

/// Maximum linear memory of the wasm module (bytes, a multiple of the 64 KiB page size).
const MAX_MEMORY: usize = 512 * 1024 * 1024;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  engine_version();
  reactor_link();
}

/// Engine version: the rheon repository commit (`RHEON_COMMIT`) and the upstream dsntk-rs
/// commit the engine crates are based on (`[package.metadata.rheon] upstream-commit` in this
/// crate's `Cargo.toml`, `RHEON_UPSTREAM_COMMIT`).
///
/// The repository commit comes from, in this order: `RHEON_COMMIT` in the environment (source
/// archives, CI without a checkout); `.cargo_vcs_info.json`, which cargo writes into every
/// packaged crate (so the crates.io build reports the commit it was packaged from); and
/// `git rev-parse HEAD`, with `-dirty` appended when the work tree has uncommitted changes.
/// Without any of them the value is `unknown` and a warning is printed.
fn engine_version() {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let manifest = crate_dir.join("Cargo.toml");
  println!("cargo:rerun-if-changed={}", manifest.display());
  let text = std::fs::read_to_string(&manifest)
    .unwrap_or_else(|e| panic!("feel-wasm: cannot read {}: {e}", manifest.display()));
  let upstream = text
    .lines()
    .map(str::trim)
    .filter(|l| !l.starts_with('#'))
    .find_map(|l| quoted_value(l, "upstream-commit"))
    .unwrap_or_else(|| panic!("feel-wasm: `[package.metadata.rheon] upstream-commit` missing in {}", manifest.display()));
  if !is_sha(&upstream) {
    panic!("feel-wasm: `upstream-commit` in {} is not a full 40-digit commit SHA: {upstream:?}", manifest.display());
  }

  println!("cargo:rerun-if-env-changed=RHEON_COMMIT");
  let vcs_info = crate_dir.join(".cargo_vcs_info.json");
  println!("cargo:rerun-if-changed={}", vcs_info.display());
  let commit = match std::env::var("RHEON_COMMIT") {
    Ok(c) if !c.trim().is_empty() => c.trim().to_string(),
    _ => packaged_sha(&vcs_info).or_else(|| git_head(&crate_dir)).unwrap_or_else(|| {
      println!("cargo:warning=feel-wasm: not a git checkout or packaged crate and RHEON_COMMIT is unset; `info` will report engineCommit \"unknown\"");
      "unknown".to_string()
    }),
  };
  println!("cargo:rustc-env=RHEON_COMMIT={commit}");
  println!("cargo:rustc-env=RHEON_UPSTREAM_COMMIT={upstream}");
}

fn is_sha(s: &str) -> bool {
  s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The commit recorded in `.cargo_vcs_info.json` (`{"git": {"sha1": "…", "dirty": true?}, …}`),
/// which exists only in a crate packaged by cargo; `-dirty` when cargo marked it so.
fn packaged_sha(path: &Path) -> Option<String> {
  let text = std::fs::read_to_string(path).ok()?;
  let after = &text[text.find("\"sha1\"")? + 6..];
  let after = after.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
  let sha = &after[..after.find('"')?];
  if !is_sha(sha) {
    return None;
  }
  let dirty = text.contains("\"dirty\": true") || text.contains("\"dirty\":true");
  Some(if dirty { format!("{sha}-dirty") } else { sha.to_string() })
}

/// `<HEAD sha>[-dirty]` of the git repository containing `dir`, or `None` when git or the checkout is missing.
fn git_head(dir: &Path) -> Option<String> {
  let git = |args: &[&str]| {
    Command::new("git")
      .arg("-C")
      .arg(dir)
      .args(args)
      .output()
      .ok()
      .filter(|o| o.status.success())
      .and_then(|o| String::from_utf8(o.stdout).ok())
  };
  // rebuild when HEAD moves (commit, checkout) — the ref file is resolved through `git rev-parse`
  if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
    let git_dir = PathBuf::from(git_dir.trim());
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    if let Some(r) = git(&["symbolic-ref", "-q", "HEAD"]) {
      println!("cargo:rerun-if-changed={}", git_dir.join(r.trim()).display());
    }
    println!("cargo:rerun-if-changed={}", git_dir.join("packed-refs").display());
  }
  let head = git(&["rev-parse", "HEAD"])?.trim().to_string();
  if head.len() != 40 {
    return None;
  }
  let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).map(|s| !s.trim().is_empty()).unwrap_or(false);
  Some(if dirty { format!("{head}-dirty") } else { head })
}

/// `key = "value"` → `value` (the first double-quoted string after `key`).
fn quoted_value(s: &str, key: &str) -> Option<String> {
  let rest = s.strip_prefix(key)?.trim_start().strip_prefix('=')?.trim_start();
  let rest = rest.strip_prefix('"')?;
  let end = rest.find('"')?;
  Some(rest[..end].to_string())
}

fn reactor_link() {
  // the sysroot (and so crt1-reactor.o) depends on the compiler in use
  println!("cargo:rerun-if-env-changed=RUSTC");
  println!("cargo:rerun-if-env-changed=RUSTUP_TOOLCHAIN");
  let target = std::env::var("TARGET").unwrap_or_default();
  if target != "wasm32-wasip1" {
    return;
  }
  // the contract tests (run under wasmtime) get the same shadow stack as the module
  println!("cargo:rustc-link-arg-tests=-zstack-size={STACK_SIZE}");
  let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
  let sysroot = Command::new(&rustc)
    .args(["--print", "sysroot"])
    .output()
    .ok()
    .filter(|o| o.status.success())
    .and_then(|o| String::from_utf8(o.stdout).ok())
    .map(|s| PathBuf::from(s.trim()));
  let Some(sysroot) = sysroot else {
    panic!("feel-wasm: could not determine the sysroot of {rustc} (`{rustc} --print sysroot` failed)");
  };
  let crt = sysroot.join("lib/rustlib/wasm32-wasip1/lib/self-contained/crt1-reactor.o");
  if !crt.exists() {
    panic!(
      "feel-wasm: {} not found; it is needed to build feel.wasm as a reactor (CONTRACT §1). \
       Install the target with `rustup target add wasm32-wasip1`.",
      crt.display()
    );
  }
  println!("cargo:rerun-if-changed={}", crt.display());
  println!("cargo:rustc-link-arg-cdylib={}", crt.display());
  println!("cargo:rustc-link-arg-cdylib=--export=_initialize");
  println!("cargo:rustc-link-arg-cdylib=-zstack-size={STACK_SIZE}");
  println!("cargo:rustc-link-arg-cdylib=--max-memory={MAX_MEMORY}");
}
