# 产生式建模整改计划（对照 unsyn.us）

规则（本轮确立，适用于 `src/compiler/lexer.rs` 与 `src/compiler/syntax.rs` 的手写 CST）：

1. 产生式右侧一律用 struct field 建模：右侧的每个 sequence 元素都是类型上的一个字段
   （`X*` → `Vec<X>`，`X+` → `OneOrMore`/生成形状，`X?` → `Option<X>`，匿名括号组 → tuple /
   `Delimiter`，punct/keyword → `#[parserc(keyword = … / parser = …)]` 的 `I` 字段）。
   不再用“整体文本 + `read_*` 函数”的 `struct X<I>(pub I)` 形状。
2. 右侧出现的 or，若在中间（不是 root），建模为一个**具名 enum**（手写侧强制，不允许用
   `Choice`/`Or` 组合器嵌套表示匿名 or）。root or 建模为该产生式自己的 enum。
3. 语义分析阶段：`.us` 文件里出现“不是 root 的 Or 表达式”（即括号组里的多选）时，走
   **lint 通道**警告（不报错、不改 spec）。

## 审计：spec 右侧形状 vs 现状

### lexer.rs（全部是 `struct X<I>(I)` + `read_*` 的整体文本形状，需改）

| 产生式 | 右侧形状 | 目标建模 |
|---|---|---|
| `S -> [ … ]` | 集合 | 保持 `take_while` ✅ |
| `LF`/`CR` | 单字符 | 保持 `char` ✅ |
| `IdentOrKeyword -> (XID_Start\|'_') XID_Continue*` | seq，中间 or | `struct { start: IdentStart, tail: Vec<XID_Continue> }`，`enum IdentStart`（具名，手写补名） |
| `XID_Start`/`XID_Continue -> <…>` | 外部调用 | 保持 ✅ |
| `RawIdent -> 'r#' IdentOrKeyword` | seq | `struct { #[parserc(keyword="r#")] sigil: I, name: IdentOrKeyword }` |
| `NonKeywordIdent -> IdentOrKeyword except Keywords` | `except` 算子 | `struct { name: IdentOrKeyword }` + parse 后校验（`except` 不产字段） |
| `Ident -> NonKeywordIdent \| RawIdent` | root or | `enum`（已是；变体内部改为结构化后随之更新） |
| `Keywords -> Leading \| Following` | root or | `enum` ✅ |
| `LeadingKeywords -> 'lexer'\|'syntax'\|'whitespace'` | root or | `enum`，三个 keyword 变体（`Decl::from_word` 改为 match 变体） |
| `FollowingKeywords -> 'concat'\|'except'\|'followed'` | root or | `enum` 同上 |
| `Oct/Dec/HexDigit -> ['0'-'7']` 等 | 集合 | 保持 `next_if` ✅ |
| `LitDec -> DecDigit (DecDigit\|'_')*` | seq，中间 or | `struct { head: DecDigit, tail: Vec<DigitOrUnderscore> }` + `enum DigitOrUnderscore` |
| `LitUnicode -> 'U+' concat HexDigit{4}` | seq(concat) | `struct { prefix: I(keyword), digits: Vec<HexDigit> }`，4 个的约束放语义/校验 |
| `QuoteEscape -> '\\''` | 单 keyword | 保持 ✅ |
| `AsciiEscape -> '\\x' OctDigit HexDigit \| '\\n' \| …` | root or，其中一个 arm 是 seq | `enum { Hex { marker: I, oct: OctDigit, hex: HexDigit }, N(I), R(I), T(I), Backslash(I), Zero(I) }` |
| `UnicodeEscape -> '\\u{' (HexDigit '_'*){1,6} '}'` | seq，组内 seq+计数 | `struct { marker: I, groups: Vec<DigitGroup>, close: I }`（`DigitGroup` 手写补名）；1..6 计数校验 |
| `LitStr -> '\'' (…)  '\''` | seq，中间 or | `struct { open: I, body: Vec<LitStrContent>, close: I }` + `enum LitStrContent { Plain(I), Quote(QuoteEscape), Ascii(AsciiEscape), Unicode(UnicodeEscape) }` |
| `LineComment` | 已从 spec 删除 | 已删 ✅ |
| `BlockComment`/`InnerBlockDoc`/`OuterBlockDoc` | seq，中间 or，嵌套计数 | 需结构化 + 手写补名（`BlockCommentHead`/`BlockCommentTail` 等）；扫描函数保留但产出结构化节点 |
| `BlockCommentOrDoc -> A\|B\|C` | root or | `enum` ✅ |

