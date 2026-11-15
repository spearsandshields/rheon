"""Judges the module's answers to the oracle's cases.

  python3 -I judge.py <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>

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


def main(argv):
    usage = 'judge.py <quick|full> [--seed N] <answers.jsonl | -> <known-wrong.json>'
    if len(argv) < 3:
        print(f'usage: {usage}', file=sys.stderr)
        return 2
    size, seed = generator.parse_arguments(argv[:-2], usage)
    answers_path, known_path = argv[-2:]
    known = load_known(known_path)
    alive = [False] * len(known)
    per_family = defaultdict(Counter)
    failing = defaultdict(list)  # (family, verdict) -> example lines
    classes = Counter()
    total = 0
    answers = sys.stdin if answers_path == '-' else open(answers_path)
    for case in generator.cases(size, seed):
        answer_line = answers.readline()
        if not answer_line:
            fail('fewer answers than cases')
        verdict, detail = judge(case['tree'], json.loads(answer_line))
        total += 1
        name = case['family']
        if verdict == 'ok':
            per_family[name]['ok'] += 1
            continue
        classes[(name, verdict)] += 1
        matches = [i for i, e in enumerate(known)
                   if e['verdict'] == verdict and fnmatch.fnmatchcase(name, e['family'])]
        for i in matches:
            alive[i] = alive[i] or case['edge']
        if matches:
            per_family[name]['exempt'] += 1
            continue
        per_family[name]['failing'] += 1
        if len(failing[(name, verdict)]) < EXAMPLES:
            failing[(name, verdict)].append(f'{case["request"]}    # {detail}')
    if answers.readline():
        fail('more answers than cases')

    print(f'{"family":40} {"cases":>7} {"ok":>7} {"exempt":>7} {"failing":>7}')
    for name in sorted(per_family):
        c = per_family[name]
        print(f'{name:40} {sum(c.values()):>7} {c["ok"]:>7} {c["exempt"]:>7} {c["failing"]:>7}')
    if classes:
        print('\nclasses (family, verdict: cases):')
        for (name, verdict), n in sorted(classes.items()):
            print(f'  {name}, {verdict}: {n}')
    status = 0
    for (name, verdict), lines in sorted(failing.items()):
        status = 1
        print(f'\nFAIL {name} {verdict}:')
        print('\n'.join(f'  {line}' for line in lines))
    for entry, used in zip(known, alive):
        if not used:
            status = 1
            print(f'\nSTALE known-wrong entry {entry["family"]} {entry["verdict"]}: no failing edge vector')
    counts = Counter()
    for c in per_family.values():
        counts.update(c)
    stale = alive.count(False)
    print(f'\njudge: {size}, seed {seed}, {len(generator.FAMILIES)} families, {total} cases, {counts["ok"]} right, '
          f'{counts["exempt"]} exempt, {counts["failing"]} failing' + (f', {stale} stale entries' if stale else ''))
    return status


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
