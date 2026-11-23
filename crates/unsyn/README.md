# `unsyn` — the language, and the Rust it becomes

`unsyn` is a DSL for describing a concrete syntax tree. A file of the DSL is a
list of productions; a build turns one file into the types of that tree, the
parsers that read it, and a visitor over it.

This document has two parts. **Part I is the language**: what a file is, what
every construct means, what a file must obey, and what it is warned about.
**Part II is the build**: how a production becomes a Rust type, how the results
are named, what the generator refuses, and what the derive reads. Both are
written from the code as it stands (`src/compiler/`) and from the file it writes
for the spec itself (`tests/generated/unsyn_us.rs`), not from intention. Where
the two disagree the code is wrong, not this file.

The language's own grammar, written in the language, is
[`unsyn.us`](unsyn.us) — and it is the authority for everything Part I claims.

---

# Part I — The language

## 1. Files, items and declarations

A file is a sequence of **items**: outer documentation and definitions, with an
optional run of inner documentation at the top.

```text
//! inner documentation for the file

whitespace S -> [ U+0009-U+000D, U+0020 ];

/// documentation for the rule
lexer  Ident -> [ 'a'-'z' ]+;
syntax Expr  -> Term ( '+' Term )*;
```

A definition — the language's one statement form — opens with a keyword, names
one rule, and gives its right side:

```text
syntax Name -> <right side>;
```

The keyword says what kind of thing the rule is:

| keyword      | the rule is                       | what it means in the language |
|--------------|-----------------------------------|-------------------------------|
| `whitespace` | the separator                     | the characters the language skips between tokens, and between the operands of any `syntax` right side; it is a natural separator and is never asked for inside a terminal |
| `lexer`      | a **terminal** (token)            | one run of characters, matched without skipping anything |
| `syntax`     | a **node**                        | a piece of structure, read operand by operand with the separator between them |

A file must declare the separator exactly once, and may not define one name
twice:

- no `whitespace` is `MissingWhitespace`, and a second one is
  `DuplicateWhitespace`;
- a name the file already defined is `NameCollision`, pointing at the first
  definition.

The rule is mandatory because a token has no end without it. The reader of a
`syntax` sequence drops a run of the trivia before every operand, and it is that
dropping which tells `lexer` from `lexerX`: with no separator the operands abut,
the two are one uninterrupted run of characters, and `except` — which refuses a
whole token and never a boundary inside one — cannot save the reading. A build of
such a file has no code to write, so it is refused before any rule is read.

`r#gen` and `gen` are **one name** — a raw spelling is a sigil on the same
name, not a second one — so they collide with each other, answer each other's
references, and bind the same external (§2, §10).

## 2. Names, words and literals

**Identifiers** are the usual: a letter of `XID_Start` or `_`, then as many
`XID_Continue` as wanted. A keyword of the language can not name a rule
(`Keyword`), and a name written after Rust's heart may wear the raw sigil:
`lexer r#type -> …`. The keywords are exactly six:

| where a name may open a definition | inside a right side |
|---|---|
| `lexer`, `syntax`, `whitespace` | `concat`, `followed`, `except` |

**Character and string literals** are written between `'`:

```text
'a'  '\n'  '\''  '\x41'  '\u{1F600}'  'abc'  '->'
```

The escapes are:

| escape | reads | value |
|---|---|---|
| `\'` | the quote itself | `'` |
| `\n` `\r` `\t` `\\` `\0` | the named control | that character |
| `\xNN` | exactly two hex digits, the first octal | the 7-bit code point — the spelling itself keeps it under `0x80` |
| `\u{…}` | one to six hex digits, no separators | that Unicode code point, refused for a surrogate or anything past `U+10FFFF` |

A literal of **one character** (`'a'`, `'\n'`, `\x41`, `\u{41}` — each is one
character however many its spelling takes) is a *character*; a longer literal
is a *word*. The distinction is semantic, not cosmetic: a word never matches
less than its whole text, and §5, §6 and §9 tell apart what each may stand for.

