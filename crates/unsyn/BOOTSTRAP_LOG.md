# Bootstrap log

Goal: `unsyn.us` is read by the parser the generator wrote for it, and the
crate's own analysis and generation — unchanged, no `cfg` inside them — are run
on the tree that parser built, producing the code that made that parser. The
loop closing is the proof that the hand front end and the generated front end
are one front end.

The experiment was run once already. Pointing the three front-end slots at the
generated file and compiling `semantic.rs` + `codegen.rs` against it gives
**543 errors**, which is the distance, and its shape is what the stages below
are ordered by.

## The scoreboard, and how it is read

`--all-features` turns on `bootstrap` and so builds the harness: it is **not** a
gate, it is the measurement. The gates are the shipped set —
`--features serde,compiler,runtimes`, or plain defaults, or `cargo test
--workspace` — which leave the harness out.

```ntext
cargo check -p unsyn --features bootstrap --example bootstrap   # the distance
cargo test  -p unsyn --features serde,compiler,runtimes         # the gate
```

The opening count, by what the compiler is actually complaining about:

| errors | what it means | taken out by |
|---|---|---|
| 338 | `I: CharsInput` not satisfied — a build binds a stream to the runtime's bound while the analysis is written against the compiler's `UnsynInput` | **S2** ✅ |
| 132 | no variant by that name, or no such method | **S4** |
| 25 | no field by that name | **S4** |
| 22 | a variant that does not have those fields | **S4** |
| 10 | no type `OneOrMore` (and its import) | **S4** |
| 8 + 4 | a shape mismatch, a pattern of the wrong arity | **S4** |
| 1 | unresolved import | **S4** |

So one stage — naming the input what the analysis already assumes — is 62% of
the distance, and the rest is the tree's own vocabulary. S1 earns nothing on
this scoreboard; it is here because a generated file that hard-codes
`::parserc::Kind` cannot be the same file as the hand-written one whatever the
types are called, and because the phase split is what the comparison is *about*.

**Where it stands after S2: 179**, all of it S4's vocabulary — see the table
there.

## Ground rules

- **Generated code mimics the hand-written code.** Same families, same split of
  errors by compilation phase, same attribute conventions. Where the hand file
  is merely prettier, the generator does not get to be uglier.
- **No conditional compilation in `semantic.rs` / `codegen.rs` / `read.rs`.**
  Which front end they were given is expressed only by what the example wires by
  path, and by names the `Builder` configures.
- **The generator states nothing about unsyn.** Names it cannot read off the
  tree are conventions, and every convention is written down here before it is
  implemented.
- Every stage ends with the gates green and the tree committed-ready. A stage
  that leaves the crate red is not a stage.

## Decisions to make before the code

