use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let scope = &te_scope(r#"{A:[1,3,5]}"#);
  te_null(false, scope, "A[0]", "index in filter is out of range [1..3], actual index is 0");
  te_null(false, scope, "A[4]", "index in filter is out of range [1..3], actual index is 4");
  te_null(false, scope, "A[-4]", "index in filter is out of range [-3..-1], actual index is -4");
}

#[test]
fn _0002() {
  let scope = &te_scope(r#"{A:[1,3,5]}"#);
  te_number(false, scope, "A[1]", 1, 0);
  te_number(false, scope, "A[-3]", 1, 0);
}

#[test]
fn _0003() {
  let scope = &te_scope(r#"{A:[]}"#);
  te_null(false, scope, "A[1]", "");
}

#[test]
fn _0004() {
  let scope = &te_scope(r#"{A:[]}"#);
  te_null(false, scope, "A[2]", "");
}

#[test]
fn _0005() {
  let scope = &te_scope(r#"{A:[]}"#);
  te_null(false, scope, "A[-1]", "");
}

#[test]
fn _0006() {
  let scope = &te_scope(r#"{A:[]}"#);
  te_null(false, scope, "A[-2]", "");
}

#[test]
fn _0007() {
  let scope = &te_scope(r#"{A:[1,3,5]}"#);
  te_number(false, scope, "A[2]", 3, 0);
  te_number(false, scope, "A[-2]", 3, 0);
}

#[test]
fn _0008() {
  let scope = &te_scope(r#"{A:[1,3,5]}"#);
  te_number(false, scope, "A[3]", 5, 0);
  te_number(false, scope, "A[-1]", 5, 0);
}

#[test]
fn _0009() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_null(false, scope, "B[0]", "index in filter is out of range [1..5], actual index is 0");
  te_null(false, scope, "B[6]", "index in filter is out of range [1..5], actual index is 6");
  te_null(false, scope, "B[-6]", "index in filter is out of range [-5..-1], actual index is -6");
}

#[test]
fn _0010() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[1]", "A");
  te_string(false, scope, "B[-5]", "A");
}

#[test]
fn _0011() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[2]", "B");
  te_string(false, scope, "B[-4]", "B");
}

#[test]
fn _0012() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[3]", "C");
  te_string(false, scope, "B[-3]", "C");
}

#[test]
fn _0013() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[4]", "D");
  te_string(false, scope, "B[-2]", "D");
}

#[test]
fn _0014() {
  let scope = &te_scope(r#"{B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[5]", "E");
  te_string(false, scope, "B[-1]", "E");
}

#[test]
fn _0015() {
  let scope = &te_scope(r#"{A:[1,3,5],B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[A[1]]", "A");
}

#[test]
fn _0016() {
  let scope = &te_scope(r#"{A:[1,3,5],B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[A[2]]", "C");
}

#[test]
fn _0017() {
  let scope = &te_scope(r#"{A:[1,3,5],B:["A","B","C","D","E"]}"#);
  te_string(false, scope, "B[A[3]]", "E");
}

#[test]
fn _0018() {
  let scope = &te_scope(r#"{l:[{x:1,y:2},{x:null,y:3}]}"#);
  te_be_value(false, scope, "l[x<2]", "[{x:1,y:2}]");
}

#[test]
fn _0019() {
  te_be_value(false, &scope!(), "[{x:1,y:2},{x:null,y:3}][x<2]", "[{x:1,y:2}]");
}

#[test]
fn _0020() {
  te_be_value(false, &scope!(), "[{x:1,y:2},{x:null,y:3}][y>2]", "[{x:null,y:3}]");
}

#[test]
fn _0021() {
  te_be_value(false, &scope!(), "[1,2,3,4,5,6,7,8,9,10][item>=5]", "[5,6,7,8,9,10]");
}

#[test]
fn _0022() {
  let scope = &te_scope(r#"{l:[{x:1,y:2},{x:2,y:4},{x:3,y:6}]}"#);
  te_be_value(false, scope, "l[2]", "{x:2,y:4}");
}

#[test]
fn _0023() {
  let scope = &te_scope(r#"{l:[{x:1,y:2},{x:2,y:4},{x:3,y:6}]}"#);
  te_be_value(false, scope, "l[x=2]", "[{x:2,y:4}]");
}

#[test]
fn _0024() {
  let scope = &te_scope(r#"{l:[{x:1,y:2},{x:2,y:4},{x:3,y:6}]}"#);
  te_be_value(false, scope, "l[x=2].y", "[4]");
}

#[test]
fn _0025() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][1]", 1, 0);
}

#[test]
fn _0026() {
  te_be_value(false, &scope!(), "[1,2,3,4,5,6][item=4]", r#"[4]"#);
}

#[test]
fn _0027() {
  te_be_value(false, &scope!(), "[1,2,3,4,5,6][item>=4]", "[4,5,6]");
}

#[test]
fn _0028() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][1][1]", 1, 0);
}

#[test]
fn _0029() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][2]", 2, 0);
}

#[test]
fn _0030() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][3]", 3, 0);
}

#[test]
fn _0031() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][4]", 4, 0);
}

#[test]
fn _0032() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][5]", 5, 0);
}

