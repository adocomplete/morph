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