### syntax.rs（大体已符合，遗留点）

| 位置 | 问题 | 处理 |
|---|---|---|
| `SuffixGroup<I> = Choice<Star, Choice<Question, Choice<Plus, Repeat>>>` | 匿名 or 用 Choice 嵌套 | 改为具名 `enum Suffix`（手写侧强制） |
| `ExprWithoutSuffix` 各 arm / `Set<I>` / `Alternative<I>` / tuple | seq 组用 tuple/Delimiter | 符合规则 1 ✅ |
| `Repeat`/`RepeatBound`/`Range` | struct+enum 已结构化 | ✅（`RepeatBound` 是具名 enum，恰好是 spec 未命名的组——保留并注明补名理由） |
| `Crate` | 手写 `impl Syntax`（字段化 OK，为 EOF 检查） | 保留 |
| `literal.rs` | 存在的唯一理由 = 整体文本网格 | lexer 结构化后删除，值改为从子节点读取 |

## 阶段

- [x] **M1 lint**：`LintKind::UnnamedAlternative`；`Checker::visit_expr` 深度计数，
      非 root 的多分支 Expr → `diagnostics.lint`；单测钉住（spec 片段 + `unsyn.us` 全文
      恰好 10 处未命名 or 组）。
- [x] **M2 手写 lexer.rs**：全部终端按 spec 形状结构化——sequence → struct field，
      root or → enum，中间 or → 手写补名 enum（`IdentStart`/`LitStrContent`/
      `DigitOrUnderscore`/`EscapeDigit`/`BlockCommentHead`/`OuterBlockDocHead`/
      `BlockCommentTail`/`LineDocBody`）；字符集 → `next_if`/`take_while` 谓词；
      keyword → `keyword` 属性 + `map_err`。仅存的自定义解析：`except` 算子
      （`read_non_keyword_ident`）、重复计数 `{n}`/`{1,6}`（`LitUnicode`、
      `read_escape_digits`）——都是 spec 算子本身，不是发明行为。
      **按 spec 直读纠正**：`keyword_word`（长名拒绝）等发明语义已删除；
      `lexerX` 现在按文法读作 `lexer` + `X`；`////` 读作空 outer doc；
      未闭合注释按“哪个形态都不匹配/缺少的收尾在哪”报告。
- [x] **M3 手写 syntax.rs**：`SuffixGroup`(Choice 嵌套) → 具名 `enum Suffix`；
      `Stmt::decl()` 改为 match `LeadingKeywords` 变体；`syntax_word!` 直接用
      `keyword` 属性；`visit.rs` 与所有新节点同步（文本重建测试钉住顺序完整）。
- [x] **M4 删 literal.rs**：值读取挪到节点（`chars()/value()/count()/text()`），
      `semantic.rs` 与 `codegen.rs` 的调用点全部改走节点；非法码点的报告位置
      改为 escape 节点自身的 span。
- [x] **M5 生成器**：`codegen.rs` 对 lexer 规则输出与手写侧同构的建模——
      sequence → 每个操作数一个 field（引用规则 → `X<I>`/`Option<X<I>>`/`Vec<X<I>>`，
      匿名操作数 → `I` + 属性注入 parser）；右侧全部匿名的 sequence → tuple struct；
      无名字段按 `f1`/`f2`…；全引用选择的 root or → enum（变体名=规则名）；
      node/text 形状由 `node_rules` 不动点判定（fold 臂、集合折叠、except 目标、
      单名规则/单名组的答案会流回文本位置 → 该规则降级为文本）；
      `{n,m}` over 引用 → `read_` 计数函数返回 `Vec<X<I>>`；字符类重复 →
      `take_while(_with)` 直接写进属性。
      重生成 `tests/generated/unsyn_us.rs`；冒烟测试（两种流）全部适配新形状。
      中间 or 组的具名 enum 生成：按裁定取消——生成侧匿名中间 or = `I` 字段 +
      注入 parser，不造名（手写侧侧保留补名 enum，规则 2 只约束手写）。
- [x] **M5.5 BuildError → errors**：`BuildError` 消失，改为
      `UnsynError::Build(BuildKind, Span)`；`BuildKind`（External/Whitespace/Name/
      Literal/Syntax）住在 `errors.rs`，与 Punct/Keyword/Syntax/Semantics 四族同列，
      实现同一套 `ParseError` 约定（to_span/control_flow/into_fatal）；
      `build()` 返回 `Result<TokenStream, UnsynError>`。