| # | question | rule chosen |
|---|---|---|
| D1 | how is a literal a **punct** rather than a **keyword** | a literal whose text is all non-`XID` characters (`';'`, `'->'`, `'~'`, `'r#'`) is a punct; a literal that begins with an `XID_Start` character (`'lexer'`, `'concat'`) is a keyword. Recorded because `r#` is two letters' worth of sign and `->` is two characters |
| D2 | what names a `SyntaxKind` variant | the rule's own left side, snake-cased then CamelCase, exactly as the type is named — one variant per named rule, no exceptions |
| D3 | which phases a generated file speaks | **all of them, split the way the hand file splits them** — `PunctKind`, `KeywordKind`, `SyntaxKind`, `SemanticsKind`, `LintKind`, `BuildKind`, and an `…Error` that carries them, because a file that bundles its reports by phase is what the analysis is written against. The first three are *derived* from the tree (D1, D2). The last three are the DSL's own rules and no target grammar states them, so a build cannot invent their variants: it writes them under the **configured path**, and `Builder` decides whether that path is the generated file's own or the compiler's (`unsyn::compiler::errors`). The open question is only which of those two the bootstrap loop should exercise — see below |
| D4 | the default input name | `UnsynInput`, overridable by `Builder::input_name(..)`; the `TokenStream` alias follows it |
| D5 | inline variant vs named rule | whatever the spec names decides it — the generator keeps folding nothing and inventing nothing; the hand file moves to match (S4) |
| D6 | does the bootstrap loop emit its own `Error` and `Input`? | **Yes — both, and this is the shape D3 and S2 already asked for.** An earlier version of this row said *no*, on the ground that `UnsynInput` pins `I::Error = UnsynError` and so a file that emitted its own `UnsynError` would collide with the bound it imports. That premise was self-inflicted: it held only *because* S2 made the file import the crate's trait. `chars::TokenStream<'a, E>` needs only `E: ParseError`, and `ParseError: From<Kind>`, so a build can emit the error; and a build that emits **both** the error and the trait makes `I::Error` name the error it just wrote, which cannot collide with itself. The rule is therefore: **a build emits `UnsynInput`, the `TokenStream` alias, and the three derived families; it carries `SemanticsKind`/`LintKind`/`BuildKind`/`Lint`/`Diagnostics` from `errors_at`**, because those are the DSL's own rules and no target grammar states them (D3). For a build aimed at a foreign crate, `errors_at` names that crate's analysis families instead. |

**Consequence for the stages, corrected.** S1 and S2 are one stage, not two: the trait's `Error = …` clause and the emitted `…Error` enum are the same decision seen from either side, and S2's first bullet said so from the start. Emitting the input alone (renaming the bound) leaves `::parserc::Kind` hard-coded where a phase-specific report belongs, which is the defect S1 exists to remove. **Stage order: S2 + S1 together → S4 → S3 → S5 → S6**, and the harness keeps `input`/`errors` as slots that take what the generated file emits.

## Stage 0 — the harness, inert ✅

- [x] `examples/bootstrap/main.rs` + `examples/bootstrap/compiler/mod.rs`,
      mirroring `src/`'s layout, the three front-end slots re-exporting one
      generated file
- [x] `[[example]] name = "bootstrap"`, `required-features = ["bootstrap"]`, and
      a `bootstrap = ["compiler"]` feature, so the shipped gates stay green
      while the loop cannot compile
- [x] the fixed point printed, not asserted — `main.rs` asserts, but nothing
      reaches it until S5
- [x] baseline measured and recorded above: **543**

The gates as this closed: 111 + 5 + 10 + 1 tests green, clippy silent, `fmt`
clean, `cargo test --workspace` 15 results green.

## Stage 2b — the file emits its own stream ✅

*(Placed before Stage 2 for reading order because it **supersedes** part of that
stage's conclusion: Stage 2 imported the stream trait, 2b emits it. Stage 2 is
kept below because its measurement of the 338, and the harness lesson in it, are
what made the count move.)*

D6 said a build emits the trait and the alias. It now does.

- [x] `input_defs` writes `pub trait #input: ::parserc::Input<Item = char,
      Error = #errors_at::#error> + #bounds`, the `TokenStream` alias bound to
      that error, **and the `impl` that the alias satisfies the trait** — the
      last of these was first forgotten, which is the whole difference between
      a trait a stream is written against and one nothing is.
- [x] the bound list is a `Builder` setting, not a reading of the grammar. A
      grammar's rules ask only to take one character at a time; the *analysis*
      of the tree asks `AsStr`, `AsBytes`, `StartWith`, `Find` too, and those
      appear nowhere in what a build writes for the rules. Measured, not
      assumed: the emitted file uses none of them, `semantic.rs` and
      `codegen.rs` use all four. So they are configured, and the default is
      this crate's set — an unconfigured build writes the trait the crate
      ships, which is the property `the_kept_file_is_the_code_of_the_spec` now
      pins for the stream as much as for the nodes.
- [x] all bounds fully qualified (`::parserc::`, `::core::`), so the emitted
      file needs no import for the stream — the `is_empty` sentinel of the
      helper prelude and the unused-import warnings are both avoided.
- [x] `input_at(path)` kept as the **opt-in import mode**: a build aimed at a
      crate that already has the stream imports it and writes no trait, alias
      or impl — which is why `a_build_aimed_at_another_crate_names_that_crate`
      still passes unchanged and still says nothing of unsyn.
- [x] the harness `input` slot flipped to the generated file (it now writes
      the trait), while `errors` and `decode` stay re-exported from the crate;
      `two_trees.rs` and `unsyn_us.rs` feed each front end *its own* stream,
      both the same concrete `chars::TokenStream<'_, UnsynError>`.

