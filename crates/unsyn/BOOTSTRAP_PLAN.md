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
  `Call(Angle<I, Ident<I>>)`、`Paren(Paren<I, Box<Expr<I>>>)`；`Repeat<I>` 自己
  就是 `Brace<I, RepeatGroup<I>>`（含花括号，与产生式左侧同边界）。
  `Brace/Bracket/Paren/Angle` 是 `Delimiter` 的别名（`lexical/delimiter.rs`），
  `RepeatGroup`/`SuffixGroup` 是 `Choice` 的别名 —— 别名不新增节点。
- 13 个节点类型里 12 个用 `#[derive(Syntax)]`，只有 `Crate` 手写 `parse`/`to_span`。

**当前生成物的实测状态**（`Codegen::new().with_externals(XID_Start/XID_Continue)`
作用于 `bootstrap.uns`，102,698 字节 / 61 个类型，0 错误）：

| 计数 | `Delimiter` 0 · `Punctuated` 0 · `Limits*` 0 · `Char<` 0 · `take_till` 0 · `take_until` 0 |
| --- | --- |
| 已有的 | `keyword` 41 · `next_if` 37 · `take_while` 5 · `pub struct` 54 · `pub enum` 7 |
| 结论 | **字符级（lexer）已经用上内置加速函数**；缺陷集中在**节点级（syntax）** |

具体偏差（可直接对照生成文本）：

- `S<I>(pub I)` 一次只吃**一个**字符，靠 `Vec<S<I>>` 拼成一串；
  不是参考实现里的"非空极大串"。
- `Stmt` 为 5 个产生式元素生成了 **9 个字段**（4 个 `::std::vec::Vec<S<I>>`）。
- `ExprWithoutSuffix::ExprWithoutSuffixBracketStart` 把 `'[' ( SetItem ',' )* SetItem? ']'`
  展平成一串字段（`BracketStart, Vec<(Vec<WS>, ExprWithoutSuffix1)>, Vec<WS>,
  Option<SetItem>, Vec<WS>, BracketEnd`），其中 `ExprWithoutSuffix1<I>` =
  `(SetItem<I>, Vec<S<I>>, Comma<I>)` —— 这就是 `Punctuated<SetItem, Comma>`。
- `Repeat` = `(BraceStart, Vec<WS>, Repeat1, Vec<WS>, BraceEnd)`，而手写侧的边界
  现在就是 `Repeat<I>` = `Brace<I, RepeatGroup<I>>`（花括号在节点里，内部三选一
  是匿名的 `Choice`）。剩下两处差异：生成侧把 `Repeat` 包成 newtype struct 而不是
  别名（P0 已定为行为等价，不要求别名相同），以及那个括号组生成侧仍叫 `Repeat1`。
- `syntax Expr -> ExprNoTopAlt+ ('|' ExprNoTopAlt+)?` 里的 `ExprNoTopAlt+`
  被**就地重复内联**（`Expr.0` 与 `Expr1.1` 各一份）。注意：规范里本来就没有
  `ExprNoTopAlts` 规则，它是手写树为复用而引入的；生成物内联成元组即可，
  **不需要**造这个名字（见 P3 决定）。
- `enum Repeat1` 出现 `Repeat1DEC_LITERALVariant3` 这种"同名加序号"的变体名（旧命名下的
  实测；产生式改名后它是 `Repeat1LitDecVariant3`，见 P5 的命名条目）。
  **这一条不是可读性问题**（我原先这么记，是错的）：`Repeat1` 根本不是产生式的左侧，
  它是 `Repeat` 右侧那个匿名括号组 `( ',' LitDec | … )` 被生成器提成了一个具名
  enum —— 按「只为产生式左侧生成 CST 节点」，它本来不该存在，见 P5。相比之下
  `ExprWithSuffixExprWithoutSuffixVariant2/3/4` 不同：`ExprWithSuffix` 是左侧的具名规
  则，它的变体名冲突才是真的命名问题。
- 生成的 punct 类型叫 `Or`/`Comma`/`Semi`/`Minus`/`Tilde`/`Star`，与
  `parserc::syntax::Or` 同名；生成代码里凡是用到代数节点都必须全限定（P5）。
- `{n,m}` 目前是手写 `loop` + `__items.len()` 比较，失败报
  `Kind::Syntax(name, ..)`。**这是有意选择**：`Kind::Limits*` 变体不带名字，
  具名的报错信息更好。保持不变，只在 P4 记录理由。

**`bootstrap.uns` 自身的三个缺陷**（属于"逻辑漏洞/词法错误"豁免范围，P0 处理）：

1. `:3-14` whitespace 集与 `char::is_whitespace()` 不一致。实测：`U+200E`、`U+200F`
   的 `is_whitespace()` 为 **false**（规范却列了它们）；而 `U+00A0`、`U+1680`、
   `U+2000`、`U+200A`、`U+202F`、`U+205F`、`U+3000` 都是 **true**（规范漏了）。
2. `:19` `( XID_Start | _ )` 与 `:76` `( DecDigit | _ )*` 里的 `_` 想表达**下划线
   字符**，但 DSL 把裸 `_` 定义为**通配符**（`WILDCARD = "_"`，"匹配任意一个字符"，
   `semantics/graph.rs:26-31`）。后果已在生成物中测得：
   `IdentOrKeyword` = `XID_Start.or(next_if(|_| true))`（第一个分支变成死代码，
   任意字符都能开头）；`LitDec` = 一个数字后接 `take_while(|_| true)`（一路吃到
   EOF，`{1,2}` 这种写法会被吞掉）。参考实现是 `'_'`：`lexical/ident.rs:28`
   `c == '_' || is_xid_start(c)`，`lexical/lit.rs:282` `is_ascii_digit() || c == '_'`，
   `semantics/value.rs:75` `replace('_', "")`。
3. `except` 在多字符操作数上的语义。现实现（`codegen/pattern.rs:311` 的字符类分支
   是正确的减法；`codegen/render.rs:319` 的多字符分支）是"**在目标起始处探测**被减去
   的语言"。于是 `IdentOrKeyword except Keywords` 会把 `lexerX` 也拒绝掉，
   而参考实现只拒绝**整个词恰好是关键字**的情形（`lexical/ident.rs:33-41` 比较
   `content.as_str()`）。把 `except` 定义为**"目标实际吃掉的那段文本不属于被减语言"**
   （整串差集，而不是前缀探测）即可复现参考行为，同时**取消**了"关键字边界需要写进
   规范"这条待办 —— DSL 里也就无需负向前瞻。

---

## P0 — 规范与决定（先落文字，再动代码）

- [x] **whitespace 集**：`bootstrap.uns:3-17` 改成与 Unicode `White_Space` 一致，用 range
      压缩成集合写法：
      `whitespace S -> [ U+0009-U+000D , U+0020 , U+0085 , U+00A0 , U+1680 , U+2000-U+200A , U+2028 , U+2029 , U+202F , U+205F , U+3000 ];`
      决定：**生成器仍不得引用 `char::is_whitespace`**，它只把规范列出的字符编译成字符类。
      注意 range 只能作为 `SetItem` 出现（`Range` 不在 `ExprWithoutSuffix` 里），所以顶层
      交替写法 `U+0009-U+000D | …` 是不合语法的，必须用集合。
      改后生成物实测：`S` 的字符类 = `'\t'..'\r' | ' ' | \u{85} | \u{a0} | \u{1680} |
      '\u{2000}'..'\u{200a}' | \u{2028} | \u{2029} | \u{202f} | \u{205f} | \u{3000}`，
      与手写 `lexical/s.rs` 的 `is_whitespace()` 恰好相同（25 个码点）。
- [x] **`_` → `'_'`**：修改 `bootstrap.uns` 的 `IdentOrKeyword` 与 `LitDec`。
      裸 `_` 保持"任意一个字符"的含义（`tests/data/spec.rs` 同时用了 `_` 和 `UNDER -> '_'`，
      证明 DSL 本来就区分这两者）。
      改后生成物实测：`IdentOrKeyword` = `XID_Start.or(next_if(c == '_'))` 再接
      `XID_Continue*`；`LitDec` = `DecDigit` 再接
      `take_while(|c| ('0'..='9').contains || c == '_')` —— 与 `lexical/ident.rs:28`、
      `lexical/lit.rs:282` 一致。
