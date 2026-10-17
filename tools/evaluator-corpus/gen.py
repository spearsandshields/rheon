"""Generated supplemental FEEL corpus: numeric builtins/operators over edge-case operands."""
import json, sys, itertools
V = ["0", "-0", "0.0", "1", "-1", "0.1", "0.5", "2", "3", "7", "10", "1/3", "2/3", "-2/3", "1.5", "-2.5", "2.5", "50.50",
     "12.2384283", "3.14159265358979323846264338327950288", "123456789012345678901234567890.123456789", "1E+10", "1E-10",
     "9.999999999999999999999999999999999E+6144", "1E-6143", "1E-6176", "0.001", "1000000", "99999999999999999999999999999999999",
     "-123.456", "1.0000000000000000000000000000000001", "0.9999999999999999999999999999999999", "1.23E+34", "4", "16", "1E+100"]
U = ["exp({})", "log({})", "sqrt({})", "abs({})", "floor({})", "ceiling({})", "odd({})", "even({})", "string({})",
     "number(string({}))", "-({})", "({})*({})", "({})/7", "({})+1", "({})-0.1", "decimal({},2)", "decimal({},0)", "decimal({},-2)",
     "decimal({},33)", "round up({},1)", "round down({},1)", "round half up({},0)", "round half down({},0)", "floor({},2)",
     "ceiling({},-1)", "string(decimal({},5))", "is({}, 1)", "{} instance of number", "string(exp({}))", "string(log({}))",
     "string(sqrt({}))", "({}) ** 2", "({}) ** -1", "({}) ** 0.5", "({}) ** 1.5", "({}) ** 4.25", "({}) ** 10", "({}) ** -2.5",
     "({}) ** 0.1", "({}) ** 33", "({}) ** 3", "exp(log({}))", "log(exp({}))", "sqrt({})*sqrt({})",
     "duration(\"P1D\") * ({})", "duration(\"PT1.5S\") * ({})", "duration(\"P1Y2M\") * ({})", "duration(\"PT1H\") / ({})",
     "date(\"2020-01-01\") + duration(\"P1D\") * ({})"]
B = ["({a})+({b})", "({a})-({b})", "({a})*({b})", "({a})/({b})", "modulo({a},{b})", "({a})**({b})", "({a})<({b})", "({a})=({b})",
     "max({a},{b})", "min({a},{b})", "mean({a},{b})", "stddev({a},{b},1)", "product({a},{b})", "sum([{a},{b}])", "median({a},{b},5)",
     "decimal(({a})/({b}),4)", "string(({a})/({b}))"]
out = []
for u in U:
    for v in V:
        out.append(u.replace("{}", v))
small = ["0", "-0", "1", "-1", "0.1", "2", "3", "7", "1/3", "-2.5", "2.5", "12.2384283", "1E+10", "1E-10", "9.999999999999999999999999999999999E+6144", "1E-6143", "123456789012345678901234567890.123456789", "-123.456", "0.5", "4.25"]
for b in B:
    for a, c in itertools.product(small, small):
        out.append(b.format(a=a, b=c))
extra = ["sum(for i in 1..100 return 1/i)", "product(for i in 1..30 return i)", "product(for i in 1..40 return i)",
 "sum(for i in 1..50 return i ** 0.5)", "stddev(for i in 1..20 return i/3)", "mean(for i in 1..77 return 1/i)",
 "for i in 1..20 return exp(i/7)", "for i in 1..20 return log(i*1.1)", "for i in 1..20 return sqrt(i)", "for i in 1..20 return 1.1 ** i",
 "for i in 1..20 return 1.01 ** (i/3)", "for i in -10..10 return decimal(i/4, 0)", "for i in -10..10 return round half up(i/4, 0)",
 "for i in -10..10 return round half down(i/4, 0)", "for i in -10..10 return modulo(i, 3.3)", "for i in 1..30 return 2 ** -i",
 "for i in 1..30 return 10 ** (i*200)", "for i in 1..30 return 10 ** (-i*200)", "for i in 0..30 return exp(i*10)",
 "for i in 1..30 return 0.1 ** (i*10)", "date and time(\"2020-01-01T10:00:00.123456789\") + duration(\"PT0.000000001S\") * 7",
 "duration(\"PT1.123456789S\") * 3", "duration(\"P1DT2.5S\") / 3", "time(10, 20, 30.5) + duration(\"PT0.25S\")", "time(10.5, 20, 30)",
 "years and months duration(date(\"2020-01-31\"), date(\"2021-03-01\"))", "day of year(date(\"2020-12-31\"))", "week of year(date(\"2021-01-03\"))",
 "date(2020, 2, 29) - date(2019, 2, 28)", "string(date and time(\"2020-01-01T00:00:00@Europe/Paris\"))", "number(\"1,000.50\", \",\", \".\")",
 "number(\"1 000,5\", \" \", \",\")", "string length(string(1/3))", "string(1/3)", "string(2/3)", "string(1E+34)", "string(1E-34)",
 "string(0.000001)", "string(1234567890123456789012345678901234567)", "string(-0)", "string(0.00)", "string(1.10)", "string(100)",
 "1E+6144 * 10", "-(1E+6144 * 10)", "1/0", "0/0", "1E-6176 / 2", "1E-6176 / 1E10", "9.999999999999999999999999999999999E+6144 + 1E+6111",
 "decimal(1/3, 2)", "decimal(-2.5, 0)", "decimal(2.5, 0)", "decimal(1.5, 0)", "decimal(-1.5, 0)", "round half even(2.5, 0)", "count([1/3, 2/3])",
 "sort([3/7, 1/3, 2/7, 1E-100, -1E100])", "list contains([1/3], 1/3)", "index of([0.1+0.2, 0.3], 0.3)", "distinct values([1, 1.0, 1.00, 2/2])",
 "mode([1, 1.0, 2, 2.0, 1/3])", "median([1/3, 2/3, 1])", "mean([1/3, 2/3])", "stddev([1/3, 2/3, 3/3])", "stddev([2, 4, 7, 5])",
 "sqrt(-1)", "log(0)", "log(-1)", "exp(100000)", "exp(-100000)", "exp(14000)", "exp(14200)", "exp(-14200)", "exp(-14100)",
 "abs(-1E-6176)", "-1E-6176 * 0.1", "(-8) ** (1/3)", "(-2) ** 3", "(-2) ** -3", "0 ** -1", "0 ** 0", "0 ** 0.5", "(-0) ** 3",
 "2 ** 1E+10", "1.0000001 ** 1E+10", "0.9999999 ** 1E+10", "10 ** 6144", "10 ** 6145", "10 ** -6176", "10 ** -6177",
 "9999 ** 9999", "(-9999) ** 9999", "2 ** 0.5 * 2 ** 0.5", "e", "@\"P1.5D\"", "@\"PT0.1S\" * 10", "@\"2020-01-01T10:00:00.5\" - @\"2020-01-01T09:00:00.25\""]
out += extra
seen=set()
with open(sys.argv[1], 'w') as w:
    for e in out:
        if e in seen: continue
        seen.add(e); w.write(json.dumps({"src": "generated", "ctx": "", "expr": e}) + "\n")
print(len(seen), file=sys.stderr)
