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
- [x] **trivia 归属规则**写进 DSL 文档（`codegen/types.rs` 的 `# Whitespace` 小节 +
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

## P1–P3 — 一次重构：IR 承载节点代数，trivia 归给终结符，形状由结构识别

> 这三段不可拆：新形状（`Gap`/`Punctuated`/`Delimited`）存在的唯一理由就是承载归属规则，
> 而 `Punctuated<T,P>`/`Delimiter<S,E,B>` 能成立的前提又是 punct 自己吃掉两侧的 trivia。
> 当成一次施工，中途不追求门全绿，只在完成后跑一次完整门表。

> **已完成**（门表全绿）。归属判定落在 `codegen/mod.rs` 的 `ownership()`，空隙与两个折叠
> 落在 `codegen/types.rs` 的 `Trivia`（settle 阶段），三种终结符形状落在 `codegen/render.rs`
> 的 `token_pattern()`，两个贪心 body 落在 `codegen/scan.rs`。文档小节按实际内容命名为
> `# Trivia`（`codegen/types.rs`、`codegen/render.rs`），DSL 侧另有一节
> `# Where the trivia goes`（`lib.rs`）。除下面留的一项外，本节的验收见
> `tests/codegen.rs::a_bootstrap_spec_is_read_as_the_shapes_it_writes`。

### 归属规则（同时写进 `codegen/types.rs` 的 `# Whitespace` 小节与 `lib.rs`）

- [x] 一个终结符**拥有 trivia** 当且仅当四条都成立。判定全部从 `Step` 的结构得出，不看
      被描述语言的字符集形状：
  1. 它的模式表示**有限个固定的非空串**（`Step::fixed_strings`：只由 `Lit`、
     `Class(Chars/Or)`、`Seq`、`Alt` 组成；遇到 `Rep`/`Ref`/`External`/`Not`/`Except`/
     `Followed`/`Any`/超过上限的类就放弃归属）。注意 `pattern::literal` 把单字符字面值
     编译成 `Class`，所以 `'('` 是 `Class('(')` —— 必须接受 `Class(Chars)`，否则标点全都不是 punct。
  2. 它不是文件声明的 `whitespace` 节点（否则自己吃自己的串，无限递归）。
  3. 它没有被任何**词法模式**直接 `Step::Ref`：`Ref` 编译成 `input.parse::<T>()?`，
     被引用的类型若拥有 trivia，会把两侧的串吃进父 token 自己的文本里 ——
     `QUOTE_ESCAPE -> '\\\''` 若拥有 trivia，`'a\ ''` 就成了合法字符串；
     `LEADING_KEYWORDS` 被 `KEYWORDS` 引用，因此也不拥有。
  4. 它能匹配的字符与 `whitespace` 的字符集**不相交**。反例：把 `' '` 声明成内容 token
     的规则，拥有前导串就会让"极大串先吃光"抢掉它自己要匹配的字符。
- [x] 拥有 trivia 的终结符渲染成 `struct X<I>(pub Option<WS<I>>, pub I, pub Option<WS<I>>)`，
      `to_span` = 前导 ∪ 自身（**不含尾串**，否则兄弟节点区间重叠），与手写
      `lexical/punct.rs` 同形；`whitespace` 节点渲染成**一个非空极大串**
      `struct WS<I>(pub I)` + `take_while_with(1.., class)`（模式算不出字符类时退化成"整个
      模式贪心循环"），与手写 `lexical/s.rs` 同形；其余终结符仍是 `struct X<I>(pub I)`。
      手写的 keyword 只拥有尾侧，这里统一成两侧：串是极大且贪心的，同一串不可能被吃两次，
      所以接受语言不变，只有归属位置变。
- [x] **空隙**只在"两侧都不拥有 trivia"的两个操作数之间出现，类型是 `Option<WS<I>`（不再
      是 `Vec<WS<I>>`：`WS` 自身已经是极大串）。
- [x] 这些判定连同结构识别都放在 **settle 阶段**（`Plan` 把全部类型建完之后）：匿名 token
      类型是边建边产生的，规划期还不知道最终有哪些类型拥有 trivia；而且 `Ref` 关系要看到
      全部词法模式才能算。于是 `render.rs` 里不再有任何 trivia 决策。