- [x] **`except` = 整串差集**：**决定**记录在此；文档注释与实现一起放到 P4 改，避免文档
      先于代码说谎。判定的可实作形式：把目标吃掉的那段文本 `__from.split_to(__taken)`
      作为**独立输入**跑被减语言，能整段吃完才算命中（在起始处探测并只比长度是不健全的，
      因为被减语言可能有长短不同的多个匹配）。
- [x] 决定：`whitespace` 节点**不是一等产生式**，它的 body 只被读成一个字符类，生成的
      类型按 P0 的归属规则是 trivia 本身，因此它自己不拥有 trivia（否则无限递归）。
- [x] 决定：自举的验收标准是**行为等价**（同一批 `.uns` 上手写 parser 与生成 parser 的
      CST 与错误位置一致），不要求类型名/别名/字段数相同（类型名后来已逐一对齐，见 P5 的
      命名条目；别名与字段数仍不作要求）。因此 `Bracket`/`Angle` 这些
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
     `QuoteEscape -> '\\\''` 若拥有 trivia，`'a\ ''` 就成了合法字符串；
     `LeadingKeywords` 被 `Keywords` 引用，因此也不拥有。
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
      `syntax/expr.rs:354` 的 `Tilde(Tilde<I>, Bracket<I, Punctuated<…>>)` 同形。
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
      要求：P0 定的验收标准是行为等价，且明写"不要求类型名/别名/字段数相同"（`:138`）。
      拿字段数当指标还有更坏的一层：它给上面 `Delimiter` 那个真错打了高分 —— 窗口取得越
      贪就越"省字段"，所以这个指标只会把错误方向奖励成成绩，把 `~` 和 `]` 配成一对这种
      形状错误完全看不见。现在核对的四条（`bootstrap.uns:123-126` 对
      `syntax/expr.rs:342-376` 的 `enum ExprWithoutSuffix`）：
      - `'~' '[' … ']'` → `Tilde(Tilde, Delimiter<BracketStart, BracketEnd, Punctuated>)`
        同形于手写的 `Tilde(Tilde<I>, Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`；
      - `'[' … ']'` → 同形于手写的 `Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`；
      - `'<' Ident '>'` → 同形于手写的 `Call(Angle<I, Ident<I>>)`；
      - `'(' Expr ')'` → 同形于手写的 `Paren(Paren<I, Box<Expr<I>>>)`；
      - `Stmt -> LeadingKeywords Ident '->' Expr ';'`（`:167`）→ 生成的是
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

- [x] 多字符 `except` 改为整串差集（P0 的决定）：在被减语言上跑**目标吃掉的那段文本**，
      整段吃完才拒绝；字符类分支保持现在的 `item.and(removed.negated())`。
      两条路径都改了：词法侧 `codegen/scan.rs::Step::Except` 记下 `__start`，用
      `input.start() - __start.start()` 把目标读过的那段 `split_to` 出来交给新增的
      `Emitter::takes_all`（`is_ok() && is_empty()`，**不再** `.ok()` 后只比长度）；
      节点侧 `codegen/render.rs::Assert::Except` 同形（`__from` + `takes_all`）。
      `probe()` 去掉 `before` 参数、改成返回布尔表达式，仍服务于 `followed` 与补集。
- [x] 同时改 `syntax/expr.rs` 的 `Except` 文档注释、`codegen/pattern.rs::Step::Except`、
      `render.rs::asserted` 的注释；连带改了 `codegen/types.rs` 开头四条“需要自己的类型”
      里对 `except` 的说法、`semantics/graph.rs::same_offset_edges` 的文档、
      `runtime/input.rs` 里 `StartWith` 那条要点。`graph.rs` 的分析本身不用改：
      被减语言仍然是在目标起始位置被 consulted 的，变的只是它能看到多少文本。
- [x] **翻过来的既有断言**：`tests/generated.rs::a_name_is_a_word_that_is_not_a_keyword`
      现在 `lett x`/`xlet` 为 `is_ok()`，`let`/`let x`/`fn x` 为 `is_err()`，
      并保留 `KEYWORDS` 自己能吃掉 `lett` 的头部这一条事实断言。
      新增 `what_except_removes_cannot_see_past_the_text_it_is_asked_of`：fixture 里加
      `lexer REACH -> LETTER except 'abcd';`，`"abcd"` 只读走 `a`、留下 `bcd` ——
      被减语言若能看到操作数后面的文本就一定会拒绝，这条就是“隔离”的钉子。
      fixture 随之重生成（`tests/data/mini.rs` 95,837 → 97,948 字节，25 → 26 条实测）。
- [x] 复核 `~[X]*`：**PLAN 原来写的理由是错的**。`crates/parserc/src/c.rs:186-196` 里
      `take_till(cond)` 的定义**就是** `take_while(move |c| !cond(c))`（注释原文
      “Shorthand for [`take_while`] with a negated predicate”），所以两者不是两份实现，
      没有“统一选一个”的问题：无界形仍用 `take_till`（它本来就替我们取反，省一层闭包），
      有界形只有 `take_while_with` 能承载边界，取反就写在我们这边。
      理由记进 `codegen/scan.rs` 的 `# The shortcuts` 小节与 `complement()` 的注释。
- [x] 关键字边界：手写侧新增 `lexical/ident.rs::a_name_that_only_borders_on_a_keyword_is_a_name`
      （`lexer` 拒绝、`lexerX`/`xlexer` 接受），生成侧由 `IDENT -> WORD except KEYWORDS`
      的 `let`/`lett`/`xlet` 三条同形断言钉住 —— 整串差集正是让这两侧一致的改动。
      注：`NonKeywordIdent` 这个名字要等 P8 从 `bootstrap.uns` 生成出来才真正被编译，
      今天对它的验证走的是 fixture 里的同形状规则，而不是它本身。
- [x] 保留 `{n,m}` 报 `Kind::Syntax(具名)` 而不是 `Kind::Limits*`，取舍的理由写进
      `render.rs::bounds` 的文档注释（`parserc` 自带的 `Limits*` 不带名字，报不出是哪条
      规则短了；而这里的计数运行总是一个自己的类型，名字就从那里来）。
- [x] 门：全表重跑见末尾「门」小节。新增的行为断言都是编译后实测（`tests/generated.rs`），
      不是只看生成文本；生成文本本身另有 `render.rs`/`scan.rs` 单测与 `tests/codegen.rs`。
      跑 `--document-private-items` 时又发现 4 条本来就断的内链（`render.rs` 两处
      `[`scan`]`、`codegen/mod.rs` 两处 `[`Step::…`]`），一并补成全路径。

## P5 — 只为产生式左侧生成节点，与命名卫生

**原则**（本轮定下，原 P5 第二条把它当成命名问题，是误判断）：生成的 CST
**节点类型只对应一条产生式的左侧**；右侧那些**本身没有名字的联结结构**（括号组里的
多选与序列）一律**内联**成 `parserc` 的代数类型表达式 —— 元组、`Option<…>`、
`Vec<…>`、`Delimiter<…>`、`Punctuated<…>`，以及下面要补的 `Or<…>` —— 不替它们造名字。

**"内联"不适用于对已声明产生式的引用**（用户 2026-11-16 的质疑成立，措辞在此钉死）：
产生式右侧写成 `Ident`、`Repeat`、`LitDec` 这类**左侧名字的引用**，当然要
生成 CST 节点，而且现在就已经是节点，本条不改这个行为。实测证据（全量 codegen
`bootstrap.uns`，以及 fixture）：

- `SetItem` 四个变体持 `Range<I>` / `LitStr<I>` / `LitUnicode<I>` /
  `Ident<I>`；
- `ExprNoTopAlt` 两个变体持 `Box<ExprWithSuffix<I>>` / `Box<ExprWithoutSuffix<I>>`；
- `Repeat` 持 `Delimiter<BraceStart<I>, BraceEnd<I>, …>`；
- `tests/data/mini.rs:1386` 的 `ITEM` 持 `Delimiter<ARROW<I>, SEMI<I>, EXPR<I>>`，
  `EXPR:1424` 持 `Box<PRIMARY<I>>`，`PRIMARY:1469` 各变体持 `IDENT<I>` / `LITERAL<I>` /
  `Box<GROUP<I>>`。

