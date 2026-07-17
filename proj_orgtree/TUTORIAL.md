# proj_orgtree

## What you'll build

An interactive CLI org-chart explorer. Commands from stdin — `hire`, `chain`, `team`, `move`, `quit` — manage a company hierarchy in memory. Every employee must be reachable two ways: down, via their manager's list of reports, and up, via a link to their own manager. That bidirectionality is the whole point: a plain owned tree can't do it, so you'll reach for shared, runtime-checked ownership instead.

## Concepts you'll practice

- `Rc<T>` — shared ownership of heap data (Book ch. 15.4)
- `RefCell<T>` & interior mutability — mutating through a shared reference (ch. 15.5)
- `Weak<T>` & reference cycles — breaking cycles so memory actually frees (ch. 15.6)
- Borrow rules enforced at runtime instead of compile time

## Ground rules

- Hints are collapsed below each milestone. Sit with the problem at least 20 minutes before opening one, and open them in order — Hint 1, then 2, then 3.
- **Special rule for this project:** you *will* hit `already borrowed: BorrowMutError` panics at runtime. That's not a bug in your design — it's the curriculum. When it happens, before touching any code, write down which two `borrow()`/`borrow_mut()` calls were alive at the same time and why. Only then fix it.

## Milestone 1 — a tree that works fine without any of this

**Goal:** Build the org chart as a plain owned tree, no `Rc`/`RefCell`. Hardcode a small hierarchy in `main` and recursively print the subtree with indentation.

**Design questions**
1. If `Employee` owns `Vec<Employee>`, what happens to a report's data when the manager is dropped?
2. Can two variables both own the same `Employee` at once here? Why will that matter later?
3. Does the recursive print function need `&self` or `&mut self`?

**Definition of done** — `cargo run` prints something like the block below, and no `Rc`, `RefCell`, or `Weak` appears anywhere yet:
```
CEO
  VP Engineering
    Alice
    Bob
  VP Sales
    Carol
```

<details><summary>Hint 1 — nudge</summary>This is just a tree of owned values — everything from the struct and Vec chapters already covers it. Resist reaching for anything fancy.</details>
<details><summary>Hint 2 — API pointers</summary>`Vec::push`, recursion with a `depth: usize` parameter, `"  ".repeat(depth)`.</details>
<details><summary>Hint 3 — shape</summary>

```rust
struct Employee {
    name: String,
    reports: Vec<Employee>,
}
```
</details>

## Milestone 2 — try to add `chain`, and fail honestly

**Goal:** Don't write working code. Attempt a `chain` function that walks *upward* to the CEO, and document why Milestone 1's shape can't support it.

**Design questions**
1. To walk upward from `Bob`, what must `Bob` store a reference to?
2. If you add `manager: &Employee`, who owns that `Employee`, and does the owner outlive `Bob` for as long as you need?
3. What would `manager: Option<Box<Employee>>` do to who owns the CEO?
4. You need one node reachable from two directions without either direction *owning* it exclusively. Which two things need relaxing: how many owners are allowed, and whether you can mutate through a shared one?

**Definition of done** — no working `chain`. Instead, a short note (comment or scratch file) stating what you tried, the dead end it hit, and the two capabilities from question 4 that plain references can't give you.

<details><summary>Hint 1 — nudge</summary>This milestone is supposed to fail. If plain references or `Box` seem to work cleanly, check whether your chart ever needs to grow after construction — Milestone 4 needs `hire`.</details>
<details><summary>Hint 2 — API pointers</summary>Try `&'a Employee` fields and see what the borrow checker says once `main` needs one lifetime long enough to also support later mutation.</details>
<details><summary>Hint 3 — shape</summary>No new struct — the point is noticing `Employee` can't grow a `manager` field without picking a lifetime or an owner, and either choice conflicts with something you need later.</details>

## Milestone 3 — parent pointers via `Rc`/`RefCell`/`Weak`

**Goal:** Convert to `Rc<RefCell<Employee>>` with a `parent: Weak<RefCell<Employee>>` field. Implement `chain <name>` (prints the path from that employee up to the CEO).

