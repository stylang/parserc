# `unsyn` — the DSL and the Rust it maps to

`unsyn` is a DSL for describing a concrete syntax tree. A file of the DSL is a
list of productions; a build turns one file into the types of that tree, the
parsers that read it, and a visitor over it.

This document is the **mapping contract**: what a production becomes, and how
the result is named. It is written from the generator as it stands
(`src/compiler/codegen.rs`) and from the file it writes for the spec itself
(`tests/generated/unsyn_us.rs`), not from intention. Where the two disagree the
code is wrong, not this file.

The language's own grammar, written in the language, is [`unsyn.us`](unsyn.us).

---

## 1. The two levels, and what each one's shape is decided by

A production declares a **rule**, and the keyword says what kind of thing the
rule is. The kind decides *who gets to say the rule's shape*, which is the one
difference that matters:

| keyword      | means          | whose decision the shape is |
|--------------|----------------|------------------------------|
| `whitespace` | the separator  | always one newtype of `I`, read by `take_while` over the characters it lists |
| `lexer`      | a **terminal** | its own right side, **then possibly overruled** by the rest of the file (§1.2) |
| `syntax`     | a **node**     | its own right side, always |

A `syntax` rule is a node and nothing else can make it not one. A `lexer` rule
*may* be one run of text instead, and which ones are is the interesting part of
this document.

### 1.1 Shape from the rule's own right side

The root of the right side is read the same way at both levels, with **one
asymmetry**, and it is the whole of the difference between a terminal and a
node:

| root of the right side                       | a `syntax` rule becomes       | a `lexer` rule becomes |
|----------------------------------------------|-------------------------------|------------------------|
| a choice (`A \| B \| …`)                       | `enum`, one variant per alternative | `enum`, same — until §1.2 overrules it |
| one alternative, several operands (a sequence) | `struct`, one field per operand | `struct`, same — until §1.2 overrules it |
| one alternative, **one operand that is a bare name** | a newtype of *that node*: `pub struct X<I>(pub Y<I>)`, through a `Box` only on a cycle (§2) | **one run of text**: `pub struct A<I>(pub I)`, reading `B` and keeping its text |
| one alternative, one operand that is characters or an external (`U+000A`, `['0'-'7']`, `<Ext>`) | the shape that operand holds | a newtype of `I` |

The third row is the one worth memorising. A `lexer` rule that is nothing but
another rule's name does **not** become that rule's node — it becomes one field
of the text that rule read, because a terminal is a token and a token that is
"whatever `B` was" is still one token:

```text
lexer B -> 'b';
lexer A -> B;          // A<I>(pub I)          — the text B read
syntax Y -> 'y';
syntax X -> Y;        // X<I>(pub Y<I>)        — the node Y, bare until a cycle
```

A reference *inside* a longer right side is different again, and is never
inlined: it always names the type of the rule it reaches. So
`lexer RawIdent -> 'r#' IdentOrKeyword;` keeps a field of that type —

```rust
pub struct RawIdent<I> {
    #[parserc(keyword = "r#")]
    pub f1: I,
    pub ident_or_keyword: IdentOrKeyword<I>,
}
```

— and whether `IdentOrKeyword<I>` is itself a node or a run of text is decided
by *its* right side and by §1.2, not by this site.

### 1.2 How the rest of the file can overrule a `lexer` rule

A `lexer` rule that §1.1 made an enum or a struct is **downgraded to one run of
text** wherever the file reaches it in a position that can only answer with
text. There are exactly three such positions, and the code that finds them is
`node_rules`, which is computed over the whole file to a fixpoint *before any
rule is written*, because the answer is as much a fact about the other rules'
code as about the rule itself:

| a name written…                    | example              | why text is all it can answer |
|------------------------------------|----------------------|-------------------------------|
| as the **whole of a group**        | `( Name )`           | a group is one field, and one field of a terminal is one run of text |
| as one **member of a set** of more than one | `[ 'a', Name ]` | a set answers one question — which characters, or which of its members — however wide a member is |
| as the **target of an `except`**   | `A except B`         | the removal is computed over the *text* `A` took |

Downgrading is what keeps a terminal one token: `LitStr`'s body group, `Set`'s
members, `NonKeywordIdent`'s target. A text rule folds the arms that reach names
of its own, so the question is asked again of whatever the first answers settle,
until the two sets hold still.