要去掉名字的只有 `'<' Ident '>'`、`( '*' | '?' | '+' | Repeat )` 这种**匿名括号组
自身**——它们是联结结构而不是引用。手写侧就是这么写的：`bootstrap.uns:145` 的
`Repeat` 是左侧（也是引用点），所以 `syntax/expr.rs:420` 有 `enum Repeat`；而
`bootstrap.uns:124` 的 `'<' Ident '>'` 这个组在手写侧是内联的
`Angle<I, Ident<I>>`，没有 `ExprWithoutSuffix1` 这种名字。

**（下面这两段是动工前的实测，现已完成：那 2 个 + 6 个右侧组 enum 都已内联，
判据已写成 `tests/generated.rs` 与 `tests/bootstrap.rs` 各一条结构断言。）**

实测的偏离（一个临时探针，跑 `bootstrap.uns` 全量 codegen 后统计，已删）：36 条
产生式，58 个具名类型。多出的 22 个里：

- **20 个是字面终结符的 token 类型**（`Or`/`Comma`/`Star`/`ArrowRight`/`Concat` …，
  就是 `'|'`、`','`、`'*'` 这些写在产生式里的符号）。它们要存下 token 和它的
  trivia，必须有类型，与手写侧的 `lexical::punct`/`lexical::keyword` 同职，**不是**
  要清理的对象。
- **2 个才是真正的违规**：`Repeat1`（`Repeat` 右侧那个三选一括号组）和
  `ExprWithSuffix1`（`('*'|'?'|'+'|Repeat)` 那个四选一）—— 右侧中间结构被提成了
  具名 enum。

fixture `tests/data/spec.rs` 上同一违规有六个：`NUMBER1`（`DIGIT ( DIGIT | '_' )*`
里那个组）、`TEXT1`、`Commented1`、`Either1`、`ANYONE1` 与 `ANYONE2`。最后两个是
同一形状的**两份副本**（`(MATCHER | _)` 在 `ANYONE` 里写了两次）—— 提具名类型不
仅多余，还会造重复体，而内联没这个问题。剩下那些多出的名字不要误伤：
`Plus`/`Minus`/`Underscore`/`Any` 是 `'+'`/`'-'`/`'_'`/`_` 的 token 类型，`TEXT11`/
`TEXT12`/`Commented11` 是 `~[…]`/`[…]` 这类字符级模式的 token 类型，`SHAPE1`/
`Dotted1` 是 `{n,m}` 的载体，`Guarded1` 是 `followed` 的 `Asserted`。

- [x] **规范里的名字改成 Rust 类型写法，并与手写侧的类型逐一同名**（用户 2026-11-16：
      “如果需要自举 CST 的名称要和 Rust 侧一样，按照 rust 的命名规范修改”）。
      依据是 `codegen/name.rs` 开头写死的规则：产生式的名字**就是**生成出来的 Rust 类型名。
      只要两侧名字逐一相等，自举出来的 parser 才是手写 parser 的直接替换，否则“名字一样”
      这条要求就只是修辞。36 条产生式重写了 24 条，四条原则：

      | 情况 | 处理 |
      | --- | --- |
      | 手写侧已有同职类型 | 采用手写的名字：`S`、`Ident`、`LitStr`、`LitUnicode`、`LitDec`、`QuoteEscape`、`AsciiEscape`、`UnicodeEscape`、`LineComment`、`BlockComment`、`InnerLineDoc`、`InnerBlockDoc`、`OuterLineDoc`、`OuterBlockDoc`，加上原本就一致的 `Stmt`、`Expr`、`ExprNoTopAlt`、`ExprWithoutSuffix`、`ExprWithSuffix`、`Repeat`、`Range`、`SetItem` |
      | 手写侧折进 `match` 臂或 `is_xxx`，没有对应类型 | 按同一风格取名：`IdentOrKeyword`、`NonKeywordIdent`、`RawIdent`、`Keywords`、`LeadingKeywords`、`FollowingKeywords`、`OctDigit`、`DecDigit`、`HexDigit`、`BlockCommentOrDoc` |
      | `<…>` 外部链接 | 不动：`XID_Start`、`XID_Continue` 用 UAX #31 给两个属性的原名，那是调用方表里的键，不是 unsyn 的类型 |
      | 两个控制字符 | 不动：`LF`、`CR` 是它们的通用写法，手写侧 `AsciiEscape::LF/CR` 也这么拼；两者只当集合成员出现，不成为树上的节点 |

      手写侧跟着改的只有一处：`ASCIIEscape` → `AsciiEscape`（连同 `StrSegment` 与
      `SyntaxKind` 里的同名变体）——它是唯一违反 Rust 类型命名法的名字，其余全部本来就一致。
      一个容易混的点：`whitespace` 声明取的名字是 `S`，即 `lexical::S` 那个非空极大 trivia 串；
      `lexical::keyword::Whitespace` 是 `'whitespace'` 这个**关键字 token**，两者不同职，
      生成物里也分处不同模块，所以不冲突。
      两条测试把这件事钉住：`tests/bootstrap.rs::the_nodes_are_named_as_rust_names_its_types`
      要求每个产生式名都是合法的 Rust 类型名且不是一片大写（两个字母的 `LF`/`CR` 例外）；
      `the_spec_names_the_types_the_crate_already_has` 逐个断言手写树用到的 22 个类型名在规范
      里都有同名产生式，任何一侧单独改名都会红。
      `tests/data/spec.rs` 那套示例语言的 SCREAMING 写法**没动**：它是 fixture 的输入，改了要
      连带重生 `tests/data/mini.rs`，归下面那条重生 fixture 一起做。
- [x] `FieldType` 缺一个"多选一"分支：它有 `Group`（元组）、`Concat`、`Opt`、`Many`、
      `Punctuated`、`Delimited`，都是内联的类型表达式，唯独没有 alternative，所以
      多选只能落到 `enum_of` + `anonymous_as` 造一个具名 enum。补一个内联分支（
      `FieldType::Alts(Vec<Vec<FieldType>>)` 或二叉 `Alt`），`render.rs::ty` 把它展成右
      结合的 `::parserc::syntax::Or<A, B>`（`crates/parserc/src/syntax.rs:400`，现成的，
      不需要生成器声明任何东西），`read`/`to_span` 对应 `Or::First`/`Or::Second`。
      **已做**：`FieldType::Alts(Vec<FieldType>)`（`codegen/types.rs:187`），文档写明它
      是"右侧的组，不留名字"；`render.rs::ty` 走新的 `Renderer::choice`，展成右结合的
      全限定 `::parserc::syntax::Or<first, rest>`（全限定是必需的：生成物里有
      `Or<I>` 这个存 `'|'` 的 punct 类型，不限定就互相遮蔽）。`takes_nothing` 取
      任一候选为空即空；trivia 由 `Trivia::ty` 逐个候选各自结算，因为"哪条路被读
      出来"决定中间那个串归谁。
- [x] `codegen/types.rs::group()` 在 `expr.rest` 非空时不再走 `next(owner)` +
      `anonymous_as(…)`，而是返回上一条那个内联类型。同理，超过 `MAX_GROUP` 的长
      序列也不该提名字，而是**嵌套元组**（元组没有 arity 上限问题，只要拆成树）。
      **已做**：`group()` 单候选直接返回 `alternative()`，多候选返回 `Alts`；
      `anonymous_as` 与 `MAX_GROUP` 都已删除；`Renderer::tuple` 在 `len > MAX_TUPLE`
      时从 `div_ceil(2)` 处拆成嵌套元组。`types.rs` 模块文档里那条"超过十六个操作数
      要具名类型"的旧理由同步改掉，现在写的是"三样东西才需要具名类型"：`{n,m}` 的
      `Bounds`、`except`/`followed` 的 `Asserted`、以及字符级模式在 syntax 规则里
      需要的 token 类型（`TypeBuilder::token`）。