**Design questions**
1. If `parent` were `Rc` instead of `Weak`, trace CEO → reports → VP → parent → CEO. What happens to the CEO's `strong_count`? Does it ever reach zero?
2. What does `Weak::upgrade` hand back, and why isn't it just the value?
3. What's the exact sequence of borrow/upgrade calls to print one chain link, and when is each borrow released?
4. Could the CEO's own `parent` field be something other than `Weak<RefCell<Employee>>`? Why is that choice consistent with "no parent" rather than a special case?

**Definition of done** — `chain alice` prints e.g. `Alice -> VP Engineering -> CEO`; `chain ceo` prints just `CEO`. After building the chart, `Rc::strong_count` on the CEO reflects only real owners you kept around — parent links don't inflate it.

<details><summary>Hint 1 — nudge</summary>`Weak` is `Rc`'s cycle-safe cousin: it observes but doesn't keep the value alive. Children strongly own, parents weakly reference — that asymmetry is the whole trick.</details>
<details><summary>Hint 2 — API pointers</summary>`Rc::new`, `Rc::clone`, `Rc::downgrade` (Rc → Weak), `Weak::upgrade` (Weak → `Option<Rc<..>>`), `RefCell::borrow`, `Rc::strong_count`.</details>
<details><summary>Hint 3 — shape</summary>

```rust
struct Employee {
    name: String,
    manager: Weak<RefCell<Employee>>,
    reports: Vec<Rc<RefCell<Employee>>>,
}
type EmployeeRef = Rc<RefCell<Employee>>;
```
</details>

## Milestone 4 — the REPL, `hire`, `team`, and a name index

**Goal:** Build the stdin REPL: `hire <name> under <manager>`, `chain <name>`, `team <name>` (subtree print), `quit`. Maintain a `HashMap<String, EmployeeRef>` for name lookup.

**Design questions**
1. Once an employee is both in a manager's `reports` and a `HashMap` value, how many strong owners do they have? Problem, or just a fact to track?
2. `hire` both appends to `reports` and sets the weak `parent`. Which order avoids a half-built state?
3. What happens on `hire <x> under <unknown>`? Decide before coding.
4. `team` recurses through `reports` — what borrow do you take per node, and how long must it live relative to the recursive call?

**Definition of done** — this sequence:
```
hire alice under ceo
hire bob under alice
team ceo
chain bob
quit
```
shows `bob` under `alice` in `team ceo`, and `chain bob` prints `bob -> alice -> ceo`. Unknown manager in `hire` prints an error line, doesn't panic, and the REPL keeps running.

<details><summary>Hint 1 — nudge</summary>The `HashMap` is just another owner — owners are cheap with `Rc`. What's expensive is losing track of which owner you're currently borrowing through.</details>
<details><summary>Hint 2 — API pointers</summary>`std::io::stdin().lines()`, `str::split_whitespace`, `HashMap::get`, `HashMap::insert`, `RefCell::borrow_mut`.</details>
<details><summary>Hint 3 — shape</summary>

```rust
struct Company {
    by_name: std::collections::HashMap<String, EmployeeRef>,
}
```
</details>

## Milestone 5 — `move`, and where the panics live

**Goal:** `move <name> under <new-manager>`: detach from the old manager's `reports`, attach to the new one, repoint the weak `parent`.

**Design questions**
1. List every `borrow()`/`borrow_mut()` your `move` needs, in order, across old manager, new manager, and the moved employee. Do any two overlap in time? If a draft holds two `borrow_mut()`s at once, reorder before running.
2. What should `move alice under alice` do? What about moving a manager under their own report? Decide both before coding — one's a name check, the other needs walking a chain.
3. What happens if either name is missing from the index?
4. After a successful move, does the old manager's `strong_count` change, or does dropping the Vec entry just release it?

**Definition of done** — `move bob under carol` relocates bob; `chain bob` reflects it; `team` on the old manager no longer shows bob. `move alice under alice` is rejected with an error — not a panic, not a hang, not a silent no-op. `move ceo under alice` (ancestor under descendant) is also rejected, not a panic or infinite loop. You've deliberately triggered a `BorrowMutError` at least once while building this and can name which two borrows overlapped.

<details><summary>Hint 1 — nudge</summary>Every `borrow_mut()` should be as short-lived as possible: get in, mutate, get out, then start the next. Holding a borrow "just in case" is the seed of the panic.</details>
<details><summary>Hint 2 — API pointers</summary>`Vec::retain` or `position` + `Vec::remove` to detach; `Rc::ptr_eq` to compare identity, not name; `Rc::downgrade` again for the new parent; repeated `Weak::upgrade` to walk upward and check for the descendant case.</details>
<details><summary>Hint 3 — shape</summary>No new struct — this is entirely about call order and borrow scope on the types you already have.</details>

