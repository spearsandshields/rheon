#!/usr/bin/env node
// Answers a file of requests with feel.wasm: one JSON request text per line in, the module's
// response text per line out, in the same order; a request that traps the instance answers
// {"trap":true} and the next one runs on a fresh instance. With "-" the answers go to stdout; the
// summary line goes to stderr.
// Usage: node scripts/feel-answers.mjs <module.wasm> <requests.jsonl> <answers.jsonl | ->
import { createReadStream, openSync, writeSync, closeSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { session } from './feel-abi.mjs';

const [modulePath, requestsPath, answersPath, ...rest] = process.argv.slice(2);
if (!answersPath || rest.length > 0) {
  console.error('usage: node scripts/feel-answers.mjs <module.wasm> <requests.jsonl> <answers.jsonl | ->');
  process.exit(2);
}

const answer = await session(await WebAssembly.compile(await readFile(modulePath)));
const out = answersPath === '-' ? null : openSync(answersPath, 'w');
let pending = [];
let count = 0;
let traps = 0;
// stdout may be a pipe: wait for it to drain instead of writing past it.
const flush = async () => {
  const text = pending.join('');
  pending = [];
  if (out !== null) writeSync(out, text);
  else if (!process.stdout.write(text)) await new Promise((resolve) => process.stdout.once('drain', resolve));
};
for await (const line of createInterface({ input: createReadStream(requestsPath), crlfDelay: Infinity })) {
  const response = await answer(line);
  if (typeof response === 'string') {
    pending.push(response, '\n');
  } else {
    pending.push('{"trap":true}\n');
    traps++;
  }
  if (++count % 4096 === 0) await flush();
}
await flush();
if (out !== null) closeSync(out);
console.error(`answers: ${count} requests, ${traps} traps`);