- [x] 保留那些**不得不**具名的例外，它们不是右侧中间节点，而是 `parserc` 代数里
      没有对应节点、必须自己写 `Syntax` impl 的东西：终结符 token 类型（`token()` /
      `Step` 字符类）与 `Asserted`（`except`/`followed` 的 lookahead）。`{n,m}` 现在
      也造具名类型，那是 P4 记录的取舍（报错带名字）而非结构需要：`Limits*<T, N>`
      是内联代数类型，按原则应该内联；要不要为了错误信息保留具名类型，归 P4 判。
      一个有利于内联的事实：`|` 是 DSL 层的元符号，**不是**目标语言里的 token，
      所以一个多选组周围不需要、也不应有一个存 `'|'` 或存 gap 的字段；组内部的
      trivia 属于每条候选自己的序列，而 `Or<F, S>` 本身不留位置是对的。它用的是
      `F::into_parser().ok()`（`parserc/src/syntax.rs:414`），不回落 `Fatal` ——
      与 `#[derive(Syntax)]` 给 enum 生成的完全同一个形状，所以行为不会变。
      **已定**：例外集合就是上面那三样，已写进 `codegen/types.rs` 的模块文档；
      `{n,m}` 保留具名载体（P4 的错误名取舍）。
      **判据的更正**：计划原本想用"出现以产生式名为前缀的 `…1`/`…2` 就是回归"当
      结构门，这个模式不成立 —— token 类型与 `Bounds`/`Asserted` 载体合法地用同样
      后缀（`TEXT1`、`SHAPE1`、`Guarded1`）。真正的判据是：**每个具名 `enum` 的名字
      必须是一条产生式的左侧名字**，右侧组永远不该有 enum。两条测试据此实现。
- [x] 上面两条做完后，“变体名冲突”只剩**左侧规则**这一种情况，才轮到命名卫生：
      `syntax ExprWithSuffix -> ExprWithoutSuffix (…) | ExprWithoutSuffix 'concat' … | …`
      四条都同一个开头，`named_variant` 只看 `seq.first` 就得到四个
      `ExprWithoutSuffix`，于是 `Variant{2,3,4}` 后缀。改成从"该变体独有的形状特征"
      派生稳定名字（这里区分四条的是第二个操作符 `concat`/`followed`/`except` 和
      那个后缀组），而不是加序号。手写侧改成 4 变体不缩小这一条：它从来只是命名
      问题，不是形状问题，而且上面那条决定定下来之后，“右侧组参与不参与外层扁平
      化”已经不会再被重新推翻。
      **已做**：`named_variant` 删除，`enum_of` 先把全部候选的形状读成
      `operand_names` 列表，再交给 `variant_name(&shapes, index)` —— 取第一个“在这个
      位置上没有兄弟也有”的操作数名字（`operand_name` 不失败：解不出的码点写作
      `Unicode`）；只有两个变体派生出同一个名字时才加 `{name}Variant{n}`。
      `bootstrap.uns:132-143` 四条因此得到 `ExprWithSuffixGroup` / `…Concat` /
      `…Followed` / `…Except`，不再有序号后缀；`tests/codegen.rs` 里`the_four_quantity_operators…`
      与 `a_bootstrap_spec_is_read_as_the_shapes_it_writes` 两处钉住这件事（后者原先
      用 `starts_with("ExprWithSuffix(")` 断言，那是空断言：`enum_variants` 会把括号
      本身吃据，已改成把变体名截到字母数字处再比）。
- [x] **形状后果已定（用户定夺：改手写侧）**：`('*'|'?'|'+'|Repeat)` 不参与外层
      扁平化。手写的 `syntax/expr.rs` 的 `ExprWithSuffix` 从 **7 个变体** 改成
      **4 个**，与 `bootstrap.uns:132-143` 的四条一一对应：`Grouped` / `Concat` /
      `Followed` / `Except`。那个括号组存成它本来的样子：
      `SuffixGroup<I> = Choice<Star<I>, Choice<Question<I>, Choice<Plus<I>,
      Repeat<I>>>`（即 `parserc::syntax::Or` 的右嵌套；只是个类型别名，
      不新增节点）。消费方用 `SuffixOp::of(&group)` 把算子读回来 —— 那是**视图不是
      节点**，树上不存它。

      改动落在 `syntax/expr.rs`（枚举本体 + 两处自己构树的测试）、`semantics/graph.rs`
      （6 处，其中 4 处因为四个算子合成一条反而变短了）、`semantics/check.rs`、
      `codegen/pattern.rs`、`codegen/types.rs`。

      生成物**逐字节不变**：`types.rs::with_suffix` 按算子分支，`SuffixOp::of` 把四个
      算子重新读出来，所以 `A*` 仍生成 `Vec<A<I>>`，fixture 漂移断言仍绿。新增两条
      断言钉住这个决定：`the_four_quantity_operators_are_one_alternative_but_four_names`
      （`syntax S -> A* | A? | A+ | A{2};` 仍得到 `SStar`/`SQuestion`/`SPlus`/`SRepeat`，
      合并变体不能把算子的名字塌掉）与 `a_bootstrap_spec_is_read_as_the_shapes_it_writes`
      里的 `enum_variants` 计数（生成的 `ExprWithSuffix` 恰好 4 个变体，且不出现以
      `Question`/`Plus`/`Repeat` 打头的变体）。
- [x] **`Repeat` 的节点边界已改成含花括号**（同一类错的第二个例子，用户定夺同样
      是改手写侧）：`bootstrap.uns:145-153` 的左侧是整个 `{…}`，所以手写的
      `enum Repeat { RangeTo, RangeFrom, Count }` —— 它把产式的**名字**贴到了
      右侧那个三选一**括号组**上 —— 被拆成：
      `pub type Repeat<I> = Brace<I, RepeatGroup<I>>`（节点 = 整对花括号）加
      `pub type RepeatGroup<I> = Choice<(Comma<I>, LitDec<I>),
      Choice<(LitDec<I>, Comma<I>, Option<LitDec<I>>), LitDec<I>>>`（匿名组，不造
      名字），外加视图 `RepeatBound<'a, I>`（`RangeTo`/`RangeFrom`/`Count`，沿用
      原有词汇）。`validate_repeat`/`repeat_lower`/`repeat_bounds` 现在都拿整
      个 `&Repeat<I>`，调用点从 `repeat_lower(&brace.body)` 成了 `repeat_lower(repeat)`。

      **一个行为改变**：`SemanticsKind::InvalidRepeat` 的 span 以前只盖住花括号
      里的内容，现在盖住整个 `{…}`——跟产生式左侧一致，也跟 `pattern.rs`/
      `types.rs` 那两处 `CodegenKind::InvalidRepeat`（报的是整个带后缀的操作数）
      一致。没有测试钉在旧 span 上。

      生成物仍逐字节不变（fixture 漂移断言绿）。`tests/codegen.rs` 里那句注释
      `'{' Expr1+ '}'` 本来就是错的，已改成真实产生式并说明手写侧已同边界；
      `lexical/delimiter.rs` 的 `the_span_of_a_group_reaches_its_closing_token` 现在直接把
      `"{2, 3}"` 当 `Repeat<_>` 解析并断言 span 到 `}`，这就是这个边界的测试。

      剩下两处差异归 P6 等价清单：生成侧把 `Repeat` 包成 newtype struct（手写是
      别名）、生成侧那个括号组仍叫 `Repeat1`（P5 第一条处里）。
- [x] ~~**一个需要定夺的形状后果**~~ —— 已定并已改，见上面那条：取 **4 个变体**，
      改的是手写侧（`Star`/`Question`/`Plus`/`Repeat` 四个变体合成了 `Grouped` 一个）。
      剩下的只是把生成侧的 `ExprWithSuffix1` 换成内联的 `Or<…>`，归本段第一条。
- [x] 生成器内部引用代数节点一律全限定（`::parserc::syntax::Or`、`Delimiter`、
      `Punctuated`、`Limits*`），避免与生成的 punct 类型 `Or`/`Tilde`/`Comma` 冲突。
      第一条把 `Or` 变成天天用的内联节点后，这一条从“应修”变成“必须”：生成的
      `Or<I>`（`'|'` 那个 punct）与 `parserc::syntax::Or` 同名，不限定就互相遮蔽。
      **已做**：`choice`/`tuple`/`read`/`to_span` 写的都是 `::parserc::syntax::Or<…>`，
      `Renderer::choice` 的注释说明了为何必须限定的理由。
