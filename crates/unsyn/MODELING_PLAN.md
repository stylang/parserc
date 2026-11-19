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
- [ ] **M5 生成器**：`codegen.rs` 对 lexer 规则输出同样的 struct-field/具名 enum 建模
      （中间 or 合成名字，与手写命名一致）；重生成 `tests/generated/unsyn_us.rs`；
      冒烟测试更新。
- [ ] **M6 门禁**：M5 完成后重跑全套：`cargo fmt --all`; `check -p unsyn
      --no-default-features`; `test -p unsyn --all-features`; `test --workspace`;
      `clippy --workspace --all-targets`（0 warning）; `RUSTDOCFLAGS=-D warnings
      cargo doc --workspace --all-features`。（当前一轮已全绿。）
