# Hosts outside a JS engine

`dist/feel.wasm` runs in any WASI preview1 runtime that provides the imports of CONTRACT §1
(`npm run stack` runs it in Wasmtime):

- Give wasm at least 4 MiB of stack (Wasmtime: `max_wasm_stack` on the `Config`,
  `-W max-wasm-stack=4194304` on the command line; the default 512 KiB lets a deeply nested
  recursion trap early, CONTRACT §1), and call it on a thread whose native stack is larger than
  that.
- Bound every request with fuel or epoch interruption and cap the store's memory: nothing else
  stops a runaway loop on the calling thread. `instantiateFeel` has neither and is for trusted
  input only.
- An instance is single-threaded: use one per thread or a pool, and discard an instance after any
  trap (fuel, epoch, panic or stack overflow).
- Capture the module's stderr (fd 2) to tell the traps apart: a failed allocation at the 512 MiB
  cap writes `memory allocation of N bytes failed` before it traps, so the request needed more
  memory than the cap allows, while a panic writes its message (CONTRACT §1, §4).
- Call `_initialize` once, then `feel_alloc` → write the UTF-8 request → `feel_call` → unpack
  `(ptr << 32) | len` → read → `feel_free`, as [`packages/feel-runtime/src/instance.ts`](../packages/feel-runtime/src/instance.ts) does.
- Send `tzOffsetMinutes` with each request, and keep decimals as strings or read the `typed`
  result: decimal128 has 34 digits, more than most host decimal types.