- [x] `X+` 不再渲染成 `Vec` + `is_empty()`，而是 `(X, Vec<(间隙, X)>)`：`FieldType::OneOrMore`
      从 IR 删掉，`repeated()` 直接产出 `Concat` + `Many`。与手写
      `ExprNoTopAlts{first, rest}` 同形，也少一次运行时检查。
- [x] `TypeDefKind::Bounds`（`X{n,m}`）**保留**：`LimitsTo`/`LimitsFrom` 量的是
      `to_span().len()`（源码单位，不是元素个数），且它们的 `Kind` 不带名字，报告会变差。
      `Limits*` 有意不使用。

### 结构识别（把产生式形状映射到代数节点）

- [x] **分组内联成元组**：`(...)` 内若只有一条序列且操作数不超过 `MAX_TUPLE / 2`（空隙最坏
      让 arity 翻倍），直接产出 `Concat` 链，不再造匿名 struct。这样 `( SetItem ',' )` 在 IR
      里是 `Concat{SetItem, Comma}`，才能被下一步识别；顺带消掉 `ARGS1`/`CHOICE1` 那种
      逐字重复的匿名类型。
- [x] **`Punctuated`**：字段序列里相邻的 `Many{Concat{A,B}}` + `Opt(A')`，`A'` 与 `A`
      结构相等（比较**已建好的** `FieldType`，绝不重建类型来比 —— 重建会产生重复匿名类型），
      且 `B`、`A` 之间不需要空隙 → `Punctuated<A,B>`。命中 `bootstrap.uns:122`、`:125`。
- [x] **`Delimiter`**：在字段序列上取**最窄**窗口 `[i..=j]`（同宽取最左）：`fields[i]`、
      `fields[j]` 都是拥有 trivia 的 `Named`，两者表示的文本不同，`j ≥ i+2`，且窗口内不含与
      close 同文本的拥有类型（否则 `'(' A ')' '(' B ')'` 会被吃成一个跨两个括号的大窗口）→
      `Delimited{open, body, close}`，折完一个再在剩下的序列上找下一个，body 内部同样递归。
      "文本不同"顺带排除 `TEXT -> QUOTE … QUOTE`；识别**不查括号配对表**（那会是对目标语言
      的假设）。命中 `:122`、`:123`、`:124`、`:125`、`:144-152`。
      ◆ 本项原写"最左最短"，是错的：`'~' '[' … ']'` 里 `]` 同时够得着 `~` 和 `[`，最左的
      `~` 会把 `]` 抢走，折出 `Delimited<Tilde, BracketEnd, …>` —— 把 `~` 和 `]` 当成一对，
      而产生式从没这么说过。改成最窄，得到
      `Tilde` + `Delimiter<BracketStart, BracketEnd, Punctuated<SetItem, Comma>>`，与手写
      `syntax/expr.rs:335` 的 `Tilde(Tilde<I>, Bracket<I, Punctuated<…>>)` 同形。
- [x] `eats_leading`/`eats_trailing`：`Named` 查归属表，`Concat` 看首/尾，`Delimited` 看
      open/close，`Opt`/`Many`/`Punctuated`/`Gap` 一律 false（它们可以什么都不吃，不能指望）。
      宁可多一个 `Option<WS>` 字段，不可少一个。
- [ ] `<NAME>` 在 syntax 规则里只是被记录的文本（`Angle<I, Ident>`），在 lexer 规则里才展开成
      外部函数调用；两条路径都要有测试钉住。
      现状：两条路径没有分开 —— `types.rs` 的 `call()` 把每一处 `<NAME>` 都做成共享的
      `ExternalNAME<I>` token，其 `parse` 调用表里给的函数，只有 lexer 那条有测试
      （`spec.rs` 的 `MATCHER -> <WORD_START>`）。`bootstrap.uns` 只在 `lexer` 里写 `<NAME>`
      （`:22`、`:24`），所以自举不受影响。syntax 位置上的 `<NAME>` 该记文本还是该调用，
      是一个还没做的决定，留到 P5 一起判。