Scoreboard **unchanged at 179** — expected: emitting the stream was never for
the count, it is for the file being self-contained, and the count moved only
when the trait stopped being copied (S2). No regression, all gates green.

## Stage 1 — generated `Error`, split by phase

The hand file divides a report by **which pass could have made it**: a sign
expected, a word expected, a node expected, a thing well formed that denotes
nothing, a thing that denotes something said twice, a shape no code is written
for. A generated file has to make the same division or it is not the same file,
whatever the types are named.

- [ ] derive the three lexical/syntactic families from the tree: every punct
      literal by D1, every keyword literal by D1, every named rule by D2
- [ ] carry the three analysis families by the path `Builder` is given (D3)
- [ ] emit the `…Error` enum with the hand shape: `Punct`/`Keyword`/`Syntax`
      arms holding `(kind, ControlFlow, Span)`, a `Semantics`/`Build` arm
      holding `(kind, Span)`, `Kind` passed through, and the `ParseError` impl
      that answers `to_span`/`control_flow`/`into_fatal` per family
- [ ] replace the four hard-coded `::parserc::Kind::…` emissions
      (`codegen.rs` 1909, 2427, 2951, 3062) with the generated families — these
      are where a build today flattens a phase distinction it should keep
- [x] `Builder::error_at(name, path)`: default `UnsynError` at
      `::unsyn::compiler::errors`, so an unconfigured build names the error the
      crate ships — and the stream trait it emits now refers to that type, which
      is why the error name is a builder decision rather than a constant
- [ ] gates: shipped set green; scoreboard moves only where an error *was* the
      hard-coded `Kind`, which is not most of it — S1 is measured by the
      generated file containing the six families, not by the count

**Held after S4, on two naming blockers (D6).** Both are measured, not guessed:

- `PunctKind`: `'('` → `ParenStart`, `'<'` → `Lt`, `'->'` → `ArrowRight`,
  `';'` → `Semi`. No rule, literal or shape yields an English name for a sign,
  so a generator-held sign table is required — about punctuation rather than
  about unsyn, hence defensible, but still a table, and D5 says the spec
  decides. Wants a ruling, not an inference.
- `SyntaxKind` contradicts D2 in the hand file: it holds `Unicode`, `Dec` and
  `StrContent` where D2 and the type names say `LitUnicode`, `LitDec`, and
  `LitStr`'s body. D2 has the better rule; the hand file is the inconsistent
  side.

The bound lesson, which is why the builder owns it even though it is about the
stream: the emitted parser code uses only `Input<Item=char>` + `Clone` +
`Debug` + `PartialEq` (`split_to`, `len`, `to_span_at`, `is_empty`), while
`read`/`semantic`/`codegen` additionally use `as_str`, `starts_with` and `find`.
A bound derived from the rules alone is too weak for the loop, so the
supertraits are `Builder::input_bound(..)`, defaulting to this crate's set.

## Stage 2 — generated `Input` ✅

The 338. A build today writes `I: ::parserc::chars::CharsInput` on every type,
while the analysis is written against `UnsynInput` — a compiler-phase name, and
the reason a generated file is not a drop-in front end before any naming is
even discussed.

- [x] every bound site writes the configured name instead of
      `::parserc::chars::CharsInput` (14 of them, counting the three free
      functions `struct_item`, `newtype` and `reader`, which now take the name
      as an argument — they are called from methods that hold it)
- [x] `Builder::input_name(..)` overrides the name; `Builder::input_at(..)`
      says where it is found, because a bare name written into a bound is no
      name at all unless the file imports it
- [x] **default name `UnsynInput`**, checked as any other name the code wears
      is (`item_name`), so a builder given something no item can be called says
      so at `build` rather than at the first bound
- [ ] ~~emit the trait, its supertraits derived from the combinators the rules
      actually use, and the `TokenStream` alias bound to the generated `Error`~~
      — **not done, and D3 says it should not be.** A build *imports* the
      stream today. Emitting a trait is the same act as emitting `Error`, which
      D3 defers: the supertraits are exactly what the emitted `Error` makes
      `I::Error` name, so the two are one stage, not two. What this stage did
      settle is the hook the emission plugs into — the name and the path. The
      open half of the bullet is therefore carried to S1, where the alias can
      be bound to something the file itself wrote.