- [x] 重生 `tests/data/mini.rs`：这一条改动会删掉上面那六个右侧组 enum（以及
      bootstrap 侧的 `Repeat1`/`ExprWithSuffix1`），fixture 与 `tests/generated.rs`
      的形状断言跟着改。
      **已做**：fixture 97,948 → **86,801** 字节（-11,147），`NUMBER1`/`TEXT1`/`Commented1`/
      `Either1`/`ANYONE1`/`ANYONE2` 六个右侧组 enum 没了，留下的 `TEXT1`/`TEXT2`/`Commented1`
      是字符级模式的 token 类型（重新编了号），`SHAPE1`/`Guarded1`/`Dotted1` 是
      `{n,m}` 与 `followed` 的载体。`tests/generated.rs` 四处断言改成内联形状：
      `Or<MATCHER, Any>`（`ANYONE`)、`Or<IDENT, DIGIT>`（`Either`)、
      `Or<DIGIT, Underscore>`（`NUMBER`)、`Or<TEXT1, (BSLASH, TEXT2)>`（`TEXT`)；其中
      一个测试名从 `a_group_inside_a_rule_is_a_type_named_for_the_rule_around_it`
      改成 `…_is_a_field_of_it_and_not_a_type_of_its_own`，因为它断言的东西变了。
      `tests/dump.rs` 与 `tests/generated.rs` 里“groups that need a type of their own”
      那句旧描述也跟着改了。
- [x] 门：`cargo test -p unsyn --all-features` + `RUSTDOCFLAGS=-D warnings cargo doc`。
      再加一条结构断言：**生成的具名类型集合，必须等于产生式左侧名字集合 加
      上面那些不得不具名的例外集合**；出现任何以产生式名为前缀的右侧组类型
      （`…1`、`…2`）就是回归。这条断言比任何名字派生规则都更难钻。
      **已做**（判据按上面那条更正重写）：两条测试用 `syn` 读生成物里真正的
      `enum` 声明（不是文本搜索），逐个比规范声明的产生式：
      `tests/generated.rs::every_sum_type_is_a_production_of_the_spec` 断言 fixture
      只有 `PRIMARY`/`BINOP`/`LITERAL`；`tests/bootstrap.rs::
      every_sum_type_of_the_generated_parser_is_a_production` 断言 `bootstrap.uns`
      只有 `ExprNoTopAlt`/`ExprWithoutSuffix`/`ExprWithSuffix`/`Range`/`SetItem`
      —— 恰好是那五条多备选的产生式，注释规则与转义都不在列。
      门表数字见末尾「门」小节 P5 那一段。

## V — visit 层（P6 的前置：让一棵树能被另一棵树读）

自举后的 CST 由 `codegen` 生成，Rust 类型与手写的 `src/lexical`/`src/syntax`
不同名也不同构，所以 `src/semantics/` 与 `src/codegen/` 里那些直接 `match`
具体节点类型的遍历**没法复用**。两棵类型不同的树没有 `PartialEq`，能共用的只有
**接口**：一棵树是被怎样走过去的。

第一版把接口设计成"节点自己拆成与类型无关的形状"（`dyn` 句柄 + 一套 vocabulary），
被否掉：绕一大圈 `dyn`，换来的仍然只是"同一个 pass 读两棵树"，而这件事 `syn` 早
就用更直白的办法做到了 —— **每个节点类型一个方法**，默认体一行，往下走。所以现在
的 `src/visit/` 照 `syn::visit` 的形状写：

- `mod.rs`：`pub trait Visit<'ast, I: UnsynInput>`，**51 个方法** —— 50 个节点类型
  各一个（`visit_expr`、`visit_lit_str`、17 个 punct、6 个 keyword…），签名一律是
  `&'ast X<I>`，没有 `dyn`、没有擦除句柄、没有中间词汇表；外加一个必需的叶子
  `fn visit_input(&mut self, input: &'ast I)`。除叶子外每个默认体都只有一行
  `walk::walk_x(self, node)`。
- `walk.rs`：`pub fn walk_x<'ast, I, V: Visit<'ast, I> + ?Sized>(v, node)`，一个方法
  配一个，把该节点持有的东西**按它们在文件里的先后**报出去。

三条约定是这个接口"能被生成"的原因，都写在 `mod.rs` 的模块注释里：

1. **默认体只许是那一行调用。**"这个节点装了什么、按什么顺序"因此只写在 `walk_x`
   一处；能命名一个类型，就能同时发出接口的两半。
2. **顺序是文件里的顺序，不是字段顺序。**`parserc::syntax::Delimiter` 的三个字段是
   `start, end, body`，而 `walk_stmt` 走的是 `->`、body、`;`。
3. **叶子不许跳过任何 token。**punct 吃掉它两侧的空白、keyword 只吃后侧，所以走完
   一个文件，把所有 `visit_input` 的文本按到达顺序拼起来**就等于这个文件**。

- [x] **接口与树一致，而且只与树一致**（`tests/visit.rs`，5 条）：
  - `a_walk_of_a_file_reads_every_token_once_and_in_order`：走一遍"每种写法都摆了一句"
    的 `uns` 文件，断言拼接等于原文，且每个 token 的 `Span` 与前一个首尾相接、最后
    停在 EOF。这一条同时钉住"没跳过"和"没重复"。
  - `every_method_of_the_interface_is_reached_by_a_file_of_the_language`：`Spy` 覆盖
    全部 51 个方法，每个先记名字再往下走。断言**没有走不到的方法**（走不到的方法是
    生成树必须凭空编一个答案的方法），并且拼接仍是原文 —— 走到了不等于走下去。
  - `each_method_of_the_interface_hands_its_node_to_the_walk_named_for_it`：用 `syn`
    读 `src/visit/mod.rs` 与 `src/visit/walk.rs` 的**源码**，断言方法 ↔ `walk_` 一一
    对应、同后缀、默认体只有那一行、且没有第三种成员。
  - `the_interface_names_exactly_the_nodes_of_the_tree`：同一趟扫描 `src/lexical/*` 与
    `src/syntax/*`：凡**只对输入泛型**的 `pub struct`/`pub enum` 就是节点，
    `define_punct!`/`define_keyword!` 的每一次调用也是。两处判定是显式的、写了理由的：
    `BlockComment`/`LineComment` 排除（注释不进树，它只作为盖住它的那段 `S` 存在），
    `pub type` 别名里只有**被一对定界符包住**的算节点（`Repeat`），`Choice` 那一类不算
    （`SuffixGroup`/`RepeatGroup` —— 走进去的是选中的那个算子，不是选择本身）。断言这
    50 个类型名与 50 个方法名**双向相等**。这一条是"接口 = 树，别的都不是"的机器版本：
    新加节点类型却不给方法，或留着一个类型已不存在的方法，都会在这里失败。
  - `every_node_the_spec_defines_has_a_method`：拿 `bootstrap.uns` 自己解析出来的树，
    要求每条 `syntax` 产生式的左侧都有一个 `visit_<snake>`。这一条是**单向**的，理由
    写在它上面的注释里：规范没有文件级规则，`Crate/Item/InnerDoc/OuterDoc` 不在任何
    产生式左侧，所以规范只能说明"少了方法"，说明不了"多了方法"。这正是 P6 那个洞。
  - 命名规则（类型名 → `visit_` + snake_case）眼下只在测试里有一份：`codegen` 目前
    没有任何地方需要 snake_case（它写出的名字都是树已有的名字，见 `codegen/name.rs`），
    所以没有可共用的出口。P6 生成 `Visit` 时必须接到同一份规则上，届时删掉这份本地
    实现。
  - 接口里有两个成员**规范给不出名字**：`LeadingKey`（规范把三种声明关键字写成一条
    `lexer LeadingKeywords`）与 `Repeat`（规范左侧是 `'{' … '}'` 整段）。它们不是漏网，
    是 P6 的活，所以这里不悄悄改名。
