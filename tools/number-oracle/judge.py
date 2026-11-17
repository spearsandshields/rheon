"""Judges the module's answers to the oracle's cases.

  python3 -I judge.py <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>
  python3 -I judge.py --shard I/N --part <out.json> <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>
  python3 -I judge.py --merge <quick|full> [--seed N] <known-wrong.json> <part.json>...

With --shard I/N the answers are those of the cases whose number is I modulo N (counted from 0),
and the counts are written to the part file; --merge prints the report of a run from the parts of
all its shards. A run in shards judges the same cases as a run in one piece.

The cases are drawn again from cases.py with the same size and seed, so the n-th answer is the
answer to the n-th request that cases.py printed.

Verdicts, per case:
  ok                    the truth, in its canonical text in every text form
  value                 a number (or a boolean, a string) that is not the truth; for a number, the
                        distance in units of the last place of the truth is reported
  null-for-number       null where the truth is a value
  number-for-null       a value where the truth is null
  negative-zero         the right value, a zero written with a minus sign
  text                  the right value, a text form that is not the canonical text
  error                 the response is not ok
  trap                  the instance trapped
  undecided             the oracle found no truth (the interval rule ran out of precision)
  oracle-disagreement   two libraries of the oracle disagree

A number result's value is read from result.display; result.feel and result.typed's text are
checked as text forms. A string result is its typed text.

known-wrong.json lists [{"family": <glob>, "verdict": <verdict>, "why": <text>}]: a case that fails
with a listed verdict in a matching family is exempt. An entry that no failing edge vector matches
is stale and fails the run, so the change that repairs a class deletes its entry. Only edge vectors
keep an entry alive: they run in every size, so the quick and the full run agree on staleness.
trap, undecided and oracle-disagreement cannot be listed.

Exit status: 0 every case right or exempt, 1 a failing case or a stale entry, 2 usage or input.
"""
import fnmatch
import json
import os
import sys
from collections import Counter, defaultdict
from decimal import Decimal, InvalidOperation

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cases as generator  # noqa: E402
import oracle  # noqa: E402

VERDICTS = ['ok', 'value', 'null-for-number', 'number-for-null', 'negative-zero', 'text', 'error',
            'trap', 'undecided', 'oracle-disagreement']
NEVER_EXEMPT = {'trap', 'undecided', 'oracle-disagreement'}
EXAMPLES = 3


def ulp_distance(got, truth):
    """|got - truth| in units of the last place of the truth (the subnormal quantum below 1E-6143)."""
    exponent = max(truth.adjusted() - 33, -6176) if truth != 0 else -6176
    distance = oracle.WIDE.subtract(got, truth).copy_abs().scaleb(-exponent, oracle.WIDE)
    return f'{distance:.3g}' if distance >= 1000 else str(distance)


def text_verdict(texts, truth_text):
    """The verdict of the text forms of a right value."""
    if all(t == truth_text for t in texts):
        return 'ok', ''
    wrong = next(t for t in texts if t != truth_text)
    return ('negative-zero' if truth_text == '0' and wrong.startswith('-') else 'text'), f'text {wrong[:60]!r}'


def judge(tree, answer):
    """(verdict, detail) of one answer."""
    try:
        truth = oracle.evaluate(tree)
    except oracle.Undecided as e:
        return 'undecided', str(e)
    except oracle.Disagreement as e:
        return 'oracle-disagreement', str(e)
    if answer.get('trap') is True:
        return 'trap', ''
    if answer.get('ok') is not True:
        return 'error', json.dumps(answer.get('error'))[:120]
    result = answer['result']
    if truth is None:
        return ('ok', '') if result.get('isNull') else ('number-for-null', f'got {result.get("display", "")[:60]}')
    if result.get('isNull'):
        return 'null-for-number', result.get('nullReason', '')[:120]
    simple = (result.get('typed') or {}).get('simple') or {}
    if isinstance(truth, bool):
        got = result.get('display')
        return ('ok', '') if got == ('true' if truth else 'false') else ('value', f'got {got}')
    if isinstance(truth, str):
        got = simple.get('text')
        if result.get('type') != 'string' or not isinstance(got, str):
            return 'value', f'got {result.get("display", "")[:60]}'
        if got == truth:
            return 'ok', ''
        try:
            same_value = Decimal(got) == Decimal(truth)
        except InvalidOperation:
            same_value = False
        return text_verdict([got], truth) if same_value else ('value', f'got {got[:60]!r}')
    if result.get('type') != 'number':
        return 'value', f'got {result.get("type")}'
    display = result.get('display', '')
    try:
        got = Decimal(display)
    except InvalidOperation:
        return 'value', f'got {display[:60]!r}'
    if got != truth:
        return 'value', f'{ulp_distance(got, truth)} ulp, got {str(got)[:45]}, truth {truth}'
    return text_verdict([display, result.get('feel', ''), simple.get('text') or ''], oracle.canonical(truth))


def fail(message):
    """An input error: exit status 2."""
    print(f'judge: {message}', file=sys.stderr)
    sys.exit(2)


def load_known(path):
    try:
        with open(path) as f:
            entries = json.load(f)
    except (OSError, ValueError) as e:
        fail(f'{path}: {e}')
    if not isinstance(entries, list):
        fail(f'{path}: not a list')
    for i, entry in enumerate(entries):
        if not isinstance(entry, dict) or set(entry) != {'family', 'verdict', 'why'}:
            fail(f'{path}: entry {i} needs exactly family, verdict and why')
        if entry['verdict'] not in VERDICTS or entry['verdict'] in NEVER_EXEMPT or entry['verdict'] == 'ok':
            fail(f'{path}: entry {i}: {entry["verdict"]!r} cannot be listed')
        if not all(isinstance(entry[k], str) and entry[k] for k in entry):
            fail(f'{path}: entry {i}: empty or non-text field')
    return entries


