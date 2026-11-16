// Summarises a TCK run (scripts/tck.sh): totals for the official TCK and for tck-addendum/,
// failures grouped by test folder, and the difference against the baseline, the list of failing
// tests at the last recorded run (docs/tck-baseline.txt).
//
//   node scripts/tck-summary.mjs <tck_results.csv> [--baseline <file>] [--partial | --update-baseline]
//
// Exit status: 0 when the run equals the baseline; 1 when a test is newly failing or newly
// passing, when a baseline test has no row (it did not run), or when the result file has no rows;
// 2 on a usage error or a missing file. --partial compares only the tests that ran, for a run of
// a part of the TCK. --update-baseline rewrites the baseline with the failing tests of this run
// and exits 0.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

class UsageError extends Error {}
const fail = (message) => {
  throw new UsageError(message);
};

function main(args) {
  let csv = null;
  let baselinePath = fileURLToPath(new URL('../docs/tck-baseline.txt', import.meta.url));
  let partial = false;
  let update = false;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--baseline') baselinePath = args[++i] ?? fail('--baseline needs a file');
    else if (args[i] === '--partial') partial = true;
    else if (args[i] === '--update-baseline') update = true;
    else if (args[i].startsWith('--') || csv !== null) fail(`unexpected argument ${args[i]}`);
    else csv = args[i];
  }
  if (csv === null) fail('the result file is required');
  if (partial && update) fail('a partial run cannot update the baseline');
  if (!existsSync(csv)) fail(`no result file ${csv}`);

  const parse = (line) => [...line.matchAll(/"((?:[^"]|"")*)"/g)].map((m) => m[1].replaceAll('""', '"'));
  const rows = readFileSync(csv, 'utf8').trim().split('\n').map(parse).filter((r) => r.length >= 4);

  const key = (r) => `${r[0]} ${r[1]} ${r[2]}`;
  const isAddendum = (r) => r[0].split('/')[1]?.startsWith('rheon-');
  const stats = { official: { pass: 0, fail: 0 }, addendum: { pass: 0, fail: 0 } };
  const failing = new Map();
  const ran = new Set();
  for (const r of rows) {
    ran.add(key(r));
    const s = stats[isAddendum(r) ? 'addendum' : 'official'];
    if (r[3] === 'SUCCESS') s.pass++;
    else {
      s.fail++;
      failing.set(key(r), r[4] ?? '');
    }
  }
  for (const [name, s] of Object.entries(stats)) console.log(`${name.padEnd(9)} ${String(s.pass).padStart(5)} passed  ${String(s.fail).padStart(4)} failed  of ${s.pass + s.fail}`);

  const byFolder = new Map();
  for (const k of failing.keys()) {
    const f = k.split(' ')[0];
    byFolder.set(f, (byFolder.get(f) ?? 0) + 1);
  }
  if (byFolder.size) {
    console.log('\nfailures by folder:');
    for (const [f, n] of [...byFolder].sort()) console.log(`  ${String(n).padStart(3)}  ${f}`);
  }

  if (update) {
    const header = '# Failing TCK tests at the last recorded run of scripts/tck.sh (official + tck-addendum).\n# Regenerate: node scripts/tck-summary.mjs .tck/out/tck_results.csv --update-baseline\n';
    writeFileSync(baselinePath, header + [...failing.keys()].sort().join('\n') + '\n');
    console.log(`\nwrote ${baselinePath} (${failing.size} failing tests)`);
    return 0;
  }
  if (!existsSync(baselinePath)) fail(`no baseline ${baselinePath}`);
  if (rows.length === 0) {
    console.log('\nthe result file has no rows: the run did not complete');
    return 1;
  }
  const base = new Set(readFileSync(baselinePath, 'utf8').split('\n').filter((l) => l && !l.startsWith('#')));
  const newly = [...failing.keys()].filter((k) => !base.has(k)).sort();
  const fixed = [...base].filter((k) => ran.has(k) && !failing.has(k)).sort();
  const notRun = partial ? [] : [...base].filter((k) => !ran.has(k)).sort();
  console.log(`\nagainst the baseline: ${newly.length} newly failing, ${fixed.length} newly passing${partial ? '' : `, ${notRun.length} not run`}`);
  for (const k of newly) console.log(`  NEW FAIL  ${k}  ${failing.get(k).slice(0, 120)}`);
  for (const k of fixed) console.log(`  FIXED     ${k}`);
  for (const k of notRun) console.log(`  NOT RUN   ${k}`);
  return newly.length + fixed.length + notRun.length ? 1 : 0;
}

// exitCode rather than exit(): output to a pipe is written asynchronously on some platforms
try {
  process.exitCode = main(process.argv.slice(2));
} catch (e) {
  if (!(e instanceof UsageError)) throw e;
  console.error(`tck-summary: ${e.message}\nusage: node scripts/tck-summary.mjs <tck_results.csv> [--baseline <file>] [--partial | --update-baseline]`);
  process.exitCode = 2;
}