- [x] **V2 把 `semantics/check.rs`、`value.rs`、`graph.rs` 改成 `impl Visit`**：一个
      pass 只写它关心的那几个方法，其余靠默认体往下走；
      `validate_expr(expr, report)` 变成 `Checker { report }.visit_expr(expr)` 那个形状。
      三个文件现有的测试模块**逐字未动**，它们就是等价证明（迁移前后都是 182 lib 全绿，
      其中 `semantics::check::tests` 13 条、`semantics::value::tests` 6 条、
      `semantics::graph::tests` 7 条）。
  - `check.rs`：6 个互相调用的 `validate_*`（`validate_alts`/`validate_alt`/
    `validate_with_suffix`/`validate_without_suffix` 加入口）塌成 `Checker` 一个
    `impl Visit` 的 2 个方法（`visit_expr_without_suffix`、`visit_repeat`）+ 一个空的
    `visit_input`。集合、字面量在 `visit_expr_without_suffix` 里判，重复次数在
    `visit_repeat` 里判，其余靠 `walk::walk_*` 接力。旧链路只经
    `SuffixOp::of(suffix) == Repeat` 到达 `Repeat`，新链路只经 `walk_suffix_group` 到达
    `visit_repeat`，而 `SuffixGroup` 四个分支里只有 `Repeat` 是节点，所以每个 `Repeat`
    仍恰好报一次（`invalid_repetition_bounds_are_reported`、
    `the_lower_bound_of_a_repetition_is_evaluated` 两条钉住）。诊断的文字与先后都没变，
    因为 walk 的顺序就是文件里的顺序，而旧链路也是按 `target` 再 `suffix`/`tokens` 走。
  - `graph.rs`：三族公共遍历 `walk_refs`/`walk_calls`/`walk_sequences` 各塌成一个
    `impl Visit`（`Refs`/`Calls`/`Sequences`，各一个方法 + 空的 `visit_input`），
    945 → 841 行，删掉 15 个纯接力函数。三个 pass 的公共签名（`walk_refs(&Expr, &mut F)`）
    与调用方一字未改。`Refs` 是三个里唯一必须自己分派变体的：裸名字与集合成员是依赖，
    而 `<call>` 里的名字不是（它是调用方在 build 时给的外部链接），所以它在
    `visit_expr_without_suffix` 里判形状，而不是覆盖 `visit_ident` —— 后者会把调用目标
    也算成引用。`Calls`/`Sequences` 各只需一个方法，是因为它们要找的东西（一个 `Call`、
    一条 `ExprNoTopAlts`）在树里只有一种位置。
  - **`value.rs` 只搬了该搬的那一个**：`str_value` 改成 `Decoder`（4 个方法，
    `#[derive(Default)]`，无泛型字段），`StrSegment` 的 4 路 `match` 交给默认的
    `visit_str_segment` 去分派，三个 escape 方法**不再往下走**，于是 `visit_input` 只会
    收到「没有转义的普通文本」。`unicode_char`/`dec_usize` 这类不迁：它们读的是**单个
    token 的文本**，不是树上的下行，`Visit` 对它们是零收益。
  - **`graph.rs` 的 `_nullable`/`_first`/`_same_offset` 三族不迁**，这是本轮对原计划的一处
    收窄。它们不是 walk，是**带回程值的折叠**：`with_suffix_nullable` 要 `bool`，
    `with_suffix_first` 要在第一个必须非空的操作数处停，`with_suffix_same_offset` 要把
    `table`/`nullable`/`&mut names` 三份上下文一路传下去。默认体只能往下走、不能往回算，
    硬套 `Visit` 只会把每个 `bool` 变成 `&mut self` 上的一个累加器加一处"要不要继续"
    的旗标，读起来比现在差，而且它们 `match` 的 `ExprWithSuffix`/`ExprWithoutSuffix`
    都是规范里**有名字的左侧**，生成树会给出同名同构的类型，所以留着它们并不挡住 P6。
    真正会挡 P6 的只有那种自己手搓下行链路的代码，本轮已经把这种代码清光了。
- [x] **V3 让 `codegen` 与 `semantics` 都不再命名节点的形状**：三个被产生式左侧
      命名的类型 —— `ExprWithoutSuffix`、`ExprWithSuffix`、`SetItem` —— 的变体名，
      除了 `src/syntax/`（它们就写在那儿）和 `src/visit/walk.rs`（它要把一个节点拆成
      token 和 punct，只能问形状），全 crate 不再有任何一处 `match` 或路径写它们。
      做法不是把折叠改写成 visitor，而是**沿用 `SuffixOp::of`/`RepeatBound::of` 已经在
      用的视图枚举**：每种形状在哪一个地方回答一次"这是什么形状"，下游就不再有意见。
      `syntax/expr.rs`（现 **974** 行，比上一个提交净增 185 行）新增三个视图：
      - `Operand`（7 个：`Set/Complement/Group/External/Str/Unicode/Reference`）——
        一个操作数在序列里的七种可能，`~[…]` 与 `[…]` 因此第一次有了两个**不同**的
        名字而不是同一对括号上的两次 `match`；`reference()` 是给"只有名字才是依赖"的
        那些 pass 的出口，`graph.rs` 里手搓的 `ExprWithSuffix::Concat/Followed` 两处
        `match` 由此删掉（945 → 841 → **820** 行，`check.rs` 620 → 649 → **650**）。
      - `Suffixed`（4 个：`Repeated/Concat/Followed/Except`）—— `ExprWithSuffix` 的四
        种形状；`Grouped` 这个**手写的变体名**（它写的是 `SuffixGroup` 那棵 `Choice`
        嵌套，规范左侧并不叫它）只活在这一处翻译里，下游一律问 `Repeated{target, op}`。
      - `SetMember`（4 个：`Range/Str/Unicode/Reference`）—— `SetItem` 的四种形状，
        `semantics/check.rs` 里那处 `matches!` 现在问它。名字不叫 `Member`：
        `check.rs` 自己有一个私有 `struct Member`，那是"一个成员 denote 了哪些值"，
        与形状无关。
      改的下游：`codegen/pattern.rs`（`expr`/`alts`/`with_suffix`/`operand`/`set_item`）、
      `codegen/types.rs`（`with_suffix`/`leaf`/`operand_name`/`kind`/`group`/`enum_of`），
      并且都改用 `Expr::branches()` / `ExprNoTopAlts::elements()` / `Expr::is_single()`
      这几个 `syntax` 自己的读法，而不是 `match e { Expr { first, rest } }` 去拆字段。
      `types.rs::operand_name` **发出的名字一个都没变**（`Tilde/Set/Group/Any/Concat/
      Followed/Except/Star/Question/Plus/Repeat/Unicode`），它们烙在生成类型名和
      `tests/data/mini.rs` 里。
  - 门禁：`tests/shape.rs`（**3 条**，`required-features = ["codegen"]`）。它用
    `proc-macro2` 的词法扫 `src/` 下除 `syntax/`、`visit/` 外的每一个 `.rs`（实测
    ≥10 个文件），断言没有 `Shape :: 大写名字`。走词法不走 `syn` 的 AST，是因为要找的
    东西就是一对 token，而 AST 走查既漏 `matches!` 里的模式（那不是表达式）、又要为
    每个表达式位置写一遍 Visit。两条自证：`the_scan_is_not_blind` 钉住它对 inline 片段
    恰好报出三处（并放过 `SetItem::parse`、`Option<ExprWithSuffix<I>>`、
    `<ExprWithoutSuffix<I> as Default>::default()` 这三处合法用法）；
    `the_two_excused_directories_do_indeed_name_the_shapes` 钉住两个豁免目录**确实**在
    命名形状（`syntax/expr.rs` 24 处、`visit/walk.rs` 15 处），否则豁免就是白给的。
    把违规临时写进 `src/` 一个新文件，主测试确实失败（已验后删除）。
- [ ] **V4 `VisitMut`/`Fold`**：这里没有要改写树的 pass，所以先不做；它们与 `Visit`
      共用同一份"节点清单"，而那份清单现在由 `tests/visit.rs` 从源码里读出来。
      V3 之后这条更没必要了：三个视图枚举本身就是"与类型无关的形状"，一个要改写树的
      pass 可以照着它们写。

## P6 — 自举验收（本计划的终点）

