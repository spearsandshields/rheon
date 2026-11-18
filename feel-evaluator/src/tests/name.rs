use super::*;

#[test]
fn test_name() {
  let scope = &te_scope(
    r#"{
        thing:2.0,
        one two three four:true,
        one and two:null,
        one or two:"found",
        before: {
          after: 1.11,
          "or": {
            after: 1.12
          },
          "and": {
            after: 1.13
          },
          "between": {
            after: 1.14
          },
          next to between: {
            worm: 1.18
          }
        },
        income/loss:1.15,
        per/month/income/loss:1.17,
        a-b:1.19,
        to-be-or-not-to-be:1.20,
        that's:1.21,
        ok that's:1.22,
        bed+breakfast:1.23,
        night+and+day:1.24,
        fr**n*s:1.26,
        wh*t*v*r:1.27}"#,
  );
  te_number(false, scope, "thing", 20, 1);
  te_bool(false, scope, "one two three four", true);
  te_null(false, scope, "one and two", "");
  te_string(false, scope, "one or two", "found");
  te_number(false, scope, "before.after", 111, 2);
  te_number(false, scope, "before . next to between . worm", 118, 2);
  te_number(false, scope, " before . after ", 111, 2);
  // an entry whose key is a keyword is no name (DMN 1.6 clause 10.3.1.4): it is read by its key
  te_number(false, scope, r#"get value(before, "and").after"#, 113, 2);
  te_number(false, scope, r#"get value(before, "or").after"#, 112, 2);
  te_number(false, scope, "income/loss", 115, 2);
  te_number(false, scope, "income  /   loss", 115, 2);
  te_number(false, scope, "per/month/income/loss", 117, 2);
  te_number(false, scope, " per/ month / income/loss", 117, 2);
  te_number(false, scope, "a-b", 119, 2);
  te_number(false, scope, "to-be-or-not-to-be", 120, 2);
  te_number(false, scope, "that's", 121, 2);
  te_number(false, scope, "ok that's", 122, 2);
  te_number(false, scope, "bed+breakfast", 123, 2);
  te_number(false, scope, "night+and+day", 124, 2);
  te_number(false, scope, "night + and + day", 124, 2);
  te_number(false, scope, "fr**n*s", 126, 2);
  te_number(false, scope, "wh*t*v*r", 127, 2);
}

/// A name is matched against the keys of the scopes it stands in. The keys below them (of the
/// values of the input, and of the context literals of the text) are names only for a member of
/// a value: after a dot and in the predicate of a filter.
#[test]
fn nested_keys_are_names_only_for_a_member_of_a_value() {
  let scope = &te_scope(r#"{x: {a-b: 1}, l: [{a-b: 1}, {a-b: 2}], a: 5, b: 2}"#);
  // alone it is the subtraction of the two names in scope
  te_number(false, scope, "a-b", 3, 0);
  te_number(false, scope, "a-b * 2", 1, 0);
  // after a dot and in a filter it is the entry
  te_number(false, scope, "x.a-b", 1, 0);
  te_number(false, scope, "x.a-b + a-b", 4, 0);
  te_number(false, scope, "sum(l.a-b)", 3, 0);
  te_number(false, scope, "count(l[a-b = 1])", 1, 0);
  te_number(false, scope, "count(l[item.a-b > 0])", 2, 0);
  te_number(false, scope, "sum(for i in l return i.a-b)", 3, 0);
  // the key of a context literal is a name in the literal, and a member name after it
  te_number(false, scope, "{a-b: 1, r: a-b}.r", 1, 0);
  te_number(false, scope, "{a-b: 1, r: {q: a-b}}.r.q", 1, 0);
  te_number(false, scope, "{a-b: 1}.a-b", 1, 0);
  te_number(false, scope, "{p: {a-b: 1}, r: p.a-b}.r", 1, 0);
  te_number(false, scope, "count([{a-b: 1}][a-b = 1])", 1, 0);
  // but no name after the literal, or beside it
  te_number(false, scope, "[{a-b: 1}, a-b][2]", 3, 0);
  te_number(false, scope, "{p: {a-b: 1}, r: a-b}.r", 3, 0);
  // a key of the input itself is a name everywhere
  let scope = &te_scope(r#"{a-b: 9, a: 5, b: 2}"#);
  te_number(false, scope, "a-b", 9, 0);
  te_number(false, scope, "[a-b, {r: a-b}.r][2]", 9, 0);
}