- [x] `tests/data/spec.rs` 补形状：`(T P)* T?`、带空隙的 `T+`、拥有 trivia 的 punct 相邻处
      不留字段，让 fixture 一上来就覆盖新形状。

### 落地清单

- [x] `codegen/pattern.rs`：`Step::fixed_strings`、`CharClass::fixed_strings`、`Step::refers`。
- [x] `codegen/types.rs`：`FieldType` 加 `Gap`/`Punctuated`/`Delimited`，删 `OneOrMore` 与所有
      `separated`；删 `Field::separator`、`runs`、`opens_with_a_separator`；`group()` 内联；
      新增 settle；`# Whitespace` 文档重写。
- [x] `codegen/mod.rs`：`Plan` 存 `owning`（拥有 trivia 的 token → 它表示的串）与 `whitespace`；
      `plan()` 末尾跑 settle。
- [x] `codegen/render.rs`：`token_pattern` 三种形状；`ty`/`read` 支持新形状；`parts` 不再插
      separator；单测随形状一起改。
- [x] `codegen/scan.rs`：`Emitter` 增加"带前后 trivia 的 body"与"贪心循环 body"两个入口，
      与现有 `parse_body` 共用同一个 `split_to` 外壳。
- [x] 重生 `tests/data/mini.rs`（`cargo test -p unsyn --all-features --test dump -- --nocapture`
      → `rustfmt --edition 2024` → 覆盖 fixture），修 `tests/generated.rs`：`A;B` 与 `A ; B`
      必须解析成同一棵树。
- [x] 验收：**每条产生式折出的形状，与手写 parser 里同一条产生式用的形状一致**。
      这一条原先写的是"`Stmt` 字段数下降、重复匿名类型减少"，那是我自己加的指标，不是
      要求：P0 定的验收标准是行为等价，且明写"不要求类型名/别名/字段数相同"（`:131`）。
      拿字段数当指标还有更坏的一层：它给上面 `Delimiter` 那个真错打了高分 —— 窗口取得越
      贪就越"省字段"，所以这个指标只会把错误方向奖励成成绩，把 `~` 和 `]` 配成一对这种
      形状错误完全看不见。现在核对的四条（`bootstrap.uns:123-126` 对
      `syntax/expr.rs:335-350`）：
      - `'~' '[' … ']'` → `Tilde(Tilde, Delimiter<BracketStart, BracketEnd, Punctuated>)`
        同形于手写的 `Tilde(Tilde<I>, Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`；
      - `'[' … ']'` → 同形于手写的 `Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`；
      - `'<' IDENTIFIER '>'` → 同形于手写的 `Call(Angle<I, Ident<I>>)`；
      - `'(' Expr ')'` → 同形于手写的 `Paren(Paren<I, Box<Expr<I>>>)`；
      - `stmt: LEADING_KEYWORDS IDENTIFIER '->' Expr ';'`（`:167`）→ 生成的是
        `Delimiter<ArrowRight, Semi, Expr>`，手写原先是 `arrow_right, expr, semi` 三个
        平铺字段 —— **已改手写的一侧**：`syntax/expr.rs` 的 `Stmt` 三个变体现在都是
        `definition: Delimiter<ArrowRight<I>, Semi<I>, Expr<I>>`，`Definition::expr`
        读 `definition.body`。改完既有断言全部原样通过（包括
        `a_statement_without_a_semicolon_is_told_where_one_belongs`）：缺 `;` 的错误仍由
        `Expr` 体内的 `map_expr_err` 先报，位置不变；另外把四种缺 `;` 的写法（EOF、`)`、
        关键字、`|`）的 `Punct(Semi, Fatal, …)` 补成了永久断言。
        同一条规则还要求 `Stmt` 是 **struct** 而不是按关键字分三的 enum，见下面 P6 那一
        条（已按 `LeadingKey` 解决）。
      关于"缺闭合符报 `Fatal`"：手写 `lexical/delimiter.rs:18` 的 `Bracket` 本就是
      `parserc::syntax::Delimiter`，而 `Delimiter::parse` 对 `End` 加了 `.fatal()`
      （`parserc/src/syntax.rs:209`），`delimiter.rs:82` 的既有断言就是 `ControlFlow::Fatal`。
      所以那是后端与手写实现的既有行为，不是折叠引入的代价，我上一轮把它说成代价是错的。
      钉住的地方：`tests/codegen.rs::a_bootstrap_spec_is_read_as_the_shapes_it_writes`
      （对 `bootstrap.uns` 逐条比对上面四个形状）、
      `tests/generated.rs::a_run_stops_at_the_nearest_end_that_closes_it`
      （编译并运行同一条规则，fixture 加 `syntax RUNDED -> TILDE LEFT IDENT RIGHT;`）。