- [x] the import follows the Rust convention rather than restating itself:
      `use a::b::Trait;`, and `use a::b::Other as Trait;` only where the name
      and the path's last segment differ. Writing `as` always makes a file that
      `cargo fmt` rewrites, and a kept file the formatter rewrites is a kept
      file the equality test fails
- [x] the walk module of the visitor carries the name in its own `use super`
      list: a module of its own sees nothing that stands outside it, and the
      bound of every descent is that name

**The harness lesson, which is the reason the count moved as much as it did.**
Reaching `input.rs`, `errors.rs` and `decode.rs` from the example by `#[path]`
compiles them a *second* time, so the example's `UnsynInput` and the crate's
are two traits with one name — and the generated file, which imports the
crate's, satisfied neither of the example's. Those three modules name no node
of a front end, so the example re-exports them from the crate instead of
copying them; only `read`, `semantic` and `codegen`, whose `super::syntax`
must mean the tree under test, are compiled again. This is what turned the
stage from a rename into the whole of the 338.

- [x] gates: shipped set green (111 + 5 + 11 + 1), `--workspace` 15 results
      green, clippy silent, `fmt` clean; **scoreboard 543 → 179**, and E0277
      is *gone* rather than reduced:

| errors left | what it means | taken out by |
|---|---|---|
| 109 | no variant by that name, or no such method | **S4** |
| 22 | a variant that does not have those fields | **S4** |
| 20 | no field by that name | **S4** |
| 13 | a shape mismatch | **S4** |
| 10 | no type `OneOrMore` (and its import) | **S4** |
| 4 | a pattern of the wrong arity | **S4** |
| 1 | unresolved import | **S4** |

A test now pins the half of D4 that a default cannot pin by example:
`a_build_aimed_at_another_crate_names_that_crate` in `tests/unsyn_us.rs`
configures the name and the path and asks that `UnsynInput` and `unsyn` appear
nowhere in what comes out — which is what lets a grammar live in a crate that
never depends on unsyn, and which the kept file, written with the default,
deliberately does not show.

## Stage 3 — attribute conventions

- [ ] emit `map_err = <SyntaxKind>::<Rule>.map()` per named rule type, and the
      `crucial` mark per alternative that reads a word, as the hand file writes
      them — this is what `Kind::Syntax("LitUnicode", …)` currently approximates
- [ ] serde derives behind the same `cfg_attr` the hand file uses
- [ ] gates: suite green

**What aligning them costs — the reason this was not done as a rename sweep.**
The hand tree has **none** of the eight types: `NegatedSet`, `Call`, `Paren`,
`Suffixed`, `RangeTo`, `RangeFrom`, `UnicodeRange`, `StrRange` are all absent
from `syntax.rs`/`lexer.rs` (checked by name against both files). The hand tree
instead inlines each production's parts into the variant, e.g.:

```rust
// hand
pub enum Range<I> { UnicodeRange(Box<LitUnicode<I>>, I, Box<LitUnicode<I>>), … }
// generated
pub enum Range<I> { UnicodeRange(UnicodeRange<I>), StrRange(StrRange<I>), … }
```

So closing E0026/E0023/E0308 is not renaming but **growing the hand tree by eight
node types**: each wants a struct, a hand-written parse, a visitor method, and
its ~47 consumer sites in `read.rs`/`semantic.rs`/`codegen.rs`/`visit.rs` rewired
from the inlined parts to one held node. That is a stage on its own — bigger than
S2 — and it is exactly what S5's "the hand tree gets the ten rules as rules"
line was pointing at. It was deliberately **not** started in a nearly-full
context rather than left half-applied with the gates red.

**The box rule, restated because it is easy to get wrong here:** of the four
variants that change shape, `NegatedSet`/`Paren`/`Suffixed` hold a boxed node
(they are on a containment cycle) and **`Call` holds a bare `Call<I>`** — it
reaches only `Ident`, a lexer rule, so it is on no cycle. The generator already
gets all of this right and `cyclic_syntax_refs` was not touched by any of this
work; an alignment must copy that result, never widen it.

## Stage 4 — one tree: node names, fields, types

The remaining 179 errors, and the real work. Ordered by blast radius.

### The mapping table, read off both trees (the precondition S4 lacked)

