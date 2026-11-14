# unsyn 生成器改进计划（面向自举）

> 本文件是 `crates/unsyn` 的**施工单**，不是文档。每完成一项就把 `[ ]` 勾成 `[x]`；
> 如果一个阶段做了与本计划不同的决定，在对应的"决定"行里写明原因后再继续。
>
> 目标只有一个：让 `bootstrap.uns` 生成的 parser + CST **替代** `src/lexical/` 与
> `src/syntax/` 里手写的解析器（行为等价，而不是源码文本相同）。

## 0. 三条不可违反的前提

1. `bootstrap.uns` 是 uns DSL 的自描述，是**权威**。一切以它为准；只有当它出现
   逻辑漏洞、词法错误或 LL 错误时，才允许改规范本身（见 P0）。
2. 手写的生成器**只能**依据 DSL 编译后的 CST 产出代码，不得对"被描述的语言"
   作任何假设（字符集、标识符形状、关键字边界等等都不行）。
3. DSL 定义的是**产生式**；生成器的职责是把产生式翻译成
   `parserc::syntax` 的节点代数（`Delimiter` / `Punctuated` / `Option` / `Vec` /
   元组 / `Limits*`），而不是翻译成命令式的扫描代码。

## 1. 已核实的事实（供后续施工直接引用，勿重复推导）

**翻译目标（`crates/parserc/src/syntax.rs`、`crates/parserc/src/c.rs`）：**

| 名称 | 语义 | 备注 |
| --- | --- | --- |
| `Option<T>` | `T::ok()`，永不失败 | 对应 `T?` |
| `Vec<T>` | 循环 `T::ok()`，永不失败；`to_span` = 首∪尾 | 对应 `T*` |
| `Punctuated<T,P>` | 其 `parse` 恰为 `(T P)* T?` | 有 `len()`/`is_empty()` |
| `Delimiter<S,E,B>` | 缺 `E` 升级为 `ControlFlow::Fatal`；`to_span` = `S∪E` | 对应 `'x' B 'y'` |
| `Or<F,S>` | 二选一 | 与生成物里同名的 punct 类型冲突，见 P5 |
| `LimitsTo/Limits/LimitsFrom<T,..>` | 判定 `t.to_span().len()`，**数的是源文本单元，不是元素个数** | 只能用于无 trivia 的串 |
| 元组 ≤16 | `parserc_derive::derive_tuple_syntax!(16)` | 对应并列、也对应 `T+` |
| `parserc::keyword(KW)` | 只是 `starts_with`，**没有词边界** | 边界在手写 `define_keyword!` 里 |
| `take_till` / `take_until` / `take_while` / `take_while_with` / `next_if` / `next` | 字符级加速 | `take_until` 不消费关键字本身 |

**手写参考实现（自举要复现的行为）：**

- trivia 永远被**解析进树**并铺满源文本，从不"跳过"。
- `S<I>` = `take_while_with(1.., char::is_whitespace)`：**非空极大串**，所以
  `Option<S>` 才有意义。
- punct 拥有**两侧** trivia：`(Option<S>, I, Option<S>)`（`lexical/punct.rs`）；
  keyword 只拥有**尾侧**：`(I, Option<S>)`（`lexical/keyword.rs`）。
- 序列里两个操作数之间的空隙：`ExprNoTopAlts.rest: Vec<(Option<S<I>>, ExprNoTopAlt<I>)>`；
  `Expr.rest: Vec<(Or<I>, ExprNoTopAlts<I>)>` 不需要空隙，因为 `Or<I>` 自带两侧。
- 文件级：`Item::S(S<I>)` —— trivia 是**兄弟节点**。
- 节点代数已经在用手写树里：`Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`、
  `Tilde(Tilde<I>, Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`、
  `Call(Angle<I, Ident<I>>)`、`Paren(Paren<I, Box<Expr<I>>>)`、
  `Repeat` 包在 `Brace<I, Repeat<I>>` 里；`Brace/Bracket/Paren/Angle` 是
  `Delimiter` 的别名（`lexical/delimiter.rs`）。
- 13 个节点类型里 12 个用 `#[derive(Syntax)]`，只有 `Crate` 手写 `parse`/`to_span`。

**当前生成物的实测状态**（`Codegen::new().with_externals(XID_Start/XID_Continue)`
作用于 `bootstrap.uns`，102,698 字节 / 61 个类型，0 错误）：