- [x] 门：全部门表 + `dump` fixture 与生成字节一致。

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

- [ ] **先补规范的洞**：`bootstrap.uns` 里**没有文件级规则**。手写的 `Crate{items: Vec<Item>}`
      与 `Item = S | Stmt | InnerDoc | OuterDoc` 没有被自描述，因此生成的 parser 无从复现它。
      建议加：
      `syntax Item -> WHITESPACE | INNER_LINE_DOC | OUTER_LINE_DOC | INNER_BLOCK_DOC | OUTER_BLOCK_DOC | Stmt;`
      `syntax Crate -> Item*;`
      注意 `Item -> WHITESPACE | …` 里对 trivia 节点的**裸引用**会被 `fold_separators` 保留
      （整条序列只有它自己），所以这一条能工作；生成的 `Crate` 会是 `Vec<Item>` 的 newtype
      而不是命名字段 struct —— 按 P0 的「行为等价」决定，可以接受。
- [x] **第二个规范洞（本轮新发现）：`Stmt` 的关键字** —— 已定：**改手写的一侧**。
      `bootstrap.uns:167` 把三种声明写成**一条**规则
      `syntax Stmt -> LEADING_KEYWORDS IDENTIFIER '->' Expr ';'`，生成的是四元素扁平
      struct；手写的 `Stmt` 原先是按关键字分三变体的 enum，`semantics` 从变体名取
      `NodeKind`。现在的形状：`Stmt<I>` 是 struct，字段
      `keyword: LeadingKey<I>` / `ident` / `definition`，其中
      `LeadingKey`（`syntax/expr.rs:50`）是 `Whitespace | Lexer | Syntax` 三个关键字 token
      的和类型。kind 不从 `LEADING_KEYWORDS` 的**文本**读，而从 `LeadingKey` 的**变体**读
      （`symbol.rs::stmt_kind`），所以「是哪三种声明」仍在结构里而没掉进文本里；这一步
      同时把原来重复了四次的三路 `match` 收成这一个函数。
      为什么 `LeadingKey` 不放 `lexical/keyword.rs`：那个模块全用 `define_keyword!` 配手写
      `Syntax` impl，且写 `impl<I> parserc::syntax::Syntax<I>`，`use parserc::syntax::Syntax`
      会和局部那个名为 `Syntax` 的关键字 struct 撞掉；`syntax/expr.rs` 已经在给同类
      「若干个 token 节点的和类型」derive `Syntax`，放这里才一致。
      与生成侧的剩余差别：生成的是一个统一的 `LEADING_KEYWORDS<I>` token 字段，手写的
      `LeadingKey` 是三选一 —— 这是 P6 等价性测试要处理的一项，和关键字 trivia 的归属
      差别（手写关键字只带后置 trivia，见 `lexical/keyword.rs:9-11`）同一类，本轮不动。
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

> P1–P3 收尾时跑过全表：1/3/4 全绿（179 lib / 3 bootstrap / 18 codegen / 1 dump /
> 24 generated / 3 doctests），2 十五行全 `ok`；3 为 runtime-only（2 + 0）；5 在
> `crates/unsyn` 里 0 条（`parserc` 4 条、`sourcespan` 1 条是既有的，不属本任务）；
> 6 四组 feature 组合各 0 警告；7 干净。

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
