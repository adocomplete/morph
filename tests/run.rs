fn run(source: &str) -> Result<String, morph::error::MorphError> {
    let mut out = Vec::new();
    morph::run(source, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

fn error_code(source: &str) -> &'static str {
    run(source).expect_err("expected an error").code
}

#[test]
fn hello_world() {
    assert_eq!(run("print(\"Hello, world!\")").unwrap(), "Hello, world!\n");
}

#[test]
fn arithmetic_precedence() {
    assert_eq!(
        run("print(1 + 2 * 3)\nprint((1 + 2) * 3)\nprint(-4 / 2)").unwrap(),
        "7\n9\n-2\n"
    );
}

#[test]
fn variables_and_strings() {
    let src = "let a = \"Hi\"\nlet b = a + \", there\"\nprint(b, 1.5)";
    assert_eq!(run(src).unwrap(), "Hi, there 1.5\n");
}

#[test]
fn comments_and_blank_lines() {
    assert_eq!(run("# hi\n\nprint(1) # trailing\n\n").unwrap(), "1\n");
}

#[test]
fn examples_run() {
    let basics = std::fs::read_to_string("examples/basics.mai").unwrap();
    assert_eq!(
        run(&basics).unwrap(),
        "Welcome to Morph 0.1\nArea: 42\nAverage: 6.5\nHello, Morph!\n"
    );
}

#[test]
fn errors_have_stable_codes() {
    assert_eq!(error_code("print(\"oops)"), "E001");
    assert_eq!(error_code("let x = 1 2"), "E101");
    assert_eq!(error_code("let = 1"), "E102");
    assert_eq!(error_code("print(1"), "E105");
    assert_eq!(error_code("let x = 1\nlet x = 2"), "E201");
    assert_eq!(error_code("print(y)"), "E202");
    assert_eq!(error_code("print(1 / 0)"), "E205");
    assert_eq!(error_code("print(\"a\" + 1)"), "E206");
    assert_eq!(error_code("print(\"a\" * 2)"), "E207");
}

#[test]
fn typo_suggests_close_name() {
    let err = run("let total = 1\nprint(totl)").unwrap_err();
    assert_eq!(err.hint, "did you mean `total`?");
    assert_eq!((err.span.line, err.span.col), (2, 7));
}

#[test]
fn booleans_and_comparisons() {
    let src = "print(1 < 2, 2 <= 1, 3 > 2, 3 >= 4, 1 == 1, \"a\" != \"b\", true == false)";
    assert_eq!(run(src).unwrap(), "true false true false true true false\n");
}

#[test]
fn logic_operators() {
    let src = "print(true and false, true or false, not true, not 1 > 2 and 2 > 1)";
    assert_eq!(run(src).unwrap(), "false true false true\n");
}

#[test]
fn logic_stops_early() {
    // The right side would fail, but the left side already decides the answer.
    assert_eq!(
        run("print(false and missing, true or missing)").unwrap(),
        "false true\n"
    );
}

#[test]
fn if_else_chains() {
    let src = "let n = 5\nif n > 10 {\n    print(\"big\")\n} else if n > 3 {\n    print(\"medium\")\n} else {\n    print(\"small\")\n}\nif n == 0 {\n    print(\"zero\")\n}";
    assert_eq!(run(src).unwrap(), "medium\n");
}

#[test]
fn block_variables_stay_in_their_block() {
    assert_eq!(error_code("if true {\n    let x = 1\n}\nprint(x)"), "E202");
    assert_eq!(error_code("let x = 1\nif true {\n    let x = 2\n}"), "E201");
    // Two separate blocks may each use the same name.
    let src =
        "if true {\n    let x = 1\n    print(x)\n}\nif true {\n    let x = 2\n    print(x)\n}";
    assert_eq!(run(src).unwrap(), "1\n2\n");
}

#[test]
fn decisions_example_runs() {
    let src = std::fs::read_to_string("examples/decisions.mai").unwrap();
    assert_eq!(
        run(&src).unwrap(),
        "Is it warm? true\nIt's pleasant.\nYou'll get wet.\nHello, Morph\n"
    );
}

#[test]
fn decision_errors_have_stable_codes() {
    assert_eq!(error_code("print(1 ! 2)"), "E005");
    assert_eq!(error_code("print(1 < 2 < 3)"), "E108");
    assert_eq!(error_code("if true print(1)"), "E109");
    assert_eq!(error_code("if true { print(1) }"), "E110");
    assert_eq!(error_code("if true {\n    print(1)\n"), "E111");
    assert_eq!(error_code("if true {\n    print(1)\n}\nelse {\n}"), "E112");
    assert_eq!(error_code("if true {\n    print(1) }"), "E101");
    assert_eq!(error_code("print(1 == \"1\")"), "E208");
    assert_eq!(error_code("print(\"a\" < \"b\")"), "E207");
    assert_eq!(error_code("if 1 {\n}"), "E209");
    assert_eq!(error_code("print(1 and true)"), "E210");
    assert_eq!(error_code("print(not 0)"), "E210");
}