Two things this does **not** do, both easy to assume and both wrong:

- it does not flatten a rule reached **beside** others inside a fold. A name in
  that position keeps whichever shape its own production writes, and the fold
  wraps the *reading* of it in `text_of`. So `BlockCommentOrDoc`, reached from
  `BlockCommentBody`'s group, is still an enum of its three rules — it is the
  **group** that answers as one run of text, not the rule.
- it does not reach `syntax` rules at all. `node_rules` walks only the `lexer`
  ones, because a node's field may be a node; only a terminal is at risk of
  being asked to hold one inside its own text.

Naming a part of a terminal is therefore **not a rename**. Hoisting `LitStr`'s
body group out to `lexer LitStrChar -> …` takes it out of the first position
above, so it is no longer forced to text, and the result is
`Vec<LitStrChar>` — a node per character of every string literal in the
language. That is what the lint in §5 lets the author do deliberately and tells
them about when they do it in a `syntax` rule by accident.

---

## 2. Right side → shape

One production, one type, named for the **left side only**. The right side
decides only the *shape* of that type. Nothing on the right side invents a type
by itself; it invents one only when the file gives it a left side of its own.

Read the right side at its **root**:

| root of the right side                     | the type is |
|--------------------------------------------|-------------|
| a choice (`A \| B \| …`)                     | `enum`, one variant per alternative |
| one alternative of several operands        | `struct`, **one field per operand** |
| one alternative of one bare name           | a newtype of that operand's shape at the `syntax` level; **one field of text** at the `lexer` level (§1.1) |

The field count is a hard rule, and it is the one the author is held to: as many
fields as the production has top-level sub-expressions, no matter how few the
tree appears to need. `Concat` writes five fields for `target s keyword s2
operand` even though only two of them carry value, because a CST restores the
source and the runs and the word are part of it.

Within a sequence, each operand contributes the field its shape holds. Three
rows are level-dependent — what a **name** contributes (a node, or its text in
the three positions of §1.2), whether that node is **held through a pointer**, and what a **single
character** is — and the trivia rows below are `syntax`-only. Those are the whole
of what the two levels differ in once §1's shape question is settled:

| operand                | the field holds                                   |
|------------------------|---------------------------------------------------|
| `Name` (a rule)        | `Name<I>`, through a `Box` only where the pair of rules stands on a **cycle of containment** — a type that reaches itself through fields alone has no size, and a `Vec` cuts a cycle wherever it stands, so a DAG of nodes is held bare however wide it reaches |
| `'a'` / `U+0009`       | **level-dependent**: in a `syntax` rule, `Char<I, 'a'>` — a node of one punctuator. In a `lexer` rule, a field of `I` with `#[parserc(next = 'a')]`, because a terminal holds text and not nodes |
| `'abc'`, `'->'`        | a field of `I` with `#[parserc(keyword = "abc")]` — **at either level**. A word is never a `Char`, since `Char` is one character |
| `['a'-'z', '_']`       | **a class**, if every member denotes characters: one field of `I` with `next_if(\|c\| matches!(c, …))` — one test, not a choice. **A fold**, if any member is wider: `.or()` over each member's own parser |
| `~[…]`                 | **one character**, whatever the set is: the class refused — `next_if(\|c\| !matches!(…))` — or, with no class to test, a probe of the members' fold that must not match, then one character taken |
| `A?`                   | `Option<A<I>>`                                     |
| `A*`                   | `Vec<A<I>>` — except that a repetition of a **class** is one greedy `take_while(\|c\| …)` over its predicate, holding characters and not a list of one-character matches |
| `A+`, `{n}`, `{n,m}`   | `Vec<A<I>>` plus a `read_` function that counts    |
| `A B` inside a group   | a tuple of the two fields — no type, no name       |
| `A \| B` inside a group| a nested `Or<…>` — no type, no name                |
| `<Ext>`                | whatever the build's caller bound `Ext` to         |
| `X except Y`           | `X`'s field, whose reader also runs `Y` and refuses the whole if `Y` took all of it |
| `X followed Y`         | `X`'s field, whose reader probes `Y` at the offset `X` ended at and takes nothing |
| `X concat Y`           | **both** `X` and `Y` as fields, with no trivia between |