**Code points** are also spelled directly, with `U+` and exactly four hex
digits: `U+000A`, `U+2000`. A `U+…` of the wrong width or of no code point is
refused.

## 3. Comments and documentation

- Line comments `//…` run to the end of the line; block comments `/* … */`
  **nest** — `/* /* */ */` is one comment — and the two empty forms are
  spelled whole: `/**/` and `/***/`.
- Documentation is comments with a second marker: `///` and `/** … */` document
  what follows them; `//!` and `/*! … */` document the file a list of them
  stands in. Block documents nest like the comments they are grown from.

## 4. Patterns

The right side of a definition is a **pattern**. Patterns are built from
operands, composed three ways, and share their whole notation between the two
levels.

### Atoms

| written | reads |
|---|---|
| `Name` | a reference: the whole of the named rule's match — a terminal reads one token, a node reads its structure |
| `<Ext>` | an external: a run of characters the *caller of the build* binds `Ext` to; the name needs no definition in the file, and nothing here may say what it matches (§14) |
| `'a'` | the character |
| `'abc'` | the word, one item at a time and as a whole |
| `U+000A` | the code point |
| `[…]` | a set — see below |
| `~X` | a refusal — see below |
| `( … )` | a group: the pattern written inside, standing as one operand |

### Sets

`[ 'a', 'e'-'i', U+006F, 'u' ]` asks one question of one character: *which of
these*. Members are:

- a character literal, a code point, or a **range** of either —
  `'a'-'z'`, `U+0041-U+005A`, `'a'-U+0062`; the two ends of a range are
  compared by code point, and the lower end may not exceed the upper
  (`InvalidRange`);
- a name of a lexer rule, which answers with the characters its definition
  denotes — at least one of them, and no more.

**Every member denotes exactly one character, and nothing else is a member.** A
member that spells a word, a range reaching over a word, or a rule whose
definition answers wider than a character is refused (`WideSetMember`, §6) — the
choice the author meant is written as a choice, and the refusal they meant is `~`
over an expression. A rule whose definition can be matched by **nothing** is
refused the same way under a different name (`EmptySetMember`, §6): it offers no
character for the set to ask about, and the author meant a choice of what that
rule reads, not a set of characters. An empty set denotes nothing and is refused
(`EmptySet`).

### Negation

`~X` refuses the operand `X` at this position and takes **one character**: the
probe of `X` must not match, and whatever character stands here after the
refusal is the answer. `X` is any single operand — a set, a group, a literal, a
name, even a call of an external — and a refusal always matches exactly one
character; every longer run comes from a repetition written around it.

Three things follow from that one shape, and they are the whole of what `~` is
not:

- **a suffix beside `~` stands on the refusal, never under it** (§4). `~[LF,CR]*`
  refuses one character as often as it can. There is therefore no `~(X*)` to
  write: the `*` binds to the refusal, not to the operand inside the group, and
  no operand of a `~` ever carries a quantity of its own.
- **what a refusal can still get wrong is a name.** `~B` asks that `B` not begin
  here, and a definition that can be matched by nothing begins everywhere, so
  the refusal has no character left to take: that is `UnsatisfiableRefusal` (§6),
  told through the names a file reaches as well as through the ones it writes.
- **`~` is no lookahead.** It takes the character whatever it is, so it is a
  refusal and not an assertion. The language's one construct that consumes
  nothing is `followed` (§4), and there is deliberately no zero-width `!X`
  beside it: every operator has to advance, which is what keeps a repetition
  from being written around something that matches nothing at all (§6).
  What a lookahead would be asked for elsewhere is asked of `except` here,
  which refuses a whole token rather than a position (§4), or of the repeated
  refusal `~X*`, which stops at the first character `X` would begin.