- [x] **先补规范的洞：文件级规则** —— 已加。`bootstrap.uns` 现在自描述整个文件，从 36 条
      规则涨到 **40** 条（175 行），末尾追加的四条：
      ```
      syntax Stmt -> LeadingKeywords Ident '->' Expr ';';   -- 原有的最后一条
      syntax Item -> OuterDoc | S | Stmt;
      syntax OuterDoc -> OuterBlockDoc | OuterLineDoc;
      syntax InnerDoc -> InnerBlockDoc | InnerLineDoc;
      syntax Crate -> InnerDoc* Item*;
      ```
      形状与手写的 `syntax/item.rs` 逐一对应（`Item` 三个变体、`OuterDoc`/`InnerDoc` 各两个、
      `Crate` 两个列表），而且**照手写的样子写，不照本计划原先的建议写**：原建议把四个
      doc 规则直接算进 `Item`（`S | InnerLineDoc | OuterLineDoc | …`），那会多出两个
      手写树里根本没有的节点类型，而且把“文件级文档”与“属于后面那条语句的外部文档”
      两种归属弄成一样。名字全部沿用 Rust 侧已有的 `Crate`/`Item`/`InnerDoc`/`OuterDoc`，
      所以 `tests/visit.rs` 的节点清单（50 个）与 `visit_crate`/`visit_item`/…本来就在，
      一行也没动。
  - 与规范一起改的两处测试断言（`tests/bootstrap.rs`）：生成的 sum 类型从 5 个涨到
      **8** 个（新 `Item`/`OuterDoc`/`InnerDoc`；`Crate` 是两条重复，所以是 struct），
      `the_spec_names_the_types_the_crate_already_has` 的名字清单补上这四个。两条都是
      “规范声明了什么”的清单，规范涨了它们就得跟着涨，不是回归。
  - 语义分析对新规则**没有报模糊**：`analyze` 过、lint 空（`bootstrap_semantics` 钉住）。
  - **新发现的第三个洞，本轮没堵**：重复一个**节点**时的 trivia 归属。生成器对
      `Crate -> InnerDoc* Item*` 产出的是
      `Crate(Vec<(Option<S>, InnerDoc)>, Option<S>, Vec<(Option<S>, Item)>)`，而手写的是
      `Crate { inner_docs: Vec<InnerDoc>, items: Vec<Item> }` 加 `InnerDoc`/`OuterDoc` 自己
      带一个前置 `Option<S>` —— 同一个空白串在两边**归属不同的位置**，而生成侧的
      `Item::S` 变体因为前面的空隙先吃光 trivia 而几乎不可能被选中。
      在 DSL 里写成 `S? InnerBlockDoc` 试过，反而更差：每个变体拿到**两个** `Option<S>`
      （一个是写的 `S?`，一个是 `S?` 与终结符之间按“宁可多不可少”规则补的空隙）。
      也就是说 P1–P3 那四条归属规则只判**终结符**，判不了“一个节点的全部分支是否都
      已经吃光前导空白”（`eats_leading` 对 `Named` 只查终结符归属表，不下钻）。要堵
      它得先给 `eats_leading` 加上对节点的递归定义，那是 P6 等价测试真正会碰到的第一件事。
  - 同样不可表达的：`Crate::parse` 手写的“读到文件末尾，否则 `SyntaxKind::Eof`”那一条
      （`syntax/item.rs:93-121`）。DSL 里没有任何“必须吃光输入”的算子，`parserc` 也只
      有 `Input::is_empty`；本轮只记录，不在规范里造伪表达。
- [ ] **P6.1 — 节点级 `eats_leading`**（上面那个洞的堵法）：把 `eats_leading`/`eats_trailing`
      从“只查终结符归属表”扩展成对 `Named` 递归（enum 的全部分支都吃 → 吃；struct 看
      首字段），于是 `Item*` 不再需要空隙，`Item::S` 也重新可达。同时要么给 `Crate`
      一类“必须读到末尾”的语义找一个 DSL 写法，要么把它定为生成器的入口约定（由
      调用方在 `parse` 之后检查 `is_empty`），两条路都得先定后做。
- [x] **第二个规范洞（本轮新发现）：`Stmt` 的关键字** —— 已定：**改手写的一侧**。
      `bootstrap.uns:167` 把三种声明写成**一条**规则
      `syntax Stmt -> LeadingKeywords Ident '->' Expr ';'`，生成的是四元素扁平
      struct；手写的 `Stmt` 原先是按关键字分三变体的 enum，`semantics` 从变体名取
      `NodeKind`。现在的形状：`Stmt<I>` 是 struct，字段
      `keyword: LeadingKey<I>` / `ident` / `definition`，其中
      `LeadingKey`（`syntax/expr.rs:50`）是 `Whitespace | Lexer | Syntax` 三个关键字 token
      的和类型。kind 不从 `LeadingKeywords` 的**文本**读，而从 `LeadingKey` 的**变体**读
      （`symbol.rs::stmt_kind`），所以「是哪三种声明」仍在结构里而没掉进文本里；这一步
      同时把原来重复了四次的三路 `match` 收成这一个函数。
      为什么 `LeadingKey` 不放 `lexical/keyword.rs`：那个模块全用 `define_keyword!` 配手写
      `Syntax` impl，且写 `impl<I> parserc::syntax::Syntax<I>`，`use parserc::syntax::Syntax`
      会和局部那个名为 `Syntax` 的关键字 struct 撞掉；`syntax/expr.rs` 已经在给同类
      「若干个 token 节点的和类型」derive `Syntax`，放这里才一致。
      与生成侧的剩余差别：生成的是一个统一的 `LeadingKeywords<I>` token 字段，手写的
      `LeadingKey` 是三选一 —— 这是 P6 等价性测试要处理的一项，和关键字 trivia 的归属
      差别（手写关键字只带后置 trivia，见 `lexical/keyword.rs:9-11`）同一类，本轮不动。
- [ ] 搭一个 build 阶段：用 `codegen` 从 `bootstrap.uns` 生成 parser，编译进测试
      crate（`runtime`/`codegen` 两个 feature 已经分开，build 依赖放
      `[build-dependencies]`）。
- [ ] 等价性测试：对 `bootstrap.uns` 与 `crates/unsyn/**/*.uns`、
      `tests/data/*.uns`（含故意写坏的样本）**同时**跑手写 parser 与生成 parser，
      断言 CST 与错误位置一致；不一致就回到 P1–P5 修生成侧或改规范。
      **已知的名字差（不要静默改名，待定夺哪侧）**：生成侧
      `ExprWithSuffix::ExprWithSuffixGroup` 对 手写侧 `ExprWithSuffix::Grouped`。
      前者说的是"区分它的那个操作数是个括号组"，后者说的是"它带一个数量算子"；
      两者指的是同一条候选，但描述的不是同一件事。上一轮说过要"改手写侧"，
      但改名会动到 span 偏移相关的断言，所以先记在这里，到等价性测试跑通后再一并定。
- [ ] 全部等价后，才替换 `src/lexical/` + `src/syntax/` 的手写实现。
- [ ] 门（替换后）：`cargo test --workspace`、`--no-default-features --features runtime`、
      `--features codegen`、三组 clippy、四组 rustdoc、`cargo fmt --all --check`。

## 门（每个阶段结束都跑一遍）