| 计数 | `Delimiter` 0 · `Punctuated` 0 · `Limits*` 0 · `Char<` 0 · `take_till` 0 · `take_until` 0 |
| --- | --- |
| 已有的 | `keyword` 41 · `next_if` 37 · `take_while` 5 · `pub struct` 54 · `pub enum` 7 |
| 结论 | **字符级（lexer）已经用上内置加速函数**；缺陷集中在**节点级（syntax）** |

具体偏差（可直接对照生成文本）：

- `WHITESPACE<I>(pub I)` 一次只吃**一个**字符，靠 `Vec<WHITESPACE<I>>` 拼成一串；
  不是参考实现里的"非空极大串"。
- `Stmt` 为 5 个产生式元素生成了 **9 个字段**（4 个 `::std::vec::Vec<WHITESPACE<I>>`）。
- `ExprWithoutSuffix::ExprWithoutSuffixBracketStart` 把 `'[' ( SetItem ',' )* SetItem? ']'`
  展平成一串字段（`BracketStart, Vec<(Vec<WS>, ExprWithoutSuffix1)>, Vec<WS>,
  Option<SetItem>, Vec<WS>, BracketEnd`），其中 `ExprWithoutSuffix1<I>` =
  `(SetItem<I>, Vec<WHITESPACE<I>>, Comma<I>)` —— 这就是 `Punctuated<SetItem, Comma>`。
- `Repeat` = `(BraceStart, Vec<WS>, Repeat1, Vec<WS>, BraceEnd)`，而参考是
  `Brace<I, Repeat<I>>`。
- `syntax Expr -> ExprNoTopAlt+ ('|' ExprNoTopAlt+)?` 里的 `ExprNoTopAlt+`
  被**就地重复内联**（`Expr.0` 与 `Expr1.1` 各一份）。注意：规范里本来就没有
  `ExprNoTopAlts` 规则，它是手写树为复用而引入的；生成物内联成元组即可，
  **不需要**造这个名字（见 P3 决定）。
- `enum Repeat1` 出现 `Repeat1DEC_LITERALVariant3` 这种"同名加序号"的变体名；
  `ExprWithSuffixExprWithoutSuffixVariant2/3/4` 同理。可读性问题，非正确性问题。
- 生成的 punct 类型叫 `Or`/`Comma`/`Semi`/`Minus`/`Tilde`/`Star`，与
  `parserc::syntax::Or` 同名；生成代码里凡是用到代数节点都必须全限定（P5）。
- `{n,m}` 目前是手写 `loop` + `__items.len()` 比较，失败报
  `Kind::Syntax(name, ..)`。**这是有意选择**：`Kind::Limits*` 变体不带名字，
  具名的报错信息更好。保持不变，只在 P4 记录理由。

**`bootstrap.uns` 自身的三个缺陷**（属于"逻辑漏洞/词法错误"豁免范围，P0 处理）：

1. `:3-14` whitespace 集与 `char::is_whitespace()` 不一致。实测：`U+200E`、`U+200F`
   的 `is_whitespace()` 为 **false**（规范却列了它们）；而 `U+00A0`、`U+1680`、
   `U+2000`、`U+200A`、`U+202F`、`U+205F`、`U+3000` 都是 **true**（规范漏了）。
2. `:19` `( XID_Start | _ )` 与 `:76` `( DEC_DIGIT | _ )*` 里的 `_` 想表达**下划线
   字符**，但 DSL 把裸 `_` 定义为**通配符**（`WILDCARD = "_"`，"匹配任意一个字符"，
   `semantics/graph.rs:26-31`）。后果已在生成物中测得：
   `IDENTIFIER_OR_KEYWORD` = `XID_Start.or(next_if(|_| true))`（第一个分支变成死代码，
   任意字符都能开头）；`DEC_LITERAL` = 一个数字后接 `take_while(|_| true)`（一路吃到
   EOF，`{1,2}` 这种写法会被吞掉）。参考实现是 `'_'`：`lexical/ident.rs:28`
   `c == '_' || is_xid_start(c)`，`lexical/lit.rs:282` `is_ascii_digit() || c == '_'`，
   `semantics/value.rs:75` `replace('_', "")`。