An empty input is no position to refuse at: a bare `~X` where the input has run
out is `Incomplete`, not a refusal that fits. Under `*` and `+` that never
reaches an author, since the repetition stops where the characters do.

### Sequence, choice, and operators

- A **sequence** is operands written side by side, read left to right. At the
  `syntax` level the separator between two of them belongs to the match and is
  skipped; at the `lexer` level the run is one token and nothing is skipped.
- A **choice** joins alternatives with `|` and is read **first-to-fitted**:
  the first alternative whose head matches is the one. Choices sit at the root
  of a right side (`A | B | C`) or inside a group (`( A | B )`).
- Three operators join two operands in a word of their own:

| operator | reads |
|---|---|
| `X concat Y` | the two operands as **one token**: whatever they match, with nothing between them |
| `X followed Y` | `X`, and nothing of `Y` — the position after `X` must begin `Y`, which is left for whatever reads next |
| `X except Y` | `X`, refused where `Y` took **the whole of it**: a run that `Y` matches in full is not `X` |

`concat` writes no separator and asks for none: `LitUnicode concat '-'` is
one word spelling `U+0-`. `followed` and `except` take no field of their
right operand — the right side is a probe, not a run (what that becomes when
it is code is §9, and what may stand in each position is the table above).

### Repetition

A suffix written on one operand repeats it:

| suffix | reads |
|---|---|
| `X*` | `X` as many times as it fits, including none — greedily |
| `X?` | `X` at most once |
| `X+` | `X` at least once — greedily |
| `X{n}` | exactly `n` times |
| `X{n,}` | at least `n` |
| `X{,n}` | at most `n` |
| `X{n,m}` | between `n` and `m`, and `n` may not exceed `m` (`InvalidRepeat`) |

The count is written in decimal (`LitDec`): underscores and leading zeros are
spelling, not different numbers.

Repetition and refusal are greedy where they can be: a `*` or `+` stops only
when its operand no longer fits, and at the lexer level a repeated class of
characters reads the longest run of them, not one character at a time (§9).

### What binds where

Choice is the whole of a right side and binds loosest. Operators and suffixes
bind to the operand written beside them, tighter than the sequence around
them: `A B* C` is a sequence of `A`, `B*`, `C`; `A concat B C` is a sequence
of `A concat B` and `C`; `~A*` is the refusal repeated, not the repetition
refused. A group parenthesizes any of it.

## 5. Kinds again, from the pattern side

The two levels share the notation above, and the kind of the rule decides what
the notation *is for*:

- a **`syntax`** rule is a piece of structure: its operands are read with the
  separator between them, and a reference reaches whatever it names as a piece
  of structure or a token.
- a **`lexer`** rule is one token: its whole match is one run of characters,
  nothing inside it skips the separator, and a reference inside it reaches a
  token too — a terminal may not name a `syntax` rule at all
  (`UnexpectedSyntax`).
- the **`whitespace`** rule is a pattern like any other — its right side is
  read at the `lexer` level — and its purpose is to say what the separator
  *is*. Naming it again inside a `syntax` pattern is the same run said twice
  (§7).

## 6. What a file must obey

A file that breaks one of these denotes no grammar; the analyzer says so, and
the build never runs.