## Milestone 6 — leak lab

**Goal:** Prove the `Weak` parent pointer is load-bearing. Temporarily change `parent` to `Rc<RefCell<Employee>>`, add a `Drop` impl that prints the employee's name, build a small chart, drop it, and observe.

**Design questions**
1. With strong parent pointers, predict *before running*: which `Drop` messages print, and in what order, if any?
2. Where does the cycle live — one big CEO-to-VP-to-CEO loop, or does every parent/child pair form its own?
3. After reverting to `Weak`, what's different at the moment the cycle would have formed?
4. Which of `strong_count`/`weak_count`, climbing without bound over repeated `hire` calls, would tip you off to this leak at runtime?

**Definition of done** — with strong parent pointers: build a small chart, drop it, observe that `Drop::drop` is *not* called for some or all employees. Revert to `Weak`, rebuild, drop, observe every `Drop` message printed. Write a one-paragraph note on which version you shipped and why it must be `Weak`.

<details><summary>Hint 1 — nudge</summary>`Drop` messages that never print are the leak made visible — the values are still on the heap with nothing left able to reach them safely, and nothing has told the allocator it can free them.</details>
<details><summary>Hint 2 — API pointers</summary>`impl Drop for Employee { fn drop(&mut self) { ... } }`. Build the chart in its own `{ }` block or function so it drops predictably. `Rc::strong_count` for numbers instead of just message absence.</details>
<details><summary>Hint 3 — shape</summary>

```rust
struct Employee {
    name: String,
    manager: Rc<RefCell<Employee>>, // strong, for this lab only
    reports: Vec<Rc<RefCell<Employee>>>,
}
impl Drop for Employee {
    fn drop(&mut self) { /* print self.name */ }
}
```
</details>

## Errors you'll probably meet

- **E0502 / E0506** (borrow conflicts) — the static borrow checker. You'll mostly dodge these inside `RefCell`, since it moves the check to runtime, but you can still hit this family if you mix up borrowing the `Rc` itself vs. borrowing what's inside it.
- **E0507 (cannot move out of `Rc`/`RefCell` contents)** — `Rc<T>` gives shared access, never ownership back. Moving a `String` out of a `RefCell<Employee>` by value is refused because other owners might depend on that data staying put. The fix is to borrow, clone, or restructure — figuring out which is the exercise.
- **`already borrowed: BorrowMutError` (runtime panic)** — `RefCell` couldn't check borrow rules at compile time, so it checks at call time and panics if you request a mutable borrow while any other borrow on the *same* `RefCell` is still alive. The message won't point at the other borrow — only at the failing call site. Read backward from there: which `Ref`/`RefMut` guard (a `let` binding, a temporary held across a method chain, a call you're still nested inside) is still in scope? That overlap is the bug. Write down both sides before fixing, per the Ground Rules.
- **The leaked cycle (`strong_count` never hits zero)** — a reference cycle means every node in it keeps another node alive forever, even with no external variable pointing at any of them. Nothing panics or errors; memory just stays allocated. You demonstrate it with a `Drop` impl whose print never fires, or by checking `Rc::strong_count` right where you expected it to hit zero and finding it's still 1 or more.

## Stretch goals

- Keep a `Drop` impl in the shipped `Weak`-based version too, to visualize shutdown order when the program exits.
- Reimplement using an arena: a single `Vec<Employee>` with `usize` indices for `manager`/`reports` instead of `Rc`/`Weak`. This is the idiomatic escape hatch many real Rust codebases reach for instead of `Rc<RefCell<>>` graphs — compare ergonomics: no runtime borrow panics, but can indices go stale, and what happens on removal?
- Swap `Rc`/`RefCell` for `Arc`/`Mutex` and spawn threads that read/hire concurrently. How do panics change — `BorrowMutError` vs. a poisoned lock?
- Add `undo` for the last `move`/`hire` — forces you to think about what state to snapshot when everything sits behind shared mutable cells.
- Add independent cycle detection: a debug check that walks every employee's `chain` on startup and confirms it terminates at the root within N hops.