#[test]
fn _0033() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][6]", 6, 0);
}

#[test]
fn _0034() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-1]", 6, 0);
}

#[test]
fn _0035() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-2]", 5, 0);
}

#[test]
fn _0036() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-3]", 4, 0);
}

#[test]
fn _0037() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-4]", 3, 0);
}

#[test]
fn _0038() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-5]", 2, 0);
}

#[test]
fn _0039() {
  te_number(false, &scope!(), "[1,2,3,4,5,6][-6]", 1, 0);
}

#[test]
fn _0040() {
  te_null(false, &scope!(), "[1,2,3,4,5,6][0]", "index in filter is out of range [1..6], actual index is 0");
}

#[test]
fn _0041() {
  te_null(false, &scope!(), "[1,2,3,4,5,6][7]", "index in filter is out of range [1..6], actual index is 7");
}

#[test]
fn _0042() {
  te_null(false, &scope!(), "[1,2,3,4,5,6][-7]", "index in filter is out of range [-6..-1], actual index is -7");
}

#[test]
fn _0043() {
  te_be_value(false, &scope!(), r#" true[true] "#, r#"[true]"#);
}

#[test]
fn _0044() {
  te_be_value(false, &scope!(), r#" true[false] "#, r#"[]"#);
}

#[test]
fn _0045() {
  te_date(false, &scope!(), r#" @"2023-02-06"[1] "#, 2023, 2, 6);
}

#[test]
fn _0046() {
  te_date_time_local(false, &scope!(), r#" @"2023-02-06T10:11:12"[1] "#, (2023, 2, 6), (10, 11, 12, 0));
}

#[test]
fn _0047() {
  te_time(false, &scope!(), r#" @"10:11:12Z"[-1] "#, FeelTime::utc_opt(10, 11, 12, 0).unwrap());
}

#[test]
fn _0048() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"P1DT2H3M15S"[1] "#, "P1DT2H3M15S");
}

#[test]
fn _0049() {
  te_years_and_months_duration_x(false, &scope!(), r#" @"P5Y4M"[1] "#, "P5Y4M");
}

#[test]
fn _0050() {
  be_be_value(false, &scope!(), r#"[{a: 1}, {a: 2}, {a: 3}]"#, r#"[{a:1},{a:2},{a:3}]"#);
}

#[test]
fn _0051() {
  te_be_value(false, &scope!(), r#"[{a: 1}, {a: 2}, {a: 3}][1]"#, r#"{a:1}"#);
}

#[test]
fn _0052() {
  te_be_value(false, &scope!(), r#"[{item: 1}, {item: 2}, {item: 3}][item >= 2]"#, r#"[{item:2},{item:3}]"#);
}

#[test]
fn _0053() {
  te_be_value(false, &scope!(), r#"[{a: 1}, {a: 2}, {a: 3}][item.a=2]"#, r#"[{a:2}]"#);
}

#[test]
fn _0054() {
  te_be_value(false, &scope!(), r#"[{a: 1}, {a: 2}, {a: 3}][item.a >= 2]"#, r#"[{a:2},{a:3}]"#);
}

#[test]
fn _0055() {
  let scope = &te_scope(
    r#"{
    DeptTable:    [ {manager:"Smith",name:"Sales",number:10},
                    {manager:"Jones",name:"Finance",number:20},
                    {manager:"King",name:"Engineering",number:30} ],
    EmployeeTable:[ {deptNum:10,id:"7792",name:"Clark"},
                    {deptNum:10,id:"7934",name:"Miller"},
                    {deptNum:20,id:"7976",name:"Adams"},
                    {deptNum:20,id:"7902",name:"Ford"},
                    {deptNum:30,id:"7900",name:"James"} ],
    LastName:"Clark"}"#,
  );
  te_string(false, scope, "DeptTable[number=EmployeeTable[name=LastName].deptNum[1]].manager[1]", "Smith");
}

#[test]
fn _0056() {
  let scope = &te_scope(
    r#"
    {
       EmployeeTable:[ 
         {deptNum:10,id:"7792",name:"Clark"},
         {deptNum:10,id:"7934",name:"Miller"},
         {deptNum:20,id:"7976",name:"Adams"},
         {deptNum:20,id:"7902",name:"Ford"},
         {deptNum:30,id:"7900",name:"James"} 
       ],
       LastName:"Clark"
    }
    "#,
  );
  te_number(false, scope, "EmployeeTable[name=LastName].deptNum[1]", 10, 0);
}

#[test]
fn _0057() {
  let scope = &te_scope(
    r#"
    {
       EmployeeTable:[ 
         {deptNum:10,id:"7792",name:"Clark"},
         {deptNum:10,id:"7934",name:"Miller"},
         {deptNum:20,id:"7976",name:"Adams"},
         {deptNum:20,id:"7902",name:"Ford"},
         {deptNum:30,id:"7900",name:"James"} 
       ],
       LastName:"Clark"
    }
    "#,
  );
  te_number(false, scope, "EmployeeTable[name=LastName].deptNum[1]", 10, 0);
}

#[test]
fn _0058() {
  let scope = &te_scope(
    r#"{
    DeptTable:    [ {manager:"Smith",name:"Sales",number:10},
                    {manager:"Jones",name:"Finance",number:20},
                    {manager:"King",name:"Engineering",number:30} ],
    EmployeeTable:[ {deptNum:10,id:"7792",name:"Clark"},
                    {deptNum:10,id:"7934",name:"Miller"},
                    {deptNum:20,id:"7976",name:"Adams"},
                    {deptNum:20,id:"7902",name:"Ford"},
                    {deptNum:30,id:"7900",name:"James"} ],
    LastName:"Clark"}"#,
  );
  te_string(false, scope, "DeptTable[number=10].manager[1]", "Smith");
}

#[test]
fn _0059() {
  let scope = &te_scope(
    r#"{
    DeptTable:    [ {manager:"Smith",name:"Sales",number:10},
                    {manager:"Jones",name:"Finance",number:20},
                    {manager:"King",name:"Engineering",number:30} ],
    EmployeeTable:[ {deptNum:10,id:"7792",name:"Clark"},
                    {deptNum:10,id:"7934",name:"Miller"},
                    {deptNum:20,id:"7976",name:"Adams"},
                    {deptNum:20,id:"7902",name:"Ford"},
                    {deptNum:30,id:"7900",name:"James"} ],
    LastName:"Clark"}"#,
  );
  te_be_value(false, scope, "DeptTable[number=10]", r#"[{manager:"Smith",name:"Sales",number:10}]"#);
}

#[test]
fn _0060() {
  let scope = &te_scope(
    r#"{
     check_eq: function(x,y) x = y,
     check_neq: function(x,y) x != y,
     num: 1 
   }"#,
  );
  te_be_value(false, scope, "[1,2,3,4,5,6][check_eq(item,4)]", r#"[4]"#);
  te_be_value(false, scope, "[1,2,3,4,5,6][check_eq(item,5)]", r#"[5]"#);
  te_be_value(false, scope, "[1,2,3,4,5,6][check_eq(item,num)]", r#"[1]"#);
  te_be_value(false, scope, "[1,2,3,4,5,6][check_neq(item,num)]", "[2,3,4,5,6]");
}

#[test]
fn _0061() {
  let scope = &te_scope(
    r#"{
      "Flight List": [
        { "Flight Number": "UA456",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T19:00:00", "Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA123",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T18:00:00", "Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Status": "cancelled" },
        { "Flight Number": "UA789",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T21:00:00", "Arrival": @"2017-01-01T23:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA1001", "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 0, "Status": "scheduled" },
        { "Flight Number": "UA1111", "From": "SFO", "To": "LAX", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 2, "Status": "scheduled" }]
    }"#,
  );
  te_be_value(
    false,
    scope,
    "Flight List[1]",
    r#"{"Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Departure": @"2017-01-01T19:00:00", "Flight Number": "UA456", "From": "SFO", "Status": "scheduled", "To": "SNA"}"#,
  );
}

#[test]
fn _0062() {
  let scope = &te_scope(
    r#"{
      "Flight List": [
        { "Flight Number": "UA456",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T19:00:00", "Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA123",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T18:00:00", "Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Status": "cancelled" },
        { "Flight Number": "UA789",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T21:00:00", "Arrival": @"2017-01-01T23:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA1001", "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 0, "Status": "scheduled" },
        { "Flight Number": "UA1111", "From": "SFO", "To": "LAX", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 2, "Status": "scheduled" }]
    }"#,
  );
  te_be_value(
    false,
    scope,
    r#"Flight List[Status = "cancelled"]"#,
    r#"[{"Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Departure": @"2017-01-01T18:00:00", "Flight Number": "UA123", "From": "SFO", "Status": "cancelled", "To": "SNA"}]"#,
  );
  te_be_value(
    false,
    scope,
    r#"Flight List[Status = "cancelled"][1]"#,
    r#"{"Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Departure": @"2017-01-01T18:00:00", "Flight Number": "UA123", "From": "SFO", "Status": "cancelled", "To": "SNA"}"#,
  );
}

#[test]
fn _0063() {
  let scope = &te_scope(
    r#"{
      "Flight List": [
        { "Flight Number": "UA456",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T19:00:00", "Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA123",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T18:00:00", "Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Status": "cancelled" },
        { "Flight Number": "UA789",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T21:00:00", "Arrival": @"2017-01-01T23:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": "UA1001", "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 0, "Status": "cancelled" },
        { "Flight Number": "UA1111", "From": "SFO", "To": "LAX", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 2, "Status": "scheduled" }]
    }"#,
  );
  te_be_value(false, scope, r#"Flight List[Status = "cancelled"].Flight Number"#, r#"["UA123", "UA1001"]"#);
}

#[test]
fn _0064() {
  let scope = &te_scope(
    r#"{
      "Flight List": [
        { "Flight Number": { n: "UA456",  tag: "A" }, "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T19:00:00", "Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": { n: "UA123",  tag: "B" }, "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T18:00:00", "Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Status": "cancelled" },
        { "Flight Number": { n: "UA789",  tag: "C" }, "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T21:00:00", "Arrival": @"2017-01-01T23:00:00", "Capacity": 2, "Status": "scheduled" },
        { "Flight Number": { n: "UA1001", tag: "D" }, "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 0, "Status": "cancelled" },
        { "Flight Number": { n: "UA1111", tag: "E" }, "From": "SFO", "To": "LAX", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 2, "Status": "scheduled" }]
    }"#,
  );
  te_be_value(false, scope, r#"Flight List[Status = "cancelled"].Flight Number.tag"#, r#"["B", "D"]"#);
}

#[test]
fn _0065() {
  let scope = &te_scope(r#"{ A: 10 }"#);
  te_number(false, scope, "A[1]", 10, 0);
}

#[test]
fn _0066() {
  let scope = &te_scope(r#"{ A: 10 }"#);
  te_number(false, scope, "A[-1]", 10, 0);
}

#[test]
fn _0067() {
  let scope = &te_scope(r#"{ A: [1,2,3] }"#);
  te_null(false, scope, "A[99999999999999999999]", r#"index is out of range 1..2⁶⁴: 99999999999999999999"#);
}

#[test]
fn _0068() {
  let scope = &te_scope(r#"{ A: [1,2,3] }"#);
  te_null(false, scope, "A[-99999999999999999999]", r#"index is out of range 1..2⁶⁴: -99999999999999999999"#);
}

#[test]
fn _0069() {
  let scope = &te_scope(r#"{ A: [1,2,3] }"#);
  te_null(false, scope, "A[2.5]", r#"index in filter must be an integer value, actual value is 2.5"#);
}

#[test]
fn _0070() {
  te_null(false, &scope!(), r#" @"2023-02-07"[2] "#, "index in filter is out of range [1..1], actual index is 2");
}

#[test]
fn _0071() {
  te_null(false, &scope!(), r#" @"2023-02-07"["a"] "#, "only number or boolean indexes are allowed in filters");
}

#[test]
fn _0072() {
  // any non-list value is filtered as a singleton list, a function value included (DMN16-94)
  te_be_value(false, &scope!(), r#" count((function() 1)[true]) "#, "1");
}

#[test]
fn _0073() {
  te_be_value(false, &scope!(), r#"[1,2,3,4,5][item >= 3]"#, "[3, 4, 5]");
}

#[test]
fn _0074() {
  te_be_value(false, &scope!(), r#"[1,2,3,4,5][item > 5]"#, "[]");
}

#[test]
fn _0075() {
  te_be_value(false, &scope!(), r#""not a list"[true]"#, r#"["not a list"]"#);
}

#[test]
fn _0076() {
  te_null(false, &scope!(), r#"[1,2,3,4,5]["not a boolean"]"#, "only number or boolean indexes are allowed in filters");
}

#[test]
fn dmn_1_6_singleton_list_filter() {
  // DMN 1.6 §10.3.2.9.4 (DMN16-94): a filter on a non-list value filters the singleton list.
  te_be_value(false, &scope!(), "5[item > 3]", "[5]");
  te_be_value(false, &scope!(), "5[item > 6]", "[]");
  te_be_value(false, &scope!(), r#""a"[item = "a"]"#, r#"["a"]"#);
  te_be_value(false, &scope!(), "{a: 1}[a = 1]", r#"[{a: 1}]"#);
  te_be_value(false, &scope!(), "{a: 1}[item.a = 1]", r#"[{a: 1}]"#);
  te_number(false, &scope!(), "5[1]", 5, 0);
  te_number(false, &scope!(), "5[-1]", 5, 0);
  te_null(false, &scope!(), "null[item > 1]", "");
}

#[test]
fn _0077() {
  // The filter gives the same result over a list whose items it shares with another holder and
  // over a list built anew, on every kind of item, and leaves the scope as it found it.
  let scope = &te_scope("{x: 2, item: 9}");
  let lists = [
    "[1, 2, 3]",
    "[{a: 1}, {a: 2}, {a: 3, b: {c: 4}}]",
    "[{item: 5, a: 1}, {item: 1, a: 2}, {a: 3}, 4, null, [1, 2]]",
    "[{a: null}, {b: 1}, {}, {item: null}]",
    "[[1, 2], [3], []]",
    "[]",
  ];
  let predicates = [
    "true",
    "false",
    "item > 1",
    "item = null",
    "a > 1",
    "a = null",
    "item.a > 1",
    "b.c = 4",
    "item > x",
    "count(item) > 1",
    "2",
    "-1",
    "1.5",
    "\"text\"",
    "a",
    "[1,2][\"x\"] = null",
    "{y: item}.y != null",
  ];
  for list in lists {
    let build = || build_evaluator(&rheon_feel_parser::parse_expression(scope, list, false).unwrap())(scope);
    let shared = build();
    for predicate in predicates {
      let evaluator = build_evaluator(&rheon_feel_parser::parse_expression(scope, predicate, false).unwrap());
      let depth = scope.depth();
      let over_shared = crate::FilterExpressionEvaluator::new().evaluate(scope, shared.clone(), &evaluator);
      assert_eq!(depth, scope.depth());
      let over_new = crate::FilterExpressionEvaluator::new().evaluate(scope, build(), &evaluator);
      assert_eq!(depth, scope.depth());
      assert_eq!(over_shared, over_new, "{list}[{predicate}]");
    }
    assert_eq!(build(), shared, "{list}");
  }
}

#[test]
fn _0078() {
  // A predicate that removes the item contexts from the scope (no FEEL predicate does, an
  // evaluator supplied by a caller can) gives a null, and the filter leaves the scope as it
  // found it: for a context item with an entry `item`, a context item without one, and an item
  // that is not a context. A predicate that replaces the special context `{item: value}` with
  // another context gives the same null. The evaluators change the scope only where `item` is in
  // scope, so the evaluation in the enclosing scope, which comes first, leaves it alone; one that
  // removes the frame of that evaluation gives a null of its own.
  let scope = &te_scope("{}");
  let name_item = rheon_feel::Name::from("item");
  let item_in_scope = move |scope: &FeelScope| scope.get_value(&name_item).is_some();
  let popping: rheon_feel::Evaluator = Box::new(move |scope: &FeelScope| {
    if item_in_scope(scope) {
      scope.pop();
    }
    Value::Boolean(true)
  });
  let name_item = rheon_feel::Name::from("item");
  let replacing: rheon_feel::Evaluator = Box::new(move |scope: &FeelScope| {
    if scope.get_value(&name_item).is_some() {
      scope.pop();
      scope.push(rheon_feel::context::FeelContext::default());
    }
    Value::Boolean(true)
  });
  let lost = Value::Null(Some("internal error: the evaluation removed the item context of a filter".to_string()));
  for (item, special_context) in [("{item: 1, a: 2}", false), ("{a: 2}", true), ("3", true)] {
    let list = Value::List(vec![build_evaluator(&rheon_feel_parser::parse_expression(scope, item, false).unwrap())(scope)].into());
    let depth = scope.depth();
    assert_eq!(lost, crate::FilterExpressionEvaluator::new().evaluate(scope, list.clone(), &popping), "{item}");
    assert_eq!(depth, scope.depth(), "{item}");
    if special_context {
      assert_eq!(lost, crate::FilterExpressionEvaluator::new().evaluate(scope, list, &replacing), "{item}");
      assert_eq!(depth, scope.depth(), "{item}");
    }
  }
  let popping: rheon_feel::Evaluator = Box::new(|scope: &FeelScope| {
    scope.pop();
    Value::Boolean(true)
  });
  let depth = scope.depth();
  let lost = Value::Null(Some(
    "internal error: the evaluation removed the frame of a filter expression in the enclosing scope".to_string(),
  ));
  assert_eq!(
    lost,
    crate::FilterExpressionEvaluator::new().evaluate(scope, Value::List(vec![Value::Boolean(true)].into()), &popping)
  );
  assert_eq!(depth, scope.depth());
}

#[test]
fn _0079() {
  // The predicate gets the item contexts shared: what it writes into the context on the top of
  // the scope (a boxed context as the match of a boxed filter does) goes into a copy, which the
  // predicate of the next item does not see, for items that are no context, context items and
  // context items with an entry `item`; and the items are kept as they were. The evaluation in
  // the enclosing scope writes into a frame of its own, so neither the items nor the enclosing
  // scope see its write.
  let written = rheon_feel::Name::from("w");
  let writing: rheon_feel::Evaluator = Box::new(move |scope: &FeelScope| {
    let unseen = scope.get_value(&written).is_none();
    scope.set_value(&written, Value::Boolean(true));
    Value::Boolean(unseen)
  });
  let scope = &te_scope("{}");
  for list in ["[1, 2]", "[{a: 1}, {a: 2}]", "[{item: 1}, {item: 2}]"] {
    let value = build_evaluator(&rheon_feel_parser::parse_expression(scope, list, false).unwrap())(scope);
    let depth = scope.depth();
    assert_eq!(value, crate::FilterExpressionEvaluator::new().evaluate(scope, value.clone(), &writing), "{list}");
    assert_eq!(depth, scope.depth(), "{list}");
    assert_eq!(None, scope.get_value(&rheon_feel::Name::from("w")), "{list}");
  }
}

/// A filter evaluates its expression once in the enclosing scope (DMN 1.6 Table 69, row 1: "e1 is
/// a list and e2 is an integer"), and a number selects the item at once: the expression of
/// `list[2]` is evaluated once for a list of 1,000 items.
#[test]
fn an_index_is_evaluated_once() {
  let runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let counted = std::sync::Arc::clone(&runs);
  let index: rheon_feel::Evaluator = Box::new(move |_: &FeelScope| {
    counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Value::Number(2.into())
  });
  let list = Value::List((1..=1000).map(|i| Value::Number(i.into())).collect());
  let item = crate::FilterExpressionEvaluator::new().evaluate(&FeelScope::default(), list, &index);
  assert_eq!(Value::Number(2.into()), item);
  assert_eq!(1, runs.load(std::sync::atomic::Ordering::Relaxed));
}

/// The evaluation in the enclosing scope counts only when it gives a number: otherwise its
/// errors and warnings are not recorded, and the expression is evaluated for each item (the
/// boolean rows of Table 69). When it is a number, it is the one evaluation and its errors stay.
#[test]
fn the_evaluation_in_the_enclosing_scope_counts_only_when_it_gives_a_number() {
  let addition = r#"incompatible types in addition: 1(number) + "a"(string)"#;
  let items = "[{amount: 50}, {amount: 150}, {amount: 200}]";
  te_errors(&format!("{{items: {items}, r: sum(items[amount > 100].amount)}}.r"), "350", &[]);
  te_errors("[1, 2, 3][item > 1]", "[2, 3]", &[]);
  te_errors(r#"[][1 + "a"]"#, "[]", &[]);
  // the expression is evaluated for each of the three items, and the repeated message is kept once
  let (value, diagnostics) = te_collect(r#"[1, 2, 3][1 + "a"]"#);
  assert_eq!(
    ("[]".to_string(), 3, vec![addition.to_string()]),
    (value.to_string(), diagnostics.count, diagnostics.messages)
  );
  te_errors(r#"[10, 20, 30][count([1 + "a"])]"#, "10", &[addition]);
  te_warnings(r#"[][@"2021-01-01T10:00:00.1234567891" = item]"#, "[]", &[]);
}

/// A number in the enclosing scope selects the item (Table 69, row 1, in the enclosing scope),
/// also where the expression for the items would be a type error or a runaway recursion; the
/// evaluation in the enclosing scope comes first, so a runaway recursion there stops the
/// evaluation.
#[test]
fn a_number_in_the_enclosing_scope_selects_the_item() {
  te_errors(r#"{a: 2, r: [{a: "x"}, {a: "y"}][a]}.r"#, r#"{a: "y"}"#, &[]);
  te_errors(
    "{f: function(n) f(n + 1), g: 1, r: [{g: f, v: 5}][if (g instance of number) then 1 else g(1)].v}.r",
    "5",
    &[],
  );
  te_aborted(
    r#"{f: function(n) f(n + 1), r: [{g: "s"}][if g = "s" then "x" else f(1)]}.r"#,
    crate::Dialect::Feel,
    "function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped",
  );
}