3. `except` 在多字符操作数上的语义。现实现（`codegen/pattern.rs:311` 的字符类分支
   是正确的减法；`codegen/render.rs:319` 的多字符分支）是"**在目标起始处探测**被减去
   的语言"。于是 `IDENTIFIER_OR_KEYWORD except KEYWORDS` 会把 `lexerX` 也拒绝掉，
   而参考实现只拒绝**整个词恰好是关键字**的情形（`lexical/ident.rs:33-41` 比较
   `content.as_str()`）。把 `except` 定义为**"目标实际吃掉的那段文本不属于被减语言"**
   （整串差集，而不是前缀探测）即可复现参考行为，同时**取消**了"关键字边界需要写进
   规范"这条待办 —— DSL 里也就无需负向前瞻。

---

## P0 — 规范与决定（先落文字，再动代码）

- [x] **whitespace 集**：`bootstrap.uns:3-17` 改成与 Unicode `White_Space` 一致，用 range
      压缩成集合写法：
      `whitespace WHITESPACE -> [ U+0009-U+000D , U+0020 , U+0085 , U+00A0 , U+1680 , U+2000-U+200A , U+2028 , U+2029 , U+202F , U+205F , U+3000 ];`
      决定：**生成器仍不得引用 `char::is_whitespace`**，它只把规范列出的字符编译成字符类。
      注意 range 只能作为 `SetItem` 出现（`Range` 不在 `ExprWithoutSuffix` 里），所以顶层
      交替写法 `U+0009-U+000D | …` 是不合语法的，必须用集合。
      改后生成物实测：`WHITESPACE` 的字符类 = `'\t'..'\r' | ' ' | \u{85} | \u{a0} | \u{1680} |
      '\u{2000}'..'\u{200a}' | \u{2028} | \u{2029} | \u{202f} | \u{205f} | \u{3000}`，
      与手写 `lexical/s.rs` 的 `is_whitespace()` 恰好相同（25 个码点）。
- [x] **`_` → `'_'`**：修改 `bootstrap.uns` 的 `IDENTIFIER_OR_KEYWORD` 与 `DEC_LITERAL`。
      裸 `_` 保持"任意一个字符"的含义（`tests/data/spec.rs` 同时用了 `_` 和 `UNDER -> '_'`，
      证明 DSL 本来就区分这两者）。
      改后生成物实测：`IDENTIFIER_OR_KEYWORD` = `XID_Start.or(next_if(c == '_'))` 再接
      `XID_Continue*`；`DEC_LITERAL` = `DEC_DIGIT` 再接
      `take_while(|c| ('0'..='9').contains || c == '_')` —— 与 `lexical/ident.rs:28`、
      `lexical/lit.rs:282` 一致。
- [x] **`except` = 整串差集**：**决定**记录在此；文档注释与实现一起放到 P4 改，避免文档
      先于代码说谎。判定的可实作形式：把目标吃掉的那段文本 `__from.split_to(__taken)`
      作为**独立输入**跑被减语言，能整段吃完才算命中（在起始处探测并只比长度是不健全的，
      因为被减语言可能有长短不同的多个匹配）。
- [x] 决定：`whitespace` 节点**不是一等产生式**，它的 body 只被读成一个字符类，生成的
      类型按 P0 的归属规则是 trivia 本身，因此它自己不拥有 trivia（否则无限递归）。
- [x] 决定：自举的验收标准是**行为等价**（同一批 `.uns` 上手写 parser 与生成 parser 的
      CST 与错误位置一致），不要求类型名/别名/字段数相同。因此 `Bracket`/`Angle` 这些
      `Delimiter` 别名不生成，`ExprNoTopAlts` 这个名字也不造（内联成元组即可）。
- [ ] **trivia 归属规则**写进 DSL 文档（`codegen/types.rs` 的 `# Whitespace` 小节 +
      `lib.rs`）：**→ 已归入 P2**，与实现同一批改，否则文档会描述一个还不存在的行为。
      规则文本先记在这里；它只依赖被描述语言的**产生式结构**，不依赖任何字符集形状：
      - 一个终结符若其模式表示**有限个固定字符串**（`'<'`、`';'`、`'->'`、`'lexer'`），
        它拥有**自身两侧**的 trivia 串：`Option<WS>` 前、`Option<WS>` 后；
      - 声明为 `whitespace` 的那个节点自身就是 trivia，永远不拥有 trivia；
      - 其余终结符（字面量、通配、外部调用、集合、补集、含这些的重复）不拥有 trivia；
      - 两个"不拥有 trivia"的操作数之间的空隙，由序列节点显式带一个 `Option<WS>`。
      这个规则在参考实现里就是 punct 的"两侧"与 keyword 的"尾侧"；把 keyword 也统一成
      两侧是允许的，因为解析是顺序的、`S` 是非空极大串，同一串 trivia 不可能被吃两次。
      - [x] 决定记录：该规则需要"该终结符是否只表示有限个固定串"这一判定，它是从
            `TokenPattern`/`Step` 结构上可判定的（`Alt`/`Seq` 只由 `Lit` 与有限 `Class`
            构成），**不是**对目标语言字符集的假设。