Generated names checked against `unsyn.us`: every one below marked *rule* is
the left side of a production the spec genuinely states, so the hand name is
the one that is wrong and the hand moves (D5). Marked *fallback*, the spec
states no rule and the generator substituted a place — those are P5's
naming-hygiene debt, and moving the hand tree onto them **fixes a defect by
forgetting a good name**, so they are held out of this stage.

| enum | hand → generated | kind |
|---|---|---|
| `ExprNoTopAlt` | `WithSuffix`→`ExprWithSuffix`, `WithoutSuffix`→`ExprWithoutSuffix` | rule |
| `ExprWithoutSuffix` | `Tilde`→`NegatedSet`, `Str`→`LitStr`, `Unicode`→`LitUnicode`, `Reference`→`Ident` (`Call`/`Paren`/`Set` already match) | rule |
| `ExprWithSuffix` | `Grouped`→`Suffixed` (`Concat`/`Except` match) | rule |
| `SetItem` | `Str`→`LitStr`, `Unicode`→`LitUnicode`, `Reference`→`Ident` (`Range` matches) | rule |
| `Range` | `Unicode`→`UnicodeRange`, `Str`→`StrRange` | rule |
| `RepeatBound` | `Count`→`LitDec` (`RangeTo`/`RangeFrom` match) | rule |
| `OuterDoc`/`InnerDoc` | `Block`→`OuterBlockDoc`/`InnerBlockDoc`, `Line`→`OuterLineDoc`/`InnerLineDoc` | rule |
| `Ident` (lexer) | `Raw`→`RawIdent`, `NonKeyword`→`NonKeywordIdent` | rule |
| `Keywords` | `Leading`→`LeadingKeywords` (`OperatorKeywords` matches) | rule |
| `Suffix` | `Star`/`Question`/`Plus`→`F1`/`F2`/F`3` | **fallback — P5** |
| `AsciiEscape` | `Hex`→`AsciiHexEscape`, `N`/`R`/`T`/`Backslash`/`Zero`→`F2`..`F6` | **mixed — P5** |
| `Crate` fields | `inner_docs`/`items` → `inner_doc`/`item` | rule-name, **singular** — style |
| `Expr` fields | `first`/`rest` → the generated element/tail names | **style — P5** |
| `Repeat.bound`, `Stmt.keyword`, `LitDec.head/tail`, `LitUnicode.digits` | rule-derived | rule |

`OneOrMore` (11 errors) is separate: the hand tree keeps a `OneOrMore<I, X>`
container for `X+`; the generator emits a `Vec` plus a first element instead.
Either the type is generated or the hand container is dropped — a shape
decision, not a rename.

### The split that decides what to do first

Renames whose generated name is a *rule* are unambiguous and do not freeze any
P5 defect: they are ~135 of the 179 and go first, hand side moving to the
generated spelling. `Suffix`/`AsciiEscape` place-names and the `Crate`
singular/`Expr` `first`/`rest` style points are **not** renames to make blindly
onto the hand tree — settling them is P5, and is left to the user.

### What was actually done, and a correction to this table

- [x] **all the *rule* variant renames above applied** to `syntax.rs`/`lexer.rs`
      and their references in `read.rs`/`semantic.rs`/`codegen.rs`/`visit.rs`
      + `unsyn_us.rs`, family by family. 179 → **149**.
- [x] `Crate.inner_docs`/`items` → `inner_doc`/`item`. This was **mis-shelved in
      this table as "singular style"** and is not: README §11 rule 1 says a
      reference to a rule names its field in `to_snake_case`, and `Crate ->
      InnerDoc* Item*` references the rules — so the singular name is
      rule-derived and the plural hand name is the wrong side. 149 → **134**.
      (`Stmt.keyword`→`leading_keywords`, `Repeat.bound`→`repeat_bound` are the
      same rule and went with it.)
- [ ] `Set.items`/`tail` are **not** in the rename set at all: the hand `Set`
      carries named fields where README §11 and the generated file both write
      `Set` as a *tuple*. That is a shape change, and `Expr` (README shows
      `pub struct Expr<I>(pub …, pub Vec<(…)>)`) is the same. The hand tree is
      the deviant side; moving it to tuples is legitimate S4, not P5.
