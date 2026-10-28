import type { Dialect } from '../engine';

export interface Example {
  id: string;
  title: string;
  blurb: string;
  dialect: Dialect;
  expression: string;
  /** JSON text for the context (kept as text so numbers keep their exact digits). */
  context: string;
  /** JSON text for the unary-tests input value `?`. */
  input?: string;
}

export const EXAMPLES: Example[] = [
  {
    id: 'decimals',
    title: 'Exact decimals',
    blurb: '34-digit decimal arithmetic: 0.1 + 0.2 really is 0.3',
    dialect: 'expression',
    expression: `{
  one third: 1 / 3,
  exact: 0.1 + 0.2 = 0.3,
  big: 12345678901234567890.123456789 * 3,
  root two: sqrt(2),
  huge: 10 ** 30 + 1
}`,
    context: '{}',
  },
  {
    id: 'salary',
    title: 'Names with spaces',
    blurb: 'Context keys with spaces become FEEL names',
    dialect: 'expression',
    expression: `{
  yearly: Monthly Salary * 12,
  taxed: decimal(Monthly Salary * 12 * (1 - Tax Rate), 2)
}`,
    context: `{
  "Monthly Salary": 10000.25,
  "Tax Rate": 0.19
}`,
  },
  {
    id: 'strings',
    title: 'String functions',
    blurb: 'substring, upper case, string join, matches',
    dialect: 'expression',
    expression: `{
  shout: upper case(substring("dsntk feel playground", 7, 4)),
  joined: string join(["a", "b", "c"], "-"),
  ok: matches("FEEL-1.5", "^[A-Z]+-\\\\d\\\\.\\\\d$"),
  words: split("decision model and notation", " ")
}`,
    context: '{}',
  },
  {
    id: 'lists',
    title: 'List functions',
    blurb: 'sort with a lambda, sum, mean, distinct values',
    dialect: 'expression',
    expression: `{
  sorted: sort(numbers, function(x, y) x < y),
  total: sum(numbers),
  avg: mean(numbers),
  distinct: distinct values(numbers)
}`,
    context: `{
  "numbers": [5, 3.5, 9, 1, 3.5]
}`,
  },
  {
    id: 'contexts',
    title: 'Context functions',
    blurb: 'context put, get value, get entries',
    dialect: 'expression',
    expression: `{
  person: context put(person, "born", 1815),
  name: get value(person, "name"),
  entries: get entries(person)
}`,
    context: `{
  "person": { "name": "Ada", "title": "Countess" }
}`,
  },
  {
    id: 'iteration',
    title: 'for / some / every',
    blurb: 'Iteration and quantified expressions',
    dialect: 'expression',
    expression: `{
  squares: for i in 1..5 return i * i,
  any big: some x in scores satisfies x > 10,
  all positive: every x in scores satisfies x > 0
}`,
    context: `{
  "scores": [3, 8, 12]
}`,
  },
  {
    id: 'filters',
    title: 'Filters & paths',
    blurb: 'Filter a list of contexts, project a field',
    dialect: 'expression',
    expression: `{
  big spenders: orders[amount > 100].customer,
  first: orders[1],
  total: sum(orders.amount)
}`,
    context: `{
  "orders": [
    { "customer": "Ada", "amount": 250.50 },
    { "customer": "Alan", "amount": 99.99 },
    { "customer": "Grace", "amount": 1200 }
  ]
}`,
  },
  {
    id: 'temporal',
    title: 'Temporal arithmetic',
    blurb: 'Dates, date-times and both duration kinds',
    dialect: 'expression',
    expression: `{
  next month: date("2026-10-16") + duration("P1M"),
  age: years and months duration(date("1990-05-17"), date("2026-10-16")),
  until xmas: date and time("2026-12-24T18:00:00") - date and time("2026-10-16T09:30:00"),
  weekday: day of week(date("2026-10-16"))
}`,
    context: '{}',
  },
  {
    id: 'ranges',
    title: 'Ranges',
    blurb: 'Range membership and the range built-ins',
    dialect: 'expression',
    expression: `{
  contains five: 5 in [1..10],
  before: before(1, [2..5]),
  overlaps: overlaps([1..5], [3..8]),
  includes: includes([1..10], 10)
}`,
    context: '{}',
  },
  {
    id: 'recursion',
    title: 'Recursive function',
    blurb: '30! computed exactly with a context-local function',
    dialect: 'expression',
    expression: `{
  fact: function(n) if n <= 1 then 1 else n * fact(n - 1),
  result: fact(30)
}.result`,
    context: '{}',
  },
  {
    id: 'null',
    title: 'Null with a reason',
    blurb: 'FEEL yields null instead of failing; dsntk says why',
    dialect: 'expression',
    expression: `"total: " + 42`,
    context: '{}',
  },
  {
    id: 'unary-age',
    title: 'Unary tests: age band',
    blurb: 'Decision-table style input entry, tested against ?',
    dialect: 'unaryTests',
    expression: `[18..65)`,
    context: '{}',
    input: '42',
  },
  {
    id: 'unary-not',
    title: 'Unary tests: negation',
    blurb: 'not(...) passes when no test matches',
    dialect: 'unaryTests',
    expression: `not("red", "green")`,
    context: '{}',
    input: '"blue"',
  },
  {
    id: 'runaway',
    title: 'Runaway loop (timeout)',
    blurb: '400M iterations: runs past the 2s budget; the engine is restarted',
    dialect: 'expression',
    expression: `count(for i in 1..20000 return count(for j in 1..20000 return j))`,
    context: '{}',
  },
];