Two consequences worth stating because they are load-bearing:

- **`concat` is not a field.** It joins two operands into one token, so it
  writes the operands and no separator. `lexer LitUnicode -> 'U+' concat
  HexDigit{4};` is *two* fields, not three.
- **an unnamed group is structure without a name.** A group of one alternative
  becomes a tuple; a group of several becomes a nested `Or`. Neither gets a
  type, a visitor hook, or a lint — but see §5 for when the file is *asked* to
  name one.
- **a set is a class only when every member is one character wide.** What that
  changes is **how the refusal is tested**, never **how much is taken**: a `~`
  matches exactly one character either way, and every longer run comes from a
  repetition around it, which is greedy at both levels alike.

  ```text
  lexer LitStr -> '\'' ( ~['\'', '\\', CR] | QuoteEscape | … )* '\'';
  //                          three members, each one character → a class
  lexer BlockCommentBody -> … ( BlockCommentOrDoc | ~['/*','*/'] )* '*/';
  //                                          two members, each two characters → no class
  ```

  A class is answered by one test of the character here, its arms collected
  from the members — and `CR` contributes its character through the same
  `denotations` table a name is asked for its text by:

  ```rust
  ::parserc::next_if(|c: char| !(matches!(c, '\'' | '\\' | '\r')))
  ```

  With no class to test, the members are read as a choice instead, so the
  refusal is a probe of that choice followed by the taking of one character
  whatever it is — the one character that a class's `matches!` also takes:

  ```rust
  if ::parserc::keyword("/*").or(::parserc::keyword("*/")).ok().parse(&mut rest)?.is_some()
  {
      return Err(::parserc::Kind::NextIf(…).into());   // refuses only what opens a delimiter
  }
  ::parserc::next_if(|_c: char| true).parse(input)?;   // one character, as the class took
  ```

  Both then sit as one arm of the `while` that their group's `*` writes, and it
  is that loop — not the `~` — that captures the whole run. The two shapes do
  part company under a repetition written **on the refusal itself**. A class is
  handed its refused predicate as one greedy combinator, with no loop at all:

  ```text
  lexer InnerLineDoc -> '//!' ~[LF,CR]*;
  ```

  ```rust
  pub struct InnerLineDoc<I>(
      #[parserc(keyword = "//!")] pub I,
      // the whole run, in one greedy combinator, holding characters
      #[parserc(take_while = |c: char| !(matches!(c, '\n' | '\r')))] pub I,
  )
  ```

  A fold has no predicate to hand over, so `code_of_repetition` falls back on
  the loop any other repetition gets, and re-runs its probe once per character.
  No refusal of a fold is written directly repeated in `unsyn.us` — each is
  reached through a group that is — so that fallback is read off the generator
  rather than shown from this crate's own file.

  A name reached from a fold is asked which member it is, and a name reached
  from a class is asked for the characters it denotes; neither keeps its node,
  which is the second position of §1.2. A name that denotes anything wider than
  characters is absent from `denotations`, and its absence is what leaves a set
  with no class in the first place.

### Trivia fields

Between two operands of a `syntax` sequence the generator lays a field of
`Option<S<I>>` — the run of the separator — which the author never writes. It is
a field like any other, so it is counted in the field numbering (§4) and the
visitor reaches it with `visit_s` like any other, and it is why a `syntax`
struct has more fields than the production appears to have:

```text
syntax Stmt -> LeadingKeywords Ident '->' Expr ';';
```

```rust
pub struct Stmt<I> {
    pub leading_keywords: LeadingKeywords<I>,
    pub f1: Option<S<I>>,
    pub ident: Ident<I>,
    pub f2: Option<S<I>>,
    #[parserc(keyword = "->")] pub f3: I,
    pub f4: Option<S<I>>,
    pub expr: Expr<I>,
    pub f5: Option<S<I>>,
    …
}
```

A repetition of a rule that can itself match the trivia carries **no** leading
run, since the trivia between two of its matches is an item of that repetition
rather than a boundary. This is how `Crate -> InnerDoc* Item*` keeps comments as
items of the file.

---

## 3. Type names

The type of a rule is the rule's own name, verbatim, in the file's spelling.

- `lexer BlockCommentBody` → `BlockCommentBody<I>`
- `syntax NegatedSet` → `NegatedSet<I>`