- [ ] the variant shape: generated makes a single-reference alternative
      `Box<X<I>>` (one field), hand inlines the parts as named fields —
      "expected 1 field, found 4" (E0026/E0023) and the `&generated::LitStr` vs
      `&lexer::LitStr` E0308 all trace here. Stage-sized, `edit_file` only (the
      mass sed/python edits have corrupted this repo twice).

**Box is cycle-only, and the generated tree already gets it right — verified, not
assumed.** `cyclic_syntax_refs` keeps only edges whose tail `reaches` back to
the head, and the 16 emitted `Box`es land on exactly `Expr`, `ExprNoTopAlt`,
`ExprWithSuffix`, `ExprWithoutSuffix`, `NegatedSet`, `Paren`, `Suffixed` — the
mutual-containment core. The sibling alternatives of the very same enum carry
**no** box: `Call(Call<I>)`, `Set(Set<I>)`, `LitStr(LitStr<I>)`,
`LitUnicode(LitUnicode<I>)`, `Ident(Ident<I>)`. So box is not a convention
applied to tuple variants and must not be added while aligning shapes — the
only place it belongs is a pair the cycle analysis names. An earlier draft of
this table attributed E0026/E0023 to "generated boxes single-reference
alternatives"; that was the wrong diagnosis and is corrected here.

**What the E0026/E0023 shape gap actually is.** The spec now gives `NegatedSet`,
`Call`, `Paren`, `Suffixed`, `RangeTo`, `RangeFrom`, `UnicodeRange`, `StrRange`,
`AsciiHexEscape`, `BlockCommentBody` productions of their own (D5). So a variant
that *references* one of those is one field holding that node —
`NegatedSet(Box<NegatedSet<I>>)` — while the hand tree still **inlines** the
parts of that production as named fields of the variant (`NegatedSet { tilde,
s, operand }`). Aligning means the hand variant drops its inlined parts and
holds the node, boxed **only** for the three on the cycle. The inline-sequence
alternatives (`Concat`, `Except`) are the opposite case and stay named-field in
both trees.

**The shape direction is settled by the README, not open.** README §11 (lines
745–759) *literally prints* `pub struct LitStr<I>(…)`, `pub struct Expr<I>(pub
(…), pub Vec<(…)>)` and `pub struct Set<I>(…)` as tuples, and "a struct whose
fields are all nameless is written as a tuple". The generator already follows
that; the hand tree's named `Set { open, items, tail, … }`, named `Expr {
first, rest }` and named `OneOrMore<I,X>` are the deviation. So `Set`/`Expr`/
`LitStr` → tuple and `OneOrMore` → the generated `(X, Vec<Led<I,X>>)` are
**hand-tree moves the README mandates**, the same class as the renames — not a
decision to defer. What remains genuinely deferred is only the `Suffix`/
`AsciiEscape` place-names (P5) and the serde key change.

### Path A taken: the hand tree grew the eight rule nodes

This round does **not** bootstrap for real — the hand tree is grown to match the
generated one rather than the generated file replacing it. 134 → **105**.

The eight types were added to `syntax.rs` (`NegatedSet`, `Call`, `Paren`,
`Suffixed`, `RangeTo`, `RangeFrom`, `UnicodeRange`, `StrRange`) with the
generated tree's field names and order, and the four inlining variants now hold
a node instead. No hand-written parse was needed: the derive produces it from the
same `#[parserc(...)]` attributes the neighbouring structs already carry.

**Box held exactly to the three cyclic variants — verified on the tree, not
asserted:** `NegatedSet [BOX]`, `Paren [BOX]`, `Suffixed [BOX]`, and bare
`Call`, `Set`, `LitStr`, `LitUnicode`, `Ident`, `RangeTo`, `RangeFrom`,
`UnicodeRange`, `StrRange`, `LitDec`. `Call` stays bare because it reaches only
`Ident`, a lexer rule, so it sits on no containment cycle. `cyclic_syntax_refs`
was never touched.

**One field was deleted, and it was right.** Hand `RangeTo` carried four fields
where the generated tree has three: an extra run *before* the comma. That slot
is a double count — the run after `{` already belongs to `Repeat.s` (generated
`Repeat.f2`), and `Repeat`'s whitespace is greedy, so the extra field could only
ever be `None`. The visitor's "every character handed to exactly one `visit_text`"
assertion and the `two_trees` span parity both still pass, which is the evidence
the trivia is not lost. Recorded here because a CST may not drop source text on
a judgement call, and this one was checked rather than assumed.