- [x] P0 完成后跑门：见本文件末尾的「门」小节（规范文本改动会影响生成物 —— 字符集与
      两处引用变了，生成体积 102,698 → 102,834 字节 —— 但不影响任何已提交的 fixture，
      因为 fixture 是从 `tests/data/spec.rs` 生成的）。

## P1 — IR 承载节点代数

> **P0 已完成**（门全绿：`--all-features` 162/3/17/1/21/3；`--workspace` 15 个 ok；
> `runtime` 2/0；`codegen` 全绿；三组 clippy 在 `crates/unsyn` 里 0 条；
> 四组 `RUSTDOCFLAGS=-D warnings cargo doc` 干净；`cargo fmt --all --check` 干净）。

- [ ] **阶段划分修正**：P1 / P2 / P3 实际上是**一个不可拆的改动**。因为：新的 IR 形状
      （`Gap`、`Punctuated`、`Delimited`）存在的唯一理由就是承载 P2 的归属规则；
      而 `Punctuated<T,P>` / `Delimiter<S,E,B>` 能成立的前提又是 P2 把 trivia 归给
      `P`/`S`/`E` 这些 punct。先做 P1 会产出死代码，先做 P2 会没有形状可渲染。
      所以下面三个阶段当成一次重构推，**中途不追求门全绿**，只在三者都完完后跑一次
      完整门表（包括 `mini.rs` fixture 重生与 `tests/generated.rs` 断言修正）。
      已拆小的可先行项：`tests/data/spec.rs` 里补上形状齐全的用例（逗号分隔的集合、
      嵌套定界符、`T+` 带间隙），这样重构一完成就能被 fixture 测到。
- [ ] `codegen/pattern.rs`：给 `Step` 增加/识别 `Delimiter{open, body, close}`、
      `Punctuated{item, sep}` 所需的信息（或在 `types.rs` 侧识别，见下一项），
      并实现 `denotes_finite_literals()` 供 P0 的归属规则使用。
- [ ] `codegen/types.rs`：`FieldType` 增加
      `Punctuated{item, sep}`、`Delimited{open, body, close}`、
      `Run{item, min, max}`（元素计数语义）、`Gap`（`Option<WS>`）表达；
      `TypeDefKind::Bounds` 与新的 `Run` 合并，避免两套上界循环。
- [ ] `OneOrMore` 改成 `(T, Vec<(Gap, T)>)` 形态（元组本身就是 syntax），
      不再产出 `Vec` + `is_empty()` 检查。
- [ ] 单测：每条 IR 规则一个用例，断言"识别出的形状"而不是断言渲染文本
      （渲染文本在 P2/P3 会变）。
- [ ] 门：`cargo test -p unsyn --all-features -- codegen` 全绿。

## P2 — trivia 归属改写（一次改完，不留兼容层）

- [ ] `whitespace` 节点生成成**非空极大串**：`struct WHITESPACE<I>(pub I)` +
      `take_while_with(1.., <规范列出的字符类>)`。
- [ ] 把 P0 记下的 **trivia 归属规则**写进 DSL 文档（`codegen/types.rs` 的 `# Whitespace`
      小节 + `lib.rs`），与下面两条实现同时落地。
- [ ] 固定字面终结符生成成 `(Option<WS>, I, Option<WS>)`，`to_span` 只到自身字符结束
      （参考：`lexical/punct.rs` 的 `the_span_of_a_punct_ends_with_its_own_characters`）。
- [ ] 删除 `Field::separator` 与所有 `Vec<WHITESPACE<I>>` 字段；只在规则要求的位置插入
      `Option<WS>` 空隙。
- [ ] `tests/data/mini.rs` 重新生成（`cargo test -p unsyn --all-features --test dump
      -- --nocapture` → `rustfmt --edition 2024` → 覆盖 fixture），
      并检查 `tests/generated.rs` 的断言：`A;B` 与 `A ; B` 必须解析成同一棵树。
