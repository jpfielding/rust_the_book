# proj_expr_calc

## What you'll build

A REPL arithmetic calculator: type `3 + 4 * (2 - 1)`, get `7`. You'll build a
tokenizer, a recursive-descent parser that respects `*`/`/` over `+`/`-` and
honors parentheses, and an evaluator — all over an AST represented as a Rust
enum. The calculator itself is a throwaway; the point is muscle memory for
enums-with-data, `Box`-based recursive types, exhaustive matching, and
`Result`-based error plumbing. Those four things show up everywhere in Rust.

## Concepts you'll practice

- Enums with data (variants carrying payloads) — Book ch. 6
- Exhaustive `match` / "non-exhaustive patterns" — Book ch. 6.2, 18.1
- `Box<T>` and recursive types — Book ch. 15.1
- Ownership of nested/owned data — Book ch. 4
- `Result<T, E>` and `?` for fallible chains — Book ch. 9.2
- `std::fmt::Display` for custom error types — Book ch. 10.2, std `fmt::Display`
- Iterators and `Peekable` — Book ch. 13.2, std `std::iter::Peekable`
- Reading stdin line by line — Book ch. 12.1, std `std::io::Stdin::lines`

## Ground rules

- Hints below are collapsed. Don't open them until you've tried the
  milestone for 20+ minutes — no cheating.
- Hint 1 is a nudge. Hint 2 names APIs. Hint 3 gives a type shape — never a
  function body. If Hint 3 doesn't unblock you, reread the Book chapter
  before moving on.
- Run `cargo check` constantly — the exhaustiveness and borrow checkers are
  teaching tools here, not obstacles.

## Milestone 1 — REPL skeleton

**Goal:** `cargo run` starts a loop. Each line typed is echoed back with a
`> ` prefix. `exit`/`quit` or EOF (Ctrl-D) ends the program cleanly.

**Design questions**
1. What type does a line of stdin input come back as — does it include the
   trailing newline?
2. How do you distinguish "empty line" from "stdin closed (EOF)"?
3. Does the loop belong in `main`, or a helper function? What changes if
   setup could fail?
4. Where does trimming whitespace belong relative to checking for `exit`?

**Definition of done**
```
$ cargo run
> hello
echo: hello
> exit
$
```
(Ctrl-D should also exit without a panic.)

<details><summary>Hint 1 — nudge</summary>
You need a loop that reads one line per iteration and a way to break out.
Think about what "no more input" looks like as a value, not an exception.
</details>

<details><summary>Hint 2 — API pointers</summary>
Look up `std::io::stdin`, `Stdin::lines`, `Iterator::next`, and
`String::trim`. Decide between a `for` loop over `.lines()` or manually
calling `.next()` inside a `loop`.
</details>

<details><summary>Hint 3 — shape</summary>
No types to design here — this milestone is control flow only.
```
fn main() {
    // loop { read a line; if EOF or "exit"/"quit", break; else echo }
}
```
</details>

## Milestone 2 — Tokenizer

**Goal:** Turn `"12 + 3*(4-1)"` into a `Vec<Token>` and print it, e.g.
`[Number(12), Plus, Number(3), LParen, Number(4), Minus, Number(1),
RParen]`. Whitespace is ignored; multi-digit numbers are one token.

**Design questions**
1. Should `Token::Number` hold `i64`, `f64`, or a `String` parsed later?
   What does each choice cost downstream?
2. How do you know a run of digits has ended — consume one char at a time,
   or grab a substring up front?
3. What happens on an unrecognized character (e.g. `#`)? Panic, skip, or
   error — which fits Milestone 5's "no panics" goal?
4. Does the tokenizer need to know about operator precedence at all?

**Definition of done**
```
$ cargo run
> 12 + 3*(4-1)
[Number(12), Plus, Number(3), Star, LParen, Number(4), Minus, Number(1), RParen]
```

<details><summary>Hint 1 — nudge</summary>
Scan left to right, deciding at each position: digit, operator, whitespace,
or garbage. One *token* can span multiple *characters* (multi-digit
numbers), so you need to look ahead without consuming until the number
ends.
</details>