| refused | when |
|---|---|
| `Keyword` | a keyword of the language is used to name a rule |
| `NameCollision` | a name is defined twice — raw and plain spellings are one name |
| `DuplicateWhitespace` | more than one `whitespace` declaration |
| `MissingWhitespace` | the file declares no `whitespace` rule, and so has no separator to end a token with (§1) |
| `UndefinedSymbol` | a pattern names a rule the file never defines (a reference, a set member, or the target of a refusal) |
| `UnexpectedSyntax` | a `lexer` rule names a `syntax` rule |
| `EmptySet` | `[]`, or `~[]`, stands in a pattern |
| `WideSetMember` | a set member that spells a word, ranges over a word, or names a rule answering wider than a character (§4) |
| `EmptySetMember` | a set member naming a rule that can be matched by nothing, which answers the set with no character to test (§4) |
| `UnsatisfiableRefusal` | a `~` whose operand can be matched by nothing — a name reaching a definition that is optional, repeated `{0,}` or `{,n}` — which begins everywhere and so leaves the refusal no character to take (§4) |
| `NotAChar` | a literal asked to be one character is not — the ends of a range, or an empty literal anywhere a character is required |
| `InvalidRange` | a range whose lower end is above its upper |
| `InvalidCodePoint` | a `U+…` or `\u{…}` that is no Unicode code point — past `U+10FFFF`, or a surrogate |
| `InvalidRepeat` | a bound whose lower number is above its upper |
| `LeftRecursion` | a definition that can begin a match of itself, advancing nothing — a node whose first operand is (or reaches) its own name |
| `SameOffsetRecursion` | a node whose match at a position can only be decided by deciding the same node at the same position — a cycle that neither advances nor terminates, through optionals and empties |

The two code-point spellings are refused where they are read: a `U+…` of
other than four hex digits, and a `\u{…}` whose digits do not close the
escape. What is well formed but spells no code point is `InvalidCodePoint`
(§6); `\x` needs no report — its grammar already admits only 7-bit values.

## 7. What a file is told

A lint is a note on a file that *does* denote a grammar and says something it
need not have said, or said twice. The two channels of a report are kept apart
by what they imply (`Diagnostics`): the refusals of §6 reject a file; a lint
travels with a build. Every lint is a `LintKind` and the span of the construct
that earned it, and there are six kinds.

| `LintKind` | fires where |
|---|---|
| `DuplicateMember(m)` | a set member `m` denotes exactly what an earlier member of the same set already denotes: `[ 'a', 'a' ]`, `[ 'a', U+0061 ]`, `[ LF, LF ]` |
| `CoveredMember(m, e)` | `m` denotes nothing that the earlier `e` does not — `m` is the redundant write: `[ 'a'-'d', 'b' ]` |
| `CoversMember(m, e)` | `m` denotes everything `e` does and more — the *earlier* member is the redundant one: `[ 'b', 'a'-'d' ]` |
| `OverlappingMember(m, e)` | `m` and `e` share some values and each keeps some of its own: `[ 'a'-'b', 'b'-'c' ]` |
| `RedundantSeparator(name)` | a `syntax` rule mentions the file's `whitespace` rule anywhere short of the whole of an alternative, where its generated separator takes that trivia whether it is named or not: `syntax A -> S 'x';` over `whitespace S -> …` — and one report per mention |
| `UnnamedAlternative` | an alternative of a root choice names no rule and reads no word, or a choice stands inside a group of a `syntax` rule — the table and every exemption are below |

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
(`WideSetMember` / `NotAChar`, §6), and never reaches the comparison.

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

### When an alternative is told

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
`LitStr`'s body group out to `lexer LitStrChar -> …` was *not* a rename: it converted
one token of text into `Vec<LitStrChar>` — a node per character of every string
literal in the language. Naming a part of a terminal changes what the terminal
*is*, which is why the check is gated on the kind of the enclosing rule.

---

# Part II — The build

## 8. The two levels, and what each one's shape is decided by

A production declares a **rule**, and the keyword says what kind of thing the
rule is. The kind decides *who gets to say the rule's shape*, which is the one
difference that matters:

| keyword      | means          | whose decision the shape is |
|--------------|----------------|------------------------------|
| `whitespace` | the separator  | always one newtype of `I`, read by `take_while` over the characters it lists |
| `lexer`      | a **terminal** | its own right side, **then possibly overruled** by the rest of the file (§8.2) |
| `syntax`     | a **node**     | its own right side, always |

A `syntax` rule is a node and nothing else can make it not one. A `lexer` rule
*may* be one run of text instead, and which ones are is the interesting part of
this document.