- [ ] 门：全部门 + `dump` fixture 与生成字节一致。

## P3 — 结构识别（把产生式形状映射到代数节点）

> **前置已核实**（探针：`syntax ARGS -> '(' (IDENT ',')* IDENT? ')';` /
> `syntax CHOICE -> '[' (IDENT ',')* IDENT? ']';` /
> `lexer OPEN -> '('; lexer CLOSE -> ')'; syntax GROUP2 -> OPEN IDENT CLOSE;`）：
>
> - 产生式里**内联的字面标点能正常工作**，且不跟声明的 lexer 冲突：`'('` → `ParenStart`、
>   `')'` → `ParenEnd`、`','` → `Comma`、`'['`/`']'` → `BracketStart`/`BracketEnd`。
>   但要注意：`lexer OPEN -> '('` 与内联 `'('` 生成的是**两个不同的类型**
>   （`OPEN<I>(pub I)` 和 `ParenStart<I>(pub I)`，匹配同一个字符）—— 字面值共享表
>   不查已声明的规则。bootstrap.uns 不依赖这一点，但 P5 可以把它收成同一个类型。
> - `(T P)*` 的当前产物：`ARGS1<I>(pub IDENT<I>, pub Vec<WS<I>>, pub Comma<I>)` 加上外层
>   `Vec<(Vec<WS<I>>, ARGS1<I>)>`；整个 `ARGS` 是 6 个字段。目标形状：
>   `Delimiter<ParenStart, ParenEnd, Punctuated<IDENT, Comma>>`（**1 个字段**）。
> - 同一个 `(T, WS, P)`  Anonymous 类型在 `ARGS1` 与 `CHOICE1` 里**各生一份**
>   （`shared` 表只覆盖外部调用与通配符）。

- [ ] `(T P)* T?` → `Punctuated<T,P>`。识别点：`bootstrap.uns:122`、`:125`
      两处 `'[' ( SetItem ',' )* SetItem? ']'`。
- [ ] `'x' B 'y'`（两侧是固定字面终结符的并列）→ `Delimiter<X, Y, B>`。识别点：
      `:122`/`:125` 的方括号、`:123` 的 `'<' IDENTIFIER '>'`、`:124` 的 `'(' Expr ')'`、
      `:144-152` 的花括号。别名（`Bracket`/`Angle`/`Paren`/`Brace`）**不生成**，
      直接内联 `Delimiter<..>`；决定：自举要求的是行为等价，不要求类型名相同。
- [ ] `T?`/`T*`/`T+`/`T{n,m}` → `Option` / `Vec` / 元组 / 计数检查（顺序处理）。
- [ ] `<NAME>` 在 syntax 规则里只是被记录的文本（`Angle<I, Ident>`），
      在 lexer 规则里才展开成外部函数调用；两条路径都要有测试钉住。
- [ ] 门：`cargo test -p unsyn --all-features` 全绿；对 `bootstrap.uns` 重新生成一次，
      `Punctuated`/`Delimiter` 计数必须 > 0（当前是 0）。

## P4 — `except` 与加速函数

- [ ] 多字符 `except` 改为整串差集（P0 的决定）：在被减语言上跑**目标吃掉的那段文本**，
      整段吃完才拒绝；字符类分支保持现在的 `item.and(removed.negated())`。
- [ ] 同时改 `syntax/expr.rs` 的 `Except` 文档注释（现在写的是
      "tested at the position the target starts at"）与 `codegen/pattern.rs::Step::Except`、
      `render.rs::asserted` 的注释。
- [ ] **必须翻过来的既有断言**：`tests/generated.rs:211`
      `assert!(mini::IDENT::parse("lett x").is_err())` 是把"前缀探测"钉住的测试；
      `except` 改成整串差集后 `lett` 必须是合法 `IDENT`，这条断言要改成 `is_ok()`，
      并补 `"let x"`/`"fn x"` 仍被拒绝、`"xlet"` 被接受的用例。
- [ ] 复核 `~[X]*`：现在生成 `take_while(|c| !class(c))`，与 `take_till(class)` 等价；
      统一选一个并在文档里写明理由（倾向 `take_while`，因为它能带 `1..` 上界形式）。
- [ ] 关键字边界：确认取消规范侧的额外规则后，`NON_KEYWORD_IDENTIFIER` 的生成物
      对 `lexer`、`lexerX`、`xlexer` 三个输入的行为与 `lexical/ident.rs` 一致，
      并加测试。