def collect(size, seed, answers, known, shard=None):
    """Judges the answers; with shard (i, n) only the cases whose number is i modulo n, whose
    answers alone the answers hold. Returns the counts as plain data (see report)."""
    alive = [False] * len(known)
    per_family = defaultdict(Counter)
    failing = defaultdict(list)  # 'family\tverdict' -> example lines
    classes = Counter()
    total = 0
    for number, case in enumerate(generator.cases(size, seed)):
        if shard is not None and number % shard[1] != shard[0]:
            continue
        answer_line = answers.readline()
        if not answer_line:
            fail('fewer answers than cases')
        verdict, detail = judge(case['tree'], json.loads(answer_line))
        total += 1
        name = case['family']
        if verdict == 'ok':
            per_family[name]['ok'] += 1
            continue
        key = f'{name}\t{verdict}'
        classes[key] += 1
        matches = [i for i, e in enumerate(known)
                   if e['verdict'] == verdict and fnmatch.fnmatchcase(name, e['family'])]
        for i in matches:
            alive[i] = alive[i] or case['edge']
        if matches:
            per_family[name]['exempt'] += 1
            continue
        per_family[name]['failing'] += 1
        if len(failing[key]) < EXAMPLES:
            failing[key].append(f'{case["request"]}    # {detail}')
    if answers.readline():
        fail('more answers than cases')
    return {'total': total, 'alive': alive, 'per_family': {name: dict(c) for name, c in per_family.items()},
            'failing': dict(failing), 'classes': dict(classes)}


def merge(parts):
    """The counts of the shards of one run as the counts of the whole run."""
    whole = {'total': 0, 'alive': None, 'per_family': defaultdict(Counter), 'failing': defaultdict(list), 'classes': Counter()}
    for part in parts:
        whole['total'] += part['total']
        whole['alive'] = part['alive'] if whole['alive'] is None else [a or b for a, b in zip(whole['alive'], part['alive'])]
        for name, counts in part['per_family'].items():
            whole['per_family'][name].update(counts)
        whole['classes'].update(part['classes'])
        for key, lines in part['failing'].items():
            whole['failing'][key] = (whole['failing'][key] + lines)[:EXAMPLES]
    return whole


def report(size, seed, known, state):
    """Prints the table, the failing classes and the stale entries; returns the exit status."""
    per_family = {name: Counter(c) for name, c in state['per_family'].items()}
    print(f'{"family":40} {"cases":>7} {"ok":>7} {"exempt":>7} {"failing":>7}')
    for name in sorted(per_family):
        c = per_family[name]
        print(f'{name:40} {sum(c.values()):>7} {c["ok"]:>7} {c["exempt"]:>7} {c["failing"]:>7}')
    if state['classes']:
        print('\nclasses (family, verdict: cases):')
        for key, n in sorted(state['classes'].items()):
            name, verdict = key.split('\t')
            print(f'  {name}, {verdict}: {n}')
    status = 0
    for key, lines in sorted(state['failing'].items()):
        name, verdict = key.split('\t')
        status = 1
        print(f'\nFAIL {name} {verdict}:')
        print('\n'.join(f'  {line}' for line in lines))
    alive = state['alive']
    for entry, used in zip(known, alive):
        if not used:
            status = 1
            print(f'\nSTALE known-wrong entry {entry["family"]} {entry["verdict"]}: no failing edge vector')
    counts = Counter()
    for c in per_family.values():
        counts.update(c)
    stale = alive.count(False)
    print(f'\njudge: {size}, seed {seed}, {len(generator.FAMILIES)} families, {state["total"]} cases, {counts["ok"]} right, '
          f'{counts["exempt"]} exempt, {counts["failing"]} failing' + (f', {stale} stale entries' if stale else ''))
    return status


def main(argv):
    usage = ('judge.py <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>\n'
             '       judge.py --shard I/N --part <out.json> <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>\n'
             '       judge.py --merge <quick|full> [--seed N] <known-wrong.json> <part.json>...')
    if argv[:1] == ['--merge']:
        split = 4 if argv[2:3] == ['--seed'] else 2
        if len(argv) < split + 2:
            print(f'usage: {usage}', file=sys.stderr)
            return 2
        size, seed = generator.parse_arguments(argv[1:split], usage)
        known = load_known(argv[split])
        parts = []
        for path in argv[split + 1:]:
            try:
                with open(path) as f:
                    parts.append(json.load(f))
            except (OSError, ValueError) as e:
                fail(f'{path}: {e}')
        if any(len(part['alive']) != len(known) for part in parts):
            fail('a part was judged with another known-wrong.json')
        return report(size, seed, known, merge(parts))
    shard = part_path = None
    if argv[:1] == ['--shard'] and len(argv) > 4 and argv[2] == '--part':
        try:
            i, n = (int(x) for x in argv[1].split('/'))
        except ValueError:
            i, n = 0, 0
        if not 0 <= i < n:
            print(f'usage: {usage}', file=sys.stderr)
            return 2
        shard, part_path, argv = (i, n), argv[3], argv[4:]
    if len(argv) < 3:
        print(f'usage: {usage}', file=sys.stderr)
        return 2
    size, seed = generator.parse_arguments(argv[:-2], usage)
    answers_path, known_path = argv[-2:]
    known = load_known(known_path)
    answers = sys.stdin if answers_path == '-' else open(answers_path)
    state = collect(size, seed, answers, known, shard)
    if part_path is not None:
        with open(part_path, 'w') as f:
            json.dump(state, f)
        return 0
    return report(size, seed, known, state)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
