# Morph

Morph is a programming language built to be easy for people **and AI models** to read, write, and fix.
It is starting very small and growing one step at a time.

```morph
let name = "Morph"
let area = 6 * 7
print("Hello from", name, "- the answer is", area)
```

## Running a program

Morph programs use the `.mai` file extension (Morph + AI). You need [Rust](https://rustup.rs) installed. Then:

```sh
cargo run -- examples/hello.mai
```

## What Morph can do today (v0.2)

| Feature    | Example                         |
|------------|---------------------------------|
| Numbers    | `42`, `3.14`, `-7`              |
| Strings    | `"hello"`, `"tab\there"`        |
| Variables  | `let total = 10`                |
| Arithmetic | `(1 + 2) * 3 / 4 - 5`           |
| Join text  | `"Hello, " + name`              |
| Print      | `print("total is", total)`      |
| Comments   | `# anything after # is ignored` |
| Booleans   | `true`, `false`                 |
| Comparison | `==` `!=` `<` `<=` `>` `>=`     |
| Logic      | `a and b`, `a or b`, `not a`    |
| Decisions  | `if` / `else if` / `else`     |

Blocks use `{ }`: the `{` goes at the end of the line that opens the block,
each statement inside goes on its own line, and `}` goes on its own line
(or before `else`, as in `} else {`). A variable made inside a block only exists inside it.

```morph
if temperature > 30 {
    print("It's hot!")
} else if temperature > 15 {
    print("It's pleasant.")
} else {
    print("Bring a jacket.")
}
```

See `examples/decisions.mai` for more.

## Design principles

These shape every feature we add.

1. **One obvious way to write each thing.** One statement per line, no optional semicolons,
   one way to make a variable (`let`), and calls always look like `name(...)`. Regular syntax is
   easier for a model to generate correctly and easier for a person to review.
2. **No hidden surprises.** Morph never converts types behind your back: `"a" + 1` is an error,
   not `"a1"`. `if 1 {` is an error because conditions must be `true` or `false`, and
   `1 == "1"` is an error rather than a silent `false`. Each `let` makes a *new* name,
   so reusing one by accident is caught. Logic uses words (`and`, `or`, `not`) instead of symbols.
3. **Errors you can act on.** Every error has a stable code, an exact `file:line:column`,
   a plain-English message, and a hint that says how to fix it:

   ```text
   error[E202]: there is no variable named `totl`
    --> examples/typo.mai:3:7
     |
   3 | print(totl * 2)
     |       ^
     |
     = hint: did you mean `total`?
   ```

## How it works

Morph is a *tree-walking interpreter* written in Rust. Running a program takes three steps:

1. **Lexer** (`src/lexer.rs`) splits the text into tokens: `let`, `total`, `=`, `10`.
2. **Parser** (`src/parser.rs`) arranges tokens into a tree that captures meaning,
   like "multiply before adding".
3. **Interpreter** (`src/interpreter.rs`) walks that tree and runs it.

Errors and their codes live in `src/error.rs`. Tests live in `tests/run.rs` (`cargo test`).