**A real remaining difference this surfaced, not yet settled:** the generated
visitor hands an anonymous punct field to `visit_text`, while the hand `Visit`
keeps a hook per punct (`visit_tilde`, `visit_lt`, `visit_comma`, …). The two
visitors therefore do not have the same method set. Whichever side moves, it is a
visitor question, not a naming one.

### The remaining 105, by kind (measured)

| kind | count | next step |
|---|---|---|
| `ExprWithSuffix::Concat`/`Except` field names | 13 | pure rename, hand moves to generated (`expr_without_suffix`/`f1`/`f2`/`f3`/`expr_no_top_alt`) |
| `Set`/`Expr` named struct → tuple | ~11 | shape, touches `read.rs::members` + destructuring sites |
| `Suffix::Star/Question/Plus` place-names | 26 | **P5 hold** |
| `OneOrMore` container absent in generated | 11 | open decision |
| E0308, the two trees' generics | ~44 | follows from the rows above |

### `Concat`/`Except` names — done, 105 → **93**

E0026 cleared to zero. Done through rebinding patterns (`new: old`) so no
function body moved. Box neither added nor removed — re-checked on the tree:
still exactly `NegatedSet`, `Paren`, `Suffixed`.

**Five differences left deliberately, and that is the honest state.** Generated
has `Box<ExprWithoutSuffix<I>>` for `Concat/Except.expr_without_suffix` and for
`Except.f4` where the hand tree is bare, and spells the two keyword fields as
`I` + `keyword = …` where the hand tree uses its `Concat<I>`/`Except<I>` punct
types. Those are type/box changes, outside a names-only pass, and they now make
up most of the remaining E0308. `ExprWithSuffix` → `Except` → `ExprWithoutSuffix`
→ `ExprWithSuffix` **is** a containment cycle, so those boxes are most likely
right and the hand tree the loose side — but that is a sizing judgement to make
once and on purpose, not to slip in while renaming.

### The remaining 93, by kind (measured)

| kind | count | needs |
|---|---|---|
| `Suffix::Star/Question/Plus` place-names | 26 | a P5 ruling |
| E0308: box depth, and `&generated::LitStr<I>` vs `&lexer::LitStr<I>` | 49 | the five boxes above |
| `OneOrMore` absent from generated | 11 | a decision |
| `Expr::first`/`rest`, `Set::items`/`tail` | 7 | named struct → tuple (README §11) |

### Still open, carried forward

- [ ] **`OneOrMore` (11 errors, needs a call):** the hand tree has
      `OneOrMore<I, X>` for `X+`; the generator emits a `Vec` plus a separate
      first element and writes no such type. Either the generator emits
      `OneOrMore` as its own type, or the hand container is dropped and `X+` is
      modelled the generated way. A shape choice, not a rename.
- [ ] `Suffix`/`AsciiEscape` place-names: P5, held out deliberately.

Serde note: `Crate` derives `Serialize`/`Deserialize`, so the JSON keys became
`inner_doc`/`item`. No snapshot in the repo depends on it (suite green), but it
is an external format change.
- [ ] `OneOrMore`: generate the type or drop the hand container (decide)

**What S4 actually is, measured before any of it was written.** Grouping the
179 by message says they are not 179 decisions but a handful of causes:

| cluster | count | cause |
|---|---|---|
| variant named by what it *means* (`Star`, `Tilde`, `Grouped`, `Reference`, `With/WithoutSuffix`) | ~70 | the hand tree invented these; D5 says the spec names, so the hand tree moves — **or** the spec gains the rule, as the ten parts already did |
| a repetition field that is a bare tuple position (`Crate.1` vs `Crate.items`) | ~15 | **a generator bug, not a naming choice** — see below |
| `OneOrMore` absent from the generated file | 11 | the generator has no emission for `X+` as a named type; the hand tree's `OneOrMore<I, X>` is the shape the analyze phases are written against |
| pattern arity / shape mismatch (`expected 1 field, found 4`) | ~10 | follows from the two above |