A name written raw keeps its sigil in the declaration, because that is the only
way Rust spells an item named after a keyword, and is read back bare when the
generator has to name something *after* it:

```text
lexer r#gen -> 'a';
```

```rust
pub struct r#gen<I>(…)          // the item is `gen`
fn visit_gen(…)                 // the hook is `visit_gen`
```

Both halves are pinned by `a_name_written_raw_and_one_written_plain_are_one_name`.

`r#gen` and `gen` are **one name**: every lookup inside the compiler — the
collision check, the recursion check, the external table, the shape fixpoint —
keys on the name stripped of its sigil (`external_key`). Only the spelling a
node and a diagnostic wear keeps the sigil.

The type parameter is always `I`, and every path is always rooted at
`::parserc`, because the `Syntax` derive reads the input type off that name.

---

## 4. Field names

Field names come from **what the operand reads**, in one of two forms, and
nothing else.

1. **A reference to a rule** names its field: the rule's name in the case the
   fields of a struct are written in — `to_snake_case`.
   `LeadingKeywords Ident '->' Expr ';'` → `leading_keywords`, `ident`, …,
   `expr`.
2. **Anything that is not a reference** has no name of its own and takes its
   place: `f1`, `f2`, … A counter per struct (per enum variant), incremented
   **only** by nameless fields, so numbering is contiguous among them and says
   nothing about the named ones around them. Trivia runs, single characters,
   keywords and sets are all in this class.

**A repeat of the same rule does not get the name twice.** Where two operands
reach one rule, the first wears the name and the second is nameless — it is a
*different* operand of the same shape, and reusing the name would be lying about
which one it is:

```text
syntax StrRange -> LitStr concat '-' concat LitStr;
```

```rust
pub struct StrRange<I> { pub lit_str: LitStr<I>, pub f1: Char<I, '-'>, pub f2: LitStr<I> }
```

**A struct whose fields are *all* nameless is written as a tuple**, and then no
`fN` is written at all. Where every field is anonymous, the position already
says what each is, and a name would be noise:

```rust
pub struct LitStr<I>(#[parserc(…)] pub I, #[parserc(…)] pub I, #[parserc(…)] pub I)
pub struct Expr<I>(pub (…), pub Vec<(…)>)
```

The moment one field earns a name, *all* the others are numbered beside it —
that is `Paren`, whose `expr` is named and whose brackets and runs are `f1`…`f4`.

`Set` is the instructive case for both rules at once: every one of its five
fields is anonymous (a bracket, a repetition of a group, an option, a bracket),
so it is a tuple, and the trivia inside the repetition and after it is laid down
without ever being named or numbered:

```rust
pub struct Set<I>(
    pub Char<I, '['>,
    pub Vec<(Option<S<I>>, SetItem<I>, Option<S<I>>, Char<I, ','>)>,
    pub Option<(Option<S<I>>, SetItem<I>)>,
    pub Option<S<I>>,
    pub Char<I, ']'>,
)
```

---

## 5. Variant names

A `syntax` enum — and a `lexer` enum, by the same rule — names its variants, in
order, by the **first** thing in the alternative that can supply a word:

| the alternative is                  | the variant is called     |
|-------------------------------------|---------------------------|
| one bare rule name (`LitStr`)       | that rule's name          |
| a `concat` / `followed` / `except`  | the keyword it reads: `Concat`, `Followed`, `Except` |
| a literal whose value starts with a letter (`'lexer'`) | that value, capitalized: `Lexer` |
| anything else                       | nothing — see below       |

A quantity operator supplies no word, including a bound: `{2}` is punctuation,
not a name, and offering `"{"` as one would offer a name no item can wear.

Then two things can still leave a variant unnamed, and both are decided over the
**whole** choice, not per alternative:

- the word is not one an item can wear (`is_item_name`), or
- an earlier alternative already wore it.

An unnamed variant is called by its place: **`F1`, `F2`, …** — one-based, and by
the alternative's position in the choice, independent of any field numbering.
`Suffix -> '*' | '?' | '+' | Repeat` gives `F1`, `F2`, `F3`, `Repeat`.

Naming is computed **once for the whole choice** by `variant_names`, and both
the generator and the lint read that one answer, so a warning can neither
promise a name the code declines to write nor stay silent about one it does.
`variant_offers` is the same pass with the settling removed — what the author
*meant* — which is what lets the lint tell "this reads no word" from "this
offered a word and lost it".