- [ ] 保留 `{n,m}` 报 `Kind::Syntax(具名)` 而不是 `Kind::Limits*`（后者不带名字）；
      在 `render.rs::bounds` 的文档注释里补一句这个取舍。
- [ ] 门：新增行为测试全绿（`lexerX` 这一类必须用 `analyze` + 生成代码在
      `tests/generated.rs` 里编译后实测，而不是只看生成文本）。

## P5 — 命名卫生

- [ ] 生成器内部引用代数节点一律全限定（`::parserc::syntax::Or`、`Delimiter`、
      `Punctuated`、`Limits*`），避免与生成的 punct 类型 `Or`/`Tilde`/`Comma` 冲突。
- [ ] 变体名冲突（`Repeat1DEC_LITERALVariant3`）：改成从"该变体独有的形状特征"派生
      稳定名字，而不是加 `Variant{n}` 后缀。
- [ ] 门：`cargo test -p unsyn --all-features` + `RUSTDOCFLAGS=-D warnings cargo doc`。

## P6 — 自举验收（本计划的终点）

- [ ] 搭一个 build 阶段：用 `codegen` 从 `bootstrap.uns` 生成 parser，编译进测试
      crate（`runtime`/`codegen` 两个 feature 已经分开，build 依赖放
      `[build-dependencies]`）。
- [ ] 等价性测试：对 `bootstrap.uns` 与 `crates/unsyn/**/*.uns`、
      `tests/data/*.uns`（含故意写坏的样本）**同时**跑手写 parser 与生成 parser，
      断言 CST 与错误位置一致；不一致就回到 P1–P5 修生成侧或改规范。
- [ ] 全部等价后，才替换 `src/lexical/` + `src/syntax/` 的手写实现。
- [ ] 门（替换后）：`cargo test --workspace`、`--no-default-features --features runtime`、
      `--features codegen`、三组 clippy、四组 rustdoc、`cargo fmt --all --check`。

## 门（每个阶段结束都跑一遍）

| # | 命令 | 期望 |
| --- | --- | --- |
| 1 | `cargo test -p unsyn --all-features` | 全绿（lib / bootstrap / codegen / dump / generated / doctests） |
| 2 | `cargo test --workspace` | 所有 `test result: ok` |
| 3 | `cargo test -p unsyn --no-default-features --features runtime` | 全绿 |
| 4 | `cargo test -p unsyn --features codegen` | 全绿 |
| 5 | `cargo clippy -p unsyn --all-targets`（三组 feature 组合） | `crates/unsyn` 里 0 条 |
| 6 | `RUSTDOCFLAGS=-D warnings cargo doc --no-deps`（四组） | 0 警告 |
| 7 | `cargo fmt --all --check` | 干净 |

## P7 — 收尾

- [ ] 删除本文件里已经过时的"实测状态"数字，或把它们移到 `CHANGELOG.md` 的条目里。
- [ ] `crates/unsyn/CHANGELOG.md` 增补 unsyn 的条目（目前为空）。
- [ ] 已知的既有杂物（非本任务引入）：`[dev-dependencies] walkdir` 未被使用。

---

## 附：施工时必须避开的坑（都已踩过）

- `edit_file` 会做**模糊匹配并自动 rustfmt**：结构性改动后要用 `git --no-pager diff`
  复核；`old_text`/`new_text` 里**绝不要**混用 `//!` 与 `///`（曾把 `codegen/mod.rs`
  改成 23 个 E0753）。
- `read_file` 的 `start_line`/`end_line` 参数在本环境会报
  `invalid type: string, expected u32`，改用 `sed -n 'A,Bp'`。
- 判断一次 cargo 运行成功与否要看完整输出（`| tail -n 30`），不要
  `| grep -E ...`：编译失败时 grep 会输出空，骗过两次。
- 检查生成代码要**先把所有空白去掉**再匹配（`proc_macro2` 的 spacing 会让
  `pub struct Expr` 匹配不上 `pubstructExpr`，反之亦然）。
- 不要重新引入对目标语言字符集的假设（`CharClass::XidContinue`、字面量的词形状守卫、
  内置外部实现、`runtime/keyword.rs` 都已删除，且都有测试钉住）。
- 不得改 `.git`（沙箱里 Git 元数据是只读的）。