- [x] **M6 门禁**：`cargo fmt --all`; `check -p unsyn --no-default-features`;
      `test -p unsyn --all-features`; `test --workspace`;
      `clippy --workspace --all-targets`（0 warning）; `RUSTDOCFLAGS=-D warnings
      cargo doc --workspace --all-features`。（M5 后已全部重跑通过。）

## 后续（本计划范围外，自举的最后一步）

- [x] **M6.5 spec：IdentOrKeyword 改为 external**（M7 前置，本轮完成）：
      `unsyn.us` 的 `lexer IdentOrKeyword -> ( XID_Start | '_' ) XID_Continue*;`
      替换为 `lexer IdentOrKeyword -> <IdentOrKeyword>;`，并删除随之孤立的
      `XID_Start`/`XID_Continue` 两条产生式（26→24 条 lexer 规则，unnamed-or
      lint 10→9）。手写侧 `IdentOrKeyword` 相应变为单字段文本节点 +
      `read_ident_or_keyword`（即该 external 的手写绑定：start 为
      `is_xid_start(c) || c=='_'`，随后 `take_while(is_xid_continue)`），
      `IdentStart`/`XID_Start`/`XID_Continue` 三个类型与对应 visit 钩子删除；
      生成侧 fixture 重生成，external 由测试绑定到
      `crate::scan_ident_or_keyword`。动机：Id 处理更高效（一次扫描而非逐字符节点）。
- [x] **M7 syntax 规则生成**：`BuildKind::Syntax` 闸门解开，`syntax` 产生式
      生成树节点——root or → enum（变体名：单引用臂=规则名；字面量词臂=词的
      首字母大写，如 `Concat`/`Except`/`Followed`；其余 `F1`/`F2`…按位置）；
      sequence → struct/tuple（全匿名 → tuple）；操作数 → 字段：引用 =
      `Box<X<I>>`（syntax）或 `X<I>`（lexer），单字符字面量 = `Char<I, c>`，
      多字符字面量 = `I` + `keyword` 属性，`?`/`*`/`+` = `Option`/`Vec`/
      `(T, Vec<T>)`，匿名中间 or = 嵌套 `Or`（零造名），匿名组 = 内联 tuple；
      操作数间隙 = `Option<S<I>>` 字段（S = 文件的 whitespace 规则；重复的操作数
      若自身可读 trivia——如 `Item`——则不加间隙）；`concat` = 相邻字段无间隙；
      `followed`/`except` = 注入 probe helper（保留目标节点值）。拒绝的形状：
      节点计数重复 `{n}`、组内算子、组内多字符词/Call/非字符类 Set
      （`BuildKind::Syntax`）。`node_rules` 不动点重写为**预演算法**：enum 变体
      折叠同样要求引用规则降级为文本（`Keywords`→`LeadingKeywords` 传染）。
      重生成全量 fixture（lexer+syntax），新验收测试
      `the_generated_parser_reads_the_spec_it_was_generated_from`：生成的 parser
      解析 `unsyn.us` 全文到 EOF，且每个定义的名字 span 与手写树逐一相等
      ——自举第一个闭环。
- [x] **M7.5 visit 接口生成**：`build()` 在节点之后输出 `pub trait Visit` +
      `pub mod walk`（syn 形状）：每个规则一个 `visit_<snake(规则名)>` 钩子
      （默认走对应 `walk_`），叶结点统一走 `visit_text`。descents 不依赖额外
      簿记——直接从已写好的字段类型 token 串解析回形状（`Vec/Option/tuple/
      Or/Char/Box` 的严格文法，`ty_shape`），walk 即按形状下钻；Box 透传
      （引用经 deref coercion）。visit 模块整体 `#[rustfmt::skip]` +
      `#[allow(...)]`，保证 freshness 逐 token 一致。
      新验收测试 `the_generated_visitor_hands_over_every_character_once`：
      生成的 visitor 走生成的 parser 对 `unsyn.us` 的树，每个字符恰好交付
      一次、顺序完整——与手写 visit 的不变量同源。
- [ ] **M8 完全自举**：semantic/codegen 两个 pass 迁移到只经 visit 接口 +
      值访问器读取树（不依赖具体节点类型），或以生成器再生一份自身
      （generated-from-generated 恒等检验），以生成的 parser 替换手写
      parser 解析 `.us` 文件。