### 8.1 Shape from the rule's own right side

The root of the right side is read the same way at both levels, with **one
asymmetry**, and it is the whole of the difference between a terminal and a
node:

| root of the right side                       | a `syntax` rule becomes       | a `lexer` rule becomes |
|----------------------------------------------|-------------------------------|------------------------|
| a choice (`A \| B \| …`)                       | `enum`, one variant per alternative | `enum`, same — until §8.2 overrules it |
| one alternative, several operands (a sequence) | `struct`, one field per operand | `struct`, same — until §8.2 overrules it |
| one alternative, **one operand** | a newtype of what that operand holds — of *that node* for a bare name (`pub struct X<I>(pub Y<I>)`, through a `Box` only on a cycle, §9), the character class or the external's answer for the rest | **one run of text**: `pub struct A<I>(pub I)`, for a bare name reading `B` and keeping its text, as for characters (`U+000A`, `['0'-'7']`) or an external (`<Ext>`) |

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
by *its* right side and by §8.2, not by this site.

### 8.2 How the rest of the file can overrule a `lexer` rule

A `lexer` rule that §8.1 made an enum or a struct is **downgraded to one run of
text** wherever the file reaches it in a position that can only answer with
text. There are exactly three such positions, and the code that finds them is
`node_rules`, which is computed over the whole file to a fixpoint *before any
rule is written*, because the answer is as much a fact about the other rules'
code as about the rule itself:

| a name written…                    | example              | why text is all it can answer |
|------------------------------------|----------------------|-------------------------------|
| as the **whole of a group**        | `( Name )`           | a group is one field, and one field of a terminal is one run of text |
| as one **member of a set**  | `[ 'a', CR ]`        | a set asks which character the position holds, and a member that names a rule answers with the characters it denotes |
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

Naming a part of a terminal is therefore **not a rename** — see the lint of
§7 for what the author is told when they do it where a node is meant.

## 9. Right side → shape

One production, one type, named for the **left side only**. The right side
decides only the *shape* of that type. Nothing on the right side invents a type
by itself; it invents one only when the file gives it a left side of its own.

Read the right side at its **root**:

| root of the right side                     | the type is |
|--------------------------------------------|-------------|
| a choice (`A \| B \| …`)                     | `enum`, one variant per alternative |
| one alternative of several operands        | `struct`, **one field per operand** |
| one alternative of one bare name           | a newtype of that operand's shape at the `syntax` level; **one field of text** at the `lexer` level (§8.1) |

The field count is a hard rule, and it is the one the author is held to: as many
fields as the production has top-level sub-expressions, no matter how few the
tree appears to need. `Concat` writes five fields for `target s keyword s2
operand` even though only two of them carry value, because a CST restores the
source and the runs and the word are part of it.

Within a sequence, each operand contributes the field its shape holds. Three
rows are level-dependent — what a **name** contributes (a node, or its text in
the three positions of §8.2), whether that node is **held through a pointer**, and what a **single
character** is — and the trivia rows below are `syntax`-only. Those are the whole
of what the two levels differ in once §8's shape question is settled:

| operand                | the field holds                                   |
|------------------------|---------------------------------------------------|
| `Name` (a rule)        | `Name<I>`, through a `Box` only where the pair of rules stands on a **cycle of containment** — a type that reaches itself through fields alone has no size, and a `Vec` cuts a cycle wherever it stands, so a DAG of nodes is held bare however wide it reaches |
| `'a'` / `U+0009`       | **level-dependent**: in a `syntax` rule, `Char<I, 'a'>` — a node of one punctuator. In a `lexer` rule, a field of `I` with `#[parserc(next = 'a')]`, because a terminal holds text and not nodes |
| `'abc'`, `'->'`        | a field of `I` with `#[parserc(keyword = "abc")]` — **at either level**. A word is never a `Char`, since `Char` is one character |
| `['a'-'z', '_']`       | **always a class**: one field of `I` with `next_if(\|c\| matches!(c, …))` — one test, not a choice. Every member denotes characters, because the analyzer refused whatever did not (§4, §6) |
| `~X`                   | **one character**, as the language says (§4): where `X` is a class, one test answers it — `next_if(\|c\| !matches!(…))`; where it reaches wider, its own parser is probed that it must not match, then one character is taken whatever it is |
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
  type, a visitor hook, or a lint — but see §7 and §12 for when the file is
  *asked* to name one.