### When the file is told

`UnnamedAlternative` fires where a name was owed and not had, which is exactly
where a *node* was made without one:

| situation                                       | told? | because |
|---|---|---|
| a choice inside a group of a **`syntax`** rule | yes, at the choice | it becomes a nested `Or` with no type and no name |
| a choice inside a group of a **`lexer`** rule  | **no** | it becomes one run of text; nothing in the tree is left unnamed |
| an alternative of the root with **parts** and no word | yes, at the alternative | its variant is an enum case that will be matched by name |
| an alternative of the root that is **one operand** reading no word | **no** | its place *is* all a name could say — there is nothing inside it to tell apart |
| an alternative that offered a word and lost it to a collision | yes | the author meant a name, and only the whole choice can see it went unnamed |

The middle exemption is deliberate and is the reason the lint is not simply
"name everything": a `*`, a `'\n'` and a set can never yield a word, and forcing
the author to write `lexer Star -> '*';` to name one variant would be making them
declare a rule with no parts.

The `lexer`-group exemption is the one that was learned the hard way. Hoisting
`LitStr`'s body group to `lexer LitStrChar -> …` was *not* a rename: it converted
one token of text into `Vec<LitStrChar>` — a node per character of every string
literal in the language. Naming a part of a terminal changes what the terminal
*is*, which is why the check is gated on the kind of the enclosing rule.

---

## 6. Visitor