| # | 命令 | 期望 |
| --- | --- | --- |
| 1 | `cargo test -p unsyn --all-features` | 全绿（lib / bootstrap / codegen / dump / generated / visit / shape / doctests） |
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
>
> 命名对齐那一轮（见 P5）后重跑全表：1 全绿（179 lib / **5** bootstrap / 19 codegen /
> 1 dump / 25 generated / 3 doctests —— bootstrap 从 3 涨到 5 是新增的两条命名测试），
> 2 十五行全 `ok`；3 runtime-only 2 绿，codegen-only（`--no-default-features`）177 lib
> 全绿；4 与 1 同数；5 三组均在 `crates/unsyn` 里 0 条；6 四组均 0 警告；7 干净。
> 生成物里的类型名随规范一起改了（`structWHITESPACE` → `structS<I>` 等，
> `tests/codegen.rs` 的断言跟着改），但 fixture 是从 `tests/data/spec.rs` 生成的，
> 不受影响（drift 测试绿）。
>
> P4 收尾时再跑全表：1 全绿（**180** lib / 5 bootstrap / 19 codegen / 1 dump /
> **26** generated / 3 doctests），2 十五行全 `ok`；3 runtime-only 2 绿；4 与 1 同数；
> 5 三组均在 `crates/unsyn` 里 0 条（仍只有既有的 `parserc` 4 条、`sourcespan` 1 条）；
> 6 四组均 0 警告，且额外加了 `--all-features --document-private-items` 一组 ——
> 它揭出 4 条早就断的内链，已补成全路径；7 干净。`tests/data/mini.rs` 95,837 →
> 97,948 字节（新增 `REACH` 一条规则，`IDENT`/`Asked` 两处的 `except` 改写）。
>
> P5 收尾时再跑全表：1 全绿（**182** lib / **6** bootstrap / 19 codegen / 1 dump /
> **27** generated / 3 doctests），2 十五行全 `ok`；3 runtime-only 2 绿；4
> `--no-default-features --features codegen` 180 lib + 6 + 19 + 1 + 3 全绿；
> 5 三组均在 `crates/unsyn` 里 0 条；6 五组（含 `--document-private-items`）均 0 警告；
> 7 干净。新增测试：lib 两条是集合成员重复的四条 lint 端到端钉字
> （`semantics::tests::a_repeated_member_is_reported_in_every_kind_of_rule`、
> `each_relation_between_two_members_is_named_in_its_message`）；bootstrap 与 generated
> 各一条是 P5 的结构断言。fixture `tests/data/mini.rs` 97,948 → **86,801** 字节。
>
> V 重做那一轮（`syn::visit` 那版，见上面的 V）跑过全表：1 全绿（182 lib / 6 bootstrap
> / 19 codegen / 1 dump / 27 generated / **5 visit** / 3 doctests —— lib 没涨是因为上一版
> 加在 `src/visit/trace.rs` 里的 6 条测试随那套 `dyn` 设计一起删了，新的 5 条在
> `tests/visit.rs`），2 十六行全 `ok`（比上一轮多一行，就是 visit 这个测试目标）；3
> runtime-only 2 绿；4 `--features codegen` 与 1 同数，`--no-default-features --features
> codegen` 是 180 lib + 6 + 19 + 1 + **5 visit** + 3 全绿（`generated` 要 `runtime`，那一组
> 不跑）；5 三组均在 `crates/unsyn` 里 0 条（仍只有既有的 `parserc` 4 条、`sourcespan` 1
> 条）；6 五组（含 `--document-private-items`）均 0 警告；7 干净。`tests/data/mini.rs`
> 未动（这一轮没碰 codegen，fixture 的 drift 测试原样通过）。
>
> V2 那一轮（`semantics/` 改成走接口，见上面的 V2）跑过全表：1 全绿，数字与上一轮完
> 全一致（182 lib / 6 bootstrap / 19 codegen / 1 dump / 27 generated / 5 visit /
> 3 doctests）—— 这是本轮的要点：一条测试都没改，也没新增测试；2 十六行全 `ok`；
> 3 runtime-only 2 绿；4 `--features codegen` 与 1 同数，`--no-default-features
> --features codegen` 仍是 180 lib + 6 + 19 + 1 + 5 + 3 全绿；5 三组均在 `crates/unsyn`
> 里 0 条；6 四组均 0 警告（`--all-features --document-private-items` 这一组一开始揭出
> 两条本轮新写的冗余显式链接 `[`walk`](crate::visit::walk)`、
> `[`Visit`](crate::visit::Visit)`，已改成裸链接）；7 干净。行数：`check.rs` 649（迁移前
> 620，涨的是三个 pass 各自要写的空 `visit_input` 与注释）、`graph.rs` 945 → **841**、
> `value.rs` 196 → 233。`tests/data/mini.rs` 未动。
>
> **一条给 P6 的接口观察**（本轮实测撞到的）：不变量 3「叶子不许跳过任何 token」要求
> `walk_lit_str` 把两侧引号也当 `visit_input` 报出去，于是**任何想「只要字面量的内容」
> 的 pass 都不能从 `visit_lit_str` 进入**，必须自己走到 `lit.content` 上去（`Decoder` 就是
> 这么做的，这也是全 crate 里唯一一处需要知道 `LitStr` 字段名的 pass）。可选的改法是把
> `StrSegment::CharWithException` 升成一个真节点（那样它就有自己的方法可以覆盖），代价是
> `tests/visit.rs` 的节点清单从 50 涨到 51、且要为它补一条产生式。本轮没动，因为现状并不
> 阻塞任何事。
>
> V3 那一轮（视图枚举，见上面的 V3）跑过全表：1 全绿（182 lib / 6 bootstrap /
> 19 codegen / 1 dump / 27 generated / 5 visit / **3 shape** / 3 doctests），2 十七行全
> `ok`（比 V2 多一行，就是 `shape` 这个测试目标）；3 runtime-only 2 绿；
> 4 `--no-default-features --features codegen` 是 180 lib + 6 + 19 + 1 + 3 shape + 5
> visit + 3 全绿（`generated` 要 `runtime`，那一组不跑）；5 均在 `crates/unsyn` 里 0 条
> （仍只有既有的 `parserc` 4 条）；6 四组 feature 组合（含 `--document-private-items`
> 那两组）均 0 警告；7 干净。**一条测试都没改**，fixture `tests/data/mini.rs` 也未动。
> 行数：`syntax/expr.rs` 净增 185（上一个提交以来），`semantics/graph.rs` 841 → 820，
> `semantics/check.rs` 649 → 650，`tests/shape.rs` 新 242。
>
> P6.0 那一轮（文件级规则，见上面的 P6）跑过全表：1 全绿（182 lib / 6 bootstrap /
> 19 codegen / 1 dump / 27 generated / 3 shape / 5 visit / 3 doctests），2 十七行全 `ok`；
> 3 runtime-only 2 绿；4 `--no-default-features --features codegen` 是 180 + 6 + 19 + 1 +
> 3 + 5 + 3 全绿；5 `crates/unsyn` 里 0 条；6 `--all-features --document-private-items`
> 0 警告；7 干净。`tests/data/mini.rs` 未动（那九条规则里没有文件级规则，新形状的
> 影响范要到拿 `bootstrap.uns` 自己生成才量得出）。拿 `bootstrap.uns` 生成一次：
> **105,488** 字节、60 个顶层类型、8 个 sum；`Item` 与手写同构（只是变体名不叫 `OuterDoc`
> 而叫 `ItemOuterDoc`），`OuterDoc`/`InnerDoc`/`Crate` 则撞到上面那条归属洞。
>
> **两处计划本身要更正**：`codegen/matchers.rs` 连同"生成的节点类型不带 matcher"
> 那一整个问题**已经不存在**（P5 之后 codegen 被重写过，现在是 `types`、`pattern`、
> `scan`、`render`、`name` 五块），所以本轮原先排在 V 后面的两件小事（把生成的别名
> 节点类型一般化、把 matcher/shape 这组名字改得一看就懂）已经没有对象，作废；
> `src/grammar.rs`（那个临时规则头解析器）也已经删掉，`bootstrap.uns` 现在由手写
> parser 自己读，P1 关于规则头形式的改动只当历史读。
>
> 那一整个问题**已经不存在**（P5 之后 codegen 被重写过，现在是 `types`、`pattern`、
> `scan`、`render`、`name` 五块），所以本轮原先排在 V 后面的两件小事（把生成的别名
> 节点类型一般化、把 matcher/shape 这组名字改得一看就懂）已经没有对象，作废；
> `src/grammar.rs`（那个临时规则头解析器）也已经删掉，`bootstrap.uns` 现在由手写
> parser 自己读，P1 关于规则头形式的改动只当历史读。
>
> **一个更正（上一轮）**：上一轮对三个提问的回答引到了不存在的文件与符号
> （`src/semantic/`、`src/lexical/parser.rs`、`CharClass` 在 semantics、`DuplicateAlt`、
> `MAX_MEMBER_NAMES` 等），所以那三条结论作废。按真实代码重查：
> 集合成员的重复**早就**在 `semantics/check.rs::validate_set` 里检了（四种关系
> `DuplicateMember`/`CoveredMember`/`CoversMember`/`OverlappingMember`，走 lint 通道）；
> `range` 两端必须是单 char 也**早就**在 `validate_range` 里以 `NotAChar` 定了（本轮
> 新加的两条测试把它端到端钉住）；`except` 是按操作符的，生成侧
> `Step::Except { item, removed }`、分图侧 `with_suffix_same_offset` 两处一致，没有洞；
> 左递归用 `first_edges` + `same_offset_edges` 显式栈 DFS（`graph.rs::cyclic_nodes` /
> `visit_edges`）而非计数器，所以也不存在 `saturating_add` 那回事。

## P7 — 收尾

- [ ] 删除本文件里已经过时的"实测状态"数字，或把它们移到 `CHANGELOG.md` 的条目里。
- [ ] `crates/unsyn/CHANGELOG.md` 增补 unsyn 的条目（目前为空）。
- [ ] 已知的既有杂物：`[dev-dependencies] walkdir` 从 V3 起由 `tests/shape.rs` 用上了
      （它要枚举 `src/**/*.rs`），但 `tests/visit.rs` 与 `tests/bootstrap.rs` 仍然自己
      `fs::read_to_string` 单文件；两边统一成一种读法是 P7 的事。

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