- **the refusal is tested by its width, and the loop is not written by the
  mark.** A class under `~` answers as one predicate, and a repetition of that
  refusal is one greedy combinator with no loop at all:

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

  Anything wider under `~` has no predicate to hand over: its own parser
  stands as the probe, the character after the refusal is taken whatever it
  is, and a repetition written around it re-runs the probe once per character
  —

  ```rust
  if ::parserc::keyword("/*").or(::parserc::keyword("*/")).ok().parse(&mut rest)?.is_some()
  {
      return Err(::parserc::Kind::NextIf(…).into());   // refuses only what opens a delimiter
  }
  ::parserc::next_if(|_c: char| true).parse(input)?;   // one character, as the class took
  ```

  which is how `BlockCommentBody`'s `( BlockCommentOrDoc | ~( '/*' | '*/' ) )*`
  captures its whole run through the `while` its group's `*` writes — the
  loop, not the `~`.

  A name written as a member is asked for the characters it denotes, and a
  name reached from a refused class is asked the same; neither keeps its
  node, which is the second position of §8.2. The table that says which
  names settle to characters is `denotations`, and the analyzer reads it
  before the generator is ever asked for code.

### Trivia fields

Between two operands of a `syntax` sequence the generator lays a field of
`Option<S<I>>` — the run of the separator — which the author never writes. It is
a field like any other, so it is counted in the field numbering (§11) and the
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

## 10. Type names

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

## 11. Field names

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

## 12. Variant names

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

When the file is *told* that a variant will be called by its place — and which
cases are exempt — is the `UnnamedAlternative` lint, and its whole table lives
with the other lints in §7.

---

## 13. Visitor

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
  of §12 — and the walk reaches them by position instead.

That last point is what makes the numbering in §11 load-bearing rather than
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
  — `open`, `head`, `tail`, `marker`, `body`. §11 is therefore the generator's
  rule and not the hand file's.

---

## 14. What the generator refuses

The generator declines instead of approximating. Each refusal is an
`UnsynError::Build` carrying one of five `BuildKind`s, and names the rule that
asked for it:

| `BuildKind`            | refused when |
|------------------------|-----------|
| `Name`                 | a name the file wrote is one no Rust item can wear (`is_item_name` fails) |
| `External`             | the pattern calls `<Ext>` and the build was never given a binding for `Ext` |
| `Whitespace`           | the `whitespace` rule's right side cannot be reduced to a test of one character |
| `Literal`              | a literal that spells no character where the code asked one |
| `Syntax`               | a right side no field can hold: a counted repetition of anything wider than a character, an operator inside a group, a name asked for its text where a tuple answers with types |

The language-level refusals — the file denoting no grammar — are §6, and they
run first: a build is only ever asked of an analyzed file, and a precondition
this module states rather than re-checks is that a rule reaching a name the
file never declares was refused there, not here.

`Builder::external` is the **only** way anything outside the file enters a
build. The generator holds no table of what `<XID_Start>` means and never will:
`<…>` is an external link, resolved by whoever runs the build, and it is
meaningless to the generator, the analyzer and the lint alike.

---

## 15. What the derive accepts

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

## 16. Reading this back

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
lagging the ten rules the spec made nodes — is in
[`MODELING_PLAN.md`](MODELING_PLAN.md), whose open item M8.2b-10 records exactly
which rules the two trees still settle differently.