<details><summary>Hint 2 — API pointers</summary>
`str::chars()` gives a `Chars` iterator; `.peekable()` wraps it into
`Peekable<Chars>`. Look up `Peekable::peek` (look, don't consume) vs
`Iterator::next` (consume). `char::is_ascii_digit` and `char::to_digit(10)`
turn characters into digits; building the number as `n = n * 10 + digit`
avoids needing a string buffer (though `String` + `str::parse` also works).
</details>

<details><summary>Hint 3 — shape</summary>
Grammar and token set are given (naming tokens isn't the exercise; the
scanner over them is):
```
Token variants: Number(i64), Plus, Minus, Star, Slash, LParen, RParen
```
Tokenizer signature is your call — roughly `&str -> Vec<Token>` (or
`Result<Vec<Token>, YourError>` once Milestone 5 exists).
</details>

## Milestone 3 — AST and recursive-descent parser

**Goal:** Turn `Vec<Token>` into a tree reflecting precedence (`*`/`/`
before `+`/`-`) and parens. No evaluation yet — just build and
`{:?}`-print the tree.

**Design questions**
1. An expression is a number, or an operator applied to two
   sub-expressions. If a variant directly contains another instance of the
   same enum by value, what does the compiler say, and why — what would
   `size_of::<Expr>()` have to be?
2. Given the compiler's suggestion, who owns each sub-expression now — the
   parent node, or something external to the tree?
3. Precedence is usually encoded as separate grammar rules (`expr` built
   from `term`s, `term` from `factor`s), not as data on `Expr`. Why does
   layering the parser functions this way get precedence right with no
   explicit precedence number stored anywhere?
4. Does your parser consume tokens by mutating a shared index/iterator, or
   by returning "value plus remaining tokens" from each function?
5. What happens when tokens run out mid-expression (input `"3 +"`)? Where
   should that surface?

**Definition of done**
A `{:?}` derive on your AST is enough:
```
$ cargo run
> 3 + 4 * (2 - 1)
Add(Number(3), Mul(Number(4), Sub(Number(2), Number(1))))
```
(Exact formatting depends on your names — the point is correct nesting.)

<details><summary>Hint 1 — nudge</summary>
This is where "an expression contains expressions" stops being a metaphor.
Solve the compile error first — don't route around it by flattening the
tree. Write the grammar down before writing parser code; each rule becomes
roughly one function.
</details>

<details><summary>Hint 2 — API pointers</summary>
Look up `Box<T>` — why it makes a type's size fixed regardless of what's
inside. For token consumption: `slice::split_first`, `Vec::remove(0)`
(inefficient but fine here), or a `Peekable` iterator over your tokens.
Reread Book ch. 15.1's recursive-`List` example — same problem, different
name.
</details>

<details><summary>Hint 3 — shape</summary>
Grammar (BNF), given as spec, not exercise:
```
expr   := term (('+' | '-') term)*
term   := factor (('*' | '/') factor)*
factor := NUMBER | '(' expr ')'
```
`Expr` needs a plain-number variant and variant(s) for binary operations
where operands are `Box<Expr>`. Whether you use one `BinOp { op, lhs:
Box<Expr>, rhs: Box<Expr> }` shape or separate `Add`/`Sub`/`Mul`/`Div`
variants is your call — both are real. Sketch on paper first.
</details>

## Milestone 4 — Evaluator

**Goal:** Walk the AST, produce a number. `3 + 4 * (2 - 1)` → `7`. Division
by zero does not panic — it produces an error you can print.

**Design questions**
1. Matching on a `Box<Expr>` sub-node — do you dereference manually, or
   does match ergonomics handle it? Try it and see.
2. What's the eval function's return type, given division by zero must be
   representable as *not a number*?
3. Where does the zero-check happen — inside the `Div` arm, or elsewhere?
   What breaks if you check before recursing into operands?
4. Integer division truncates. Is that what you want, and if not, what
   does switching cost elsewhere (tokenizer, AST, Display)?

**Definition of done**
```
$ cargo run
> 3 + 4 * (2 - 1)
7
> 1 / 0
Error: division by zero
```

<details><summary>Hint 1 — nudge</summary>
The evaluator's shape mirrors the AST's: one match arm per `Expr` variant,
binary-operator arms recurse on both operands before combining. The new
idea is that combining can fail.
</details>

<details><summary>Hint 2 — API pointers</summary>
Look up `Result<T, E>` and `?` (Book ch. 9.2) to propagate a recursive eval
failure without matching at every call site. A plain `String` as `E` works
temporarily; Milestone 5 replaces it.
</details>

<details><summary>Hint 3 — shape</summary>
```
fn eval(expr: &Expr) -> Result<i64, /* some error type */> { ... }
```
Note `&Expr` — evaluating doesn't need to consume the tree.
</details>

## Milestone 5 — Real error type + Display

**Goal:** Replace every `String` error and every `panic!`/`unwrap()` across
tokenizer, parser, and evaluator with one error enum spanning all three
stages. The REPL prints friendly messages — never panics — for `"3 + "`,
`"3 $ 4"`, `"1 / 0"`.

**Design questions**
1. One enum spanning all stages, or three separate ones? What's the
   tradeoff for the REPL's top-level handling either way?
2. What does each variant need to carry to be useful (bad character?
   position? nothing)?
3. Why does a REPL want `Display` specifically over `Debug`, and what's
   `Debug` still useful for?
4. Does `main`'s per-line error handling get simpler or messier with one
   error enum vs. three?

**Definition of done**
```
$ cargo run
> 3 +
Error: unexpected end of input
> 3 $ 4
Error: unexpected character '$'
> 1 / 0
Error: division by zero
> 3 + 4 * (2 - 1)
7
```
No input should ever panic — try to break it.

<details><summary>Hint 1 — nudge</summary>
This is a refactor, not new logic. Everywhere you currently `panic!`,
`unwrap()`, `expect()`, or return a bare `String`, ask: what concrete
situation caused this, and which enum variant represents it?
</details>

<details><summary>Hint 2 — API pointers</summary>
Implement `std::fmt::Display` (and typically `std::error::Error`) for your
error enum via `write!`. Consider `From<TokenizeError> for CalcError` so
`?` converts between stages automatically — look up the `From` trait.
</details>

<details><summary>Hint 3 — shape</summary>
```
enum CalcError {
    UnexpectedChar(char),
    UnexpectedEnd,
    DivisionByZero,
    // ...whatever else you've hit
}
```
Every `Result<T, String>` becomes `Result<T, CalcError>`.
</details>

## Compiler errors you'll probably meet

- **E0072 — recursive type has infinite size.** Hits the moment an `Expr`
  variant contains a bare `Expr` (not `Box<Expr>`). A type's size must be
  known at compile time; a type containing itself by value has no finite
  size. `Box<Expr>` fixes this — a `Box` is a pointer-sized handle to heap
  data regardless of what's behind it.
- **E0308 — mismatched types.** Common while parsing: one arm returns
  `Expr`, another `Box<Expr>`, or `i64` where you meant `Result<i64, _>`.
  Read both branches, not just the flagged line.
- **E0004 — non-exhaustive patterns.** Your `match` on `Token` or `Expr`
  misses a variant. Usually a real gap in your logic (what *should* happen
  there?), not a formality to silence with `_ =>`.
- **"cannot move out of X which is behind a shared reference."** Happens if
  the evaluator takes `&Expr` but tries to move a `Box<Expr>` out of a
  match arm instead of matching on a reference. Check whether you're
  binding by value where you meant by reference.
- **"the trait `Display` is not implemented for `CalcError`."** Shows up if
  you use `{}` on your error before writing `impl Display`. `{:?}` (Debug,
  usually derived) works without it — that's the tell.

## Stretch goals

- Variables and `let` bindings (`let x = 3 + 4` then `x * 2` later): needs
  a persistent `HashMap<String, i64>` environment surviving across REPL
  iterations — where does that state live relative to your loop?
- Unary minus: `-3 + 4` and `-(2 + 2)`. Adds a grammar rule and an `Expr`
  variant, and forces disambiguating `-` as unary vs. binary in the parser.
- Floats: switch `Token::Number`/`Expr::Number` to `f64` (or support both).
  See what ripples through tokenizer, evaluator, and division-by-zero
  handling (`f64 / 0.0` gives `inf`, not a panic — better or worse here?).
- `impl Display for Expr` that pretty-prints the AST back as an expression
  string with correct parenthesization — round-trip
  `parse(tokenize(s)).to_string()` as a sanity check.
- Unit tests (`#[cfg(test)]`, `#[test]`, `assert_eq!`) for the tokenizer and
  evaluator covering nested parens, division by zero, malformed input.