**The repetition-field bug — FIXED.** `repetition_field` returned
`Field { name: None, .. }` unconditionally, so every repeated operand lost the
name its inner field had already been given, while the *other* route to a
bounded reference (`atom_field`, codegen.rs ~1753) kept it. The two routes
disagreed about one construct; the one that threw the name away was wrong, per
README §11. `repetition_field` now carries the inner name when the repetition
is of exactly one operand, and keeps nothing when it is of several — a
repetition of a group reads no single rule's node, so it has no rule to be
called by. `syntax Crate -> InnerDoc* Item*;` comes out as

```rust
pub struct Crate<I> { pub inner_doc: Vec<(Option<S<I>>, InnerDoc<I>)>, pub item: Vec<Item<I>> }
```

which is the braced, named shape the hand tree has always had.

**What it did not buy.** The scoreboard stayed at **179**. The `items`-cluster
errors were `no field items on type &generated::Crate<I>`, and that field is
now `item`, singular — so the count only moves when the hand side takes the
rule-derived name, which is the rest of S4, not this fix. The fix is still
worth its weight: it turned ~15 name errors into a rename rather than leaving
a structural difference (tuple against braced struct) that no renaming can
cross. It also removed the tuple-vs-struct half of the `expected 1 field,
found 4` arity errors.

**The remaining S4, measured again after the fix.** 27 sites over seven files
say `.items` / `inner_docs`, and the rest is the variant-name table below. The
hand side moves (D5), and `Crate` is the cheapest family to start from
because its generated half is now settled.

- [x] `repetition_field` keeps the name of the one operand it repeats
- [ ] `Crate`: hand `inner_docs`/`items` → generated `inner_doc`/`item`
- [ ] variant names the hand tree invented (`Star`, `Tilde`, `Grouped`,
      `Reference`, `With/WithoutSuffix`) → the spec-derived names, ~70 errors
- [ ] `OneOrMore` generated as a type of its own, 11 errors

- [ ] `OuterDoc`/`InnerDoc`: `Block`/`Line` → the rule names
- [ ] `SetItem`: `Str`/`Unicode`/`Reference` → `LitStr`/`LitUnicode`/`Ident`
- [ ] `Range`: `Unicode`/`Str` inline tuple → `UnicodeRange`/`StrRange`
- [ ] `ExprNoTopAlt`: `WithSuffix`/`WithoutSuffix` → `ExprWithSuffix`/`ExprWithoutSuffix`
- [ ] `ExprWithoutSuffix`: `Tilde`/`Str`/`Unicode`/`Reference` → `NegatedSet`/`LitStr`/`LitUnicode`/`Ident`, inline struct variants → the named rules
- [ ] `ExprWithSuffix`: `Grouped` → `Suffixed`; `Concat`/`Except` fields → `f1`/`f2`/`f3` + rule-derived names
- [ ] `Suffix`: `Star`/`Question`/`Plus` → `F1`/`F2`/`F3`; the `syntax_terminal!` types and the punct types they carry
- [ ] `OneOrMore` — 10 sites; the generator writes a `Vec` and a first element, so this type must be generated or the hand uses dropped (decide at S4)
- [ ] fields: `items` on `Crate`, `first`/`rest` on `Expr`, `body` on `LitStr`, `head`/`tail` on `LitDec`, `digits`, `bound`, `keyword`
- [ ] gates: `cargo check -p unsyn --features bootstrap --example bootstrap` —
      the count, staged down to **0**

## Stage 5 — close the loop

- [ ] the example asserts rather than prints: two generations equal, the spec
      read by its own parser
- [ ] the hand `lexer.rs`/`syntax.rs`/`visit.rs` become the generated file's
      checked-in copy, and `regen_unsyn_us.rs` gains a second output
- [ ] `two_trees.rs` retired or reduced to what it can still see (D5 said the
      hand tree gets the ten rules as rules — this is where that is finally paid)
- [ ] gates: full suite + the fixed point

## Stage 6 — clean up

- [ ] delete `examples/regen_unsyn_us.rs`, its job taken by the bootstrap example
- [ ] README: what a build writes now (`Error`, `Input`, the visitor, and the
      phase split), and that the analysis files carry no switch
- [ ] this log closed, its open items moved or deleted