The build emits a `Visit` trait and a `walk` module in the shape of
[`syn`'s](https://docs.rs/syn/latest/syn/visit/index.html): one `visit_*` hook
per rule that became a **node**, each descending through a default `walk_*`
unless a visitor overrides it, plus one `visit_text` for the leaf every descent
ends at.

The hooks are derived from the shapes the generator already committed to, not
from the source text again, so the visitor is a second view of the same decision
and cannot claim a tree the nodes do not hold. Consequently:

- a rule written as **text** still gets a hook — it is a node in the tree even
  when it holds only `I`; `walk_stmt` reaching `visit_text` for `'->'` and
  `visit_s` for the run before it is exactly what the source wrote;
- a group, an anonymous choice and an anonymous character field get **no** hook
  of their own — they have no name to build one from, which is the whole subject
  of §5 — and the walk reaches them by position instead.

That last point is what makes the numbering in §4 load-bearing rather than
cosmetic: `walk_set` descends into `(inner).0`, `(inner).2` and `(inner).3`
because those are what the fields were written as, and a nameless field is only
findable by its place.

### Where the hand-written tree differs

`src/compiler/{lexer,syntax}.rs` is written by hand and tuned where a generated
file is merely correct. Two of its differences bear on this document, and both
are reasons everything above should be read as the **generated** rule:

- **a single character is a different type at each level of implementation.**
  The generated code holds `Char<I, '~'>`, wearing the character in its type.
  The hand file holds no `Char` anywhere, and names each punctuator as a type of
  its own — `Tilde`, `Star`, `Question`, `Plus`, `Or`, `Lt`, `Gt`, `ParenStart`,
  … — through `syntax_terminal!`, which is what lets a hand-written refusal say
  *which* punctuator it expected. The two trees agree on the span and disagree
  on the type that carries it.
- **field and variant names.** The generator has only the file to go by, so it
  says `f1`, `f2` and the rule's own name; the hand file names what a part holds
  — `open`, `head`, `tail`, `marker`, `body`. §4 is therefore the generator's
  rule and not the hand file's.

---

## 7. What is refused rather than written wrong

The generator declines instead of approximating. Each refusal is an
`UnsynError::Build` carrying one of five `BuildKind`s, and names the rule that
asked for it:

| `BuildKind`            | refused when |
|------------------------|-----------|
| `Name`                 | a name the file wrote is one no Rust item can wear (`is_item_name` fails) |
| `External`             | the pattern calls `<Ext>` and the build was never given a binding for `Ext` |
| `Whitespace`           | the `whitespace` rule's right side cannot be reduced to a test of one character |
| `Literal`              | a set member written as a range whose ends are not both single characters, or a literal that spells no character at all |
| `Syntax`               | a right side no field can hold: a counted repetition of anything wider than a character, an operator inside a group, a name asked for its text where a tuple answers with types |

`Builder::external` is the **only** way anything outside the file enters a
build. The generator holds no table of what `<XID_Start>` means and never will:
`<…>` is an external link, resolved by whoever runs the build, and it is
meaningless to the generator, the analyzer and the lint alike.

---

## 8. What the file is noted for: the lints

A lint is a note on a file that means what it wrote and need not have written
it, or wrote it twice. The two channels of a report are kept apart by what they
imply (`Diagnostics`): the refusals of §7 reject a build; a lint travels with a
model that builds. Every lint is a `LintKind` and the span of the construct
that earned it, and there are six kinds.

| `LintKind` | fires where |
|---|---|
| `DuplicateMember(m)` | a set member `m` denotes exactly what an earlier member of the same set already denotes: `[ 'a', 'a' ]`, `[ 'a', U+0061 ]`, `[ LF, LF ]` |
| `CoveredMember(m, e)` | `m` denotes nothing that the earlier `e` does not — `m` is the redundant write: `[ 'a'-'d', 'b' ]` |
| `CoversMember(m, e)` | `m` denotes everything `e` does and more — the *earlier* member is the redundant one: `[ 'b', 'a'-'d' ]` |
| `OverlappingMember(m, e)` | `m` and `e` share some values and each keeps some of its own: `[ 'a'-'b', 'b'-'c' ]` |
| `RedundantSeparator(name)` | a `syntax` rule mentions the file's `whitespace` rule anywhere short of the whole of an alternative, where its generated separator takes that trivia whether it is named or not: `syntax A -> S 'x';` over `whitespace S -> …` — and one report per mention |
| `UnnamedAlternative` | an alternative of a root choice names no rule and reads no word, or a choice stands inside a group of a `syntax` rule — the situations, the table, and every exemption are §5 |

### Set members compared as values

The four `…Member` kinds come from comparing one member's *value* against
another's (`conflict`), where a value is what the member denotes:

- a one-character literal is a character; a code-point literal and a range are
  code-point intervals; a longer literal is a **string** — and a character and
  a string of more than one character are never the same value, so `'a'` and
  `'ab'` simply do not compare;
- a member named by a rule is compared **by its name alone**: what the name
  denotes is what that rule's definition denotes, which is not this pass's
  business — two members naming one rule are a `DuplicateMember`, and a name
  and a literal never conflict;
- characters compare four ways — same (`Same`), inside an earlier one
  (`Covered`), around an earlier one (`Covers`), sharing some code points
  (`Overlap`) — and strings only compare for exact equality.

A member is told **once**: of the earlier members it conflicts with, the first
one earns the report and the rest are not repeated; the text each report
names is the member as the file wrote it. What cannot be settled as a value —
a range whose ends are not both single characters — is a refusal, not a lint
(`BuildKind::Literal`, §7), and never reaches the comparison.

### Naming the separator twice

`RedundantSeparator` is asked only of a `syntax` rule, and only of a mention
that is *not* the whole of an alternative: a rule whose right side is just `S`
names the trivia as the tree's own field, and no lint touches it — `syntax A ->
'a' | S | 'b';` stays silent, and the group `(S)` is the mention itself, not a
second ask. A suffix or a `concat` on the mention does not dissolve it: `S*`
and `S concat 'x'` both ask for a run the separator takes anyway, and each
mention in the rule is told once on its own. The name is
matched through `external_key`, so a rule written `r#S` is the separator the
file names just as plainly as one written `S`.

---

## 9. What the derive accepts

Everything above is what the generator *writes*; this is what the derive it
writes against *reads*, and the two are deliberately the same vocabulary.

Every combinator of `parserc` may be written as an attribute at **every**
position a parser is needed — on a `struct`, on an `enum`, on one of its
variants, and on a field — under the name of the function it calls:

| attribute                | writes the call          | reads              |
|--------------------------|--------------------------|--------------------|
| `next = 'a'`             | `parserc::next('a')`     | one item           |
| `next_if = f`            | `parserc::next_if(f)`    | one item `f` takes |
| `keyword = "if"`         | `parserc::keyword("if")` | one word           |
| `take_until = ";"`       | `parserc::take_until(";")` | the run before a word, leaving it |
| `take_while = f`         | `parserc::take_while(f)` | the longest run `f` takes, possibly none |
| `take_while_with = (r, f)` | `parserc::take_while_with(r, f)` | the same, bounded by `r` |
| `take_till = f`          | `parserc::take_till(f)`  | the longest run `f` rejects |
| `parser = p`             | `p` itself               | whatever `p` parses — the escape hatch |
| a bare path, `read_name` | `read_name` itself       | the same as `parser = read_name` |

A reader that is nothing but a name is written as the name:
`#[parserc(read_lit_str_body)]`, not `#[parserc(parser = read_lit_str_body)]`.
The two are one read to the derive; the short form is what the generated code
and this crate's own terminals use. It stops at a path because that is as far as
a nested attribute's path reaches — a call of a combinator, a fold of `.or()`, or
any other reader with tokens of its own still needs `parser = …`.

Nothing is swallowed by this. A name that resolves to nothing is a hard error
from rustc (`cannot find value \`read_nope\``), pointing at the attribute; and a
misspelled *option* — one carrying a value, like `keywor = "if"` — is still
refused by the derive, because the value is what tells the two apart.

### Where the attribute stands

A reader is said **of the item** when the item is a newtype of the input — one
unnamed field whose type is the input — and **of the field** everywhere else:

```rust
// one run of text, and one field to hold it: the type says the whole of itself
#[parserc(read_ident_or_keyword)]
pub struct IdentOrKeyword<I>(pub I)
where
    I: UnsynInput;

// several parts, each read its own way: an attribute per field
pub struct LitStr<I>(
    #[parserc(next = '\'')] pub I,
    #[parserc(read_lit_str_1)] pub I,
    #[parserc(next = '\'')] pub I,
)
where
    I: UnsynInput;
```

The rule is that a field-level attribute exists to say which of several parts is
read how. A field of one leaves no such question, so the attribute moves out and
sits beside the `derive`, where it describes the type. An enum's *variants* keep
their field-level attributes for the same reason in reverse: each is one of
several, even when it holds only one run — `lexer AsciiEscape` writes

```rust
pub enum AsciiEscape<I>
where
    I: ::parserc::chars::CharsInput,
{
    AsciiHexEscape(AsciiHexEscape<I>),
    F2(#[parserc(keyword = "\n")] I),
    F3(#[parserc(keyword = "\r")] I),
    // …
}
```

An external `<Ext>` also stays on `parser =` at either level: the builder handed
over a token stream of unknown shape, so nothing here can know whether it is a
name, a call, or a fold.

Three consequences of that table being uniform, each of which is a limitation
the derive used to have:

- **a `struct` or `enum` can be one token.** `#[parserc(keyword = "if")] struct
  If<I>(pub I)` works, and so does an `enum` of exactly one such variant. It
  demands one unnamed field whose type is the input, and says so when the item
  has more or different.
- **a variant can be a leaf of a choice**: `enum Punctuation<I> { #[parserc(next
  = ',')] Comma(I), Pair(Lhs<I>, Rhs<I>), … }` — one arm read as a run beside
  arms read from parts.
- **what a combinator reads is checked where it is written.** Each of them hands
  back a run of the input, so the thing it is set on must *be* the input type;
  that used to be checked for `keyword` and `take_while` alone, and `parser =`
  bypassed it entirely.

`crates/parserc/tests/derive_combinators.rs` pins the whole table at all four
positions. Three options are about a position rather than a read and are
therefore restricted: `input = Type` on an item only, `crucial` and
`left_recursion` on a field only. Two are accepted anywhere: `map_err`, `semantic`.

---

## 10. Reading this back

The fastest check of any claim above is to regenerate and diff:

```sh
cargo run -p unsyn --features compiler --example regen_unsyn_us
cargo fmt --all
git --no-pager diff crates/unsyn/tests/generated/unsyn_us.rs
```

`tests/unsyn_us.rs` pins that the kept file *is* the code of the spec and then
reads the spec with it; `tests/two_trees.rs` compares the generated tree against
the hand-written one in `src/compiler/{lexer,syntax}.rs`.

The plan for where this is going — self-hosting, and the hand-written side still
lagging the ten rules §5 made nodes — is in
[`MODELING_PLAN.md`](MODELING_PLAN.md), whose open item M8.2b-10 records exactly
which rules the two trees still settle differently.
