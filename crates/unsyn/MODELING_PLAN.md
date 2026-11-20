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
- [x] **M8.1 双树对等检验**：新测试 `crates/unsyn/tests/two_trees.rs`——
      同一份 `unsyn.us` 分别由「手写 parser + 手写 visitor」与「生成 parser +
      生成 visitor」走一遍，记录 (visit_<规则>, span) 事件序列并逐条比对。
      手写侧已改为生成侧的 junction 模型（终端只持有自己的文本，操作数之间的
      run 是序列的显式 `Option<S<I>>` 字段），所以两棵树的 span **直接相等**，
      不再有任何归一化：不裁空白，不过滤空 span。
      当时仅剩一件投影，且它是真实的形状差异：生成侧降级为文本的规则
      （non_keyword_ident / lit_str / lit_unicode / unicode_escape /
      ascii_escape / block_comment 家族）其内部子事件在生成树中不存在，
      手写序列按区间栈投影掉。其余 900+ 事件严格相等：规则、顺序、区间
      完全一致。`TEXT_INTERIORS` 列的是「生成代码按文本读取的规则」全集，
      而非今日 `unsyn.us` 恰好触及的子集，规范变化时投影仍然成立。
      （**后续**：M8.2b 把左边那侧的自造名节点全部去掉后，这件投影已
      整体删除，见下方 M8.2b 验收条目。）
- [ ] **M8.2 完全自举**：以生成的 parser 替换手写 parser，形成编译自身的闭环。
      审计结论（三份调查）：`semantic.rs` 的语义逻辑本身已与类型无关（lattice /
      区间代数 / 名字解析 / 递归 worklist 全跑在 `Vec<Rule>` + `String` 上），
      耦合集中在 ~66 行；`codegen.rs` 有 52/93 个函数直接读节点，但读的全是
      **产生式结构**，从不读手写类型的 Rust 名字（唯一例外 `ty_shape` 把自己
      吐出的类型串再解析回来，~135 行，应删）。两树 visit 钩子 67 vs 38。
      收敛方向（已定）：**分层** —— syntax 层的组补规则名让生成侧产出真节点；
      lexer 层保住 `take_while`/`read_*`/external 快路径，pass 需要的字符值由
      pass 自己从文本解码。
  - [x] **M8.2a syntax 层补名**：`unsyn.us` 新增 `Set` / `Suffix` / `RepeatBound`
        三条规则，`ExprWithoutSuffix`/`ExprWithSuffix`/`Repeat` 改为引用它们。
        生成侧随之产出 `struct Set` / `enum Suffix` / `enum RepeatBound` 三个真
        节点，手写侧对应钩子 `visit_set`（`walk_set` 转 pub、经钩子下钻）。
        顺带修一个真实缺陷：`ExprWithoutSuffix::Tilde` 原为 tuple 变体，`~` 与
        `[` 之间的 run 无处存放（`~  ['a']` 不可回读）；改为 struct 变体
        `{tilde, s, set}` 与生成侧 `F1{f1,f2,set}` 对齐，并加测试
        `a_negation_keeps_the_run_between_its_punct_and_its_set` 钉住。
        未命名 or 组 9→7；定义数 37→40；`two_trees` 记录器加入
        `visit_set`/`visit_suffix`/`visit_repeat_bound` 三个钩子以钉住收敛，
        并加断言：这三个钩子在**两侧事件序列里都真的出现过**，否则对等检验
        对这些规则是空转的。
  - [ ] **M8.2b lexer 层对齐**：手写侧去掉自造名内部节点，改为与生成侧相同
        的“文本 + 边界”形状；新增 `compiler::decode` 模块，把
        `LitStr::chars`/`LitUnicode::value`/`LitDec::count`/`Ident::text` 从节点
        方法变成对源文本的解码器（带 span）。验收：`two_trees` 的
        `TEXT_INTERIORS` 投影清空，两树事件序列裸相等。
    - [x] **M8.2b-1 `LitDec`**：删掉 `DigitOrUnderscore`，`LitDec` 改为
          `{head: DecDigit, tail: I}` + `take_while(['0'-'9' '_'])`，与生成侧
          `{dec_digit, f1}` 对齐；`count()`/`text()` 改为对 tail 文本去 `_`。
          **修掉一个真实潜伏缺陷**：多字符计数（`{12, 340}`、`{1_000}`）原先
          两树不等——手写逐字符发 `visit_dec_digit`，生成侧只有一段文本。
          `two_trees` 重构出 `both_trees(source)` + `assert_one_sequence`，新增
          `a_count_above_one_digit_is_one_token_in_both_trees`：`unsyn.us` 的计数
          全是单字符，规格自己发现不了这个分歧，故另造一份含多位计数的源。
          已验证该测试有牙（往 `walk_lit_dec` 里添一个多余事件即失败）。
    - [x] **M8.2b-2 `LitUnicode`**：手写侧改为单字段文本
          `LitUnicode<I>(parser = read_lit_unicode)`，与生成侧
          `keyword("U+") + take_while_with(4..=4, hex)` 同形；`visit_lit_unicode`
          从 `TEXT_INTERIORS` 去掉（已逐项开关验证：现在只有
          `non_keyword_ident`/`lit_str` 两项真被需要，其余因嵌在
          `lit_str` 区间内或本文件未出现而暂不需要，保留是为了描述形状）。
          **修掉一个比形状更严重的问题：两侧接受的语言不同**。手写侧用
          `take_while` 取光十六进制数字再要求 `len()==4`，所以 `U+11234`
          被拒绝；生成侧只取 4 个，`U+1123` 是 token、剩下的 `4` 属于下
          一个操作数。spec 写的是 `HexDigit{4}`，没有说“不得更多”，所以
          手写的拒绝是自造假设 —— 两个 parser 认不同语言就无法自举。
          现在以 spec 为准，但保留更好的报错：数字在副本上计数，所以
          span 仍覆盖已读的全部数字，且失败时不吞掉 digit（只消耗
          `U+`）。测试 `a_unicode_literal_is_exactly_four_digits` 重写：
          `U+11234` 现在断言 token 为 `U+1123` 并留下 `4`。
    - [x] **M8.2b-3 `LitStr`**：手写侧 `LitStr` 改为 `{open, body: I, close}`，
          `body` 由 `read_lit_str_body` 一次取完（用四种 piece 定边界但丢弃
          节点），与生成侧 `read_lit_str_1` 同形；删 `LitStrContent`。
          新增 `compiler::decode` 模块（唯一入口 `string_body(text, at)`），
          把转义解码从节点方法变成对源文本的解码器，返回 `chars` + 非法
          `\u{…}` 的 **span**（`at` 用 `Input::start()`，所以报错仍指向具体
          转义而不是整个字面量）；`semantic::chars_of` 改用 `lit.decoded()`。
    - [x] **M8.2b-5 `Ident` 家族**：`RawIdent` / `NonKeywordIdent` 改为单字段
          不透明文本（`read_raw_ident` / `read_non_keyword_ident` 返回整段
          token），与生成侧一致；手写树不再在 name 内部发
          `visit_ident_or_keyword`。
    - 投影列表从 9 项缩到 4 项，且逐项开关实验确认：现在只剩
          `block_comment` 家族（`inner_block_doc` / `outer_block_doc` 两项经
          实验证明是**真正在起作用**的，删掉即失败）。
          新增 `a_block_comment_is_read_as_parts_by_one_tree_and_as_text_by_the_other`：
          `unsyn.us` 全用行文档，永远走不到注释家族，所以主对等测试对它是
          空转；这个测试用含 `/*! /* */ */` 与 `/** /* nested */ */` 的源把
          差异固定下来，口头上写明“今天靠投影才通过”而不是已收敛。
    - [x] **M8.2b-4 注释/文档家族**：手写侧 `BlockComment` / `InnerBlockDoc` /
          `OuterBlockDoc` / `BlockCommentOrDoc` 均改为单字段不透明文本，
          手写 reader (`read_block_comment_body` / `read_block_comment` /
          `read_inner_block_doc` / `read_outer_block_doc` /
          `read_block_comment_or_doc`) 逐字对齐生成侧的 `read_block_comment_*`
          组合子顺序；删 `BlockCommentHead` / `OuterBlockDocHead` /
          `BlockCommentTail` / `LineDocBody` 四个自造名类型及对应钩子。
          `OuterLineDoc.body` 也从 `Option<LineDocBody>` 改为可空文本段（对齐
          `read_outer_line_doc_2`）。

          **手术前先加了特征测试** `a_document_holds_the_group_it_nests_and_
          closes_once`（多位计数那个同类做法）钉住“接受什么语言”，改完它仍然
          绿——说明只有形状变了，语言没变。

          过程中用探针确认了两件之前不确定事：
          1. `/** x /***/ */` 两侧**同样**失败（嵌入的 `/***/` 偷走外层
             closing）——这是共享的语法限制，不是对等差异，也不是本步要修的；
          2. `/***/` 单独作为 `BlockComment` 两侧都读成 5 个字符（不是生成侧
             先把 `/**/` 抢走），所以 spec 里 `/**/` 在 `/***/` 之前不构成
             LL 缺陷——先前手写注释里“必须换页”的说法是错的，已改。

          报错 span 变了一处（唯一预期内的行为变化）：未闭合注释从
          `0..1`（“选择未满足处”）变成 `0..5`（“能读到的最长形式”），
          因为生成侧就是 `0..5`，手写侧现在跟它一致；已更新
          `an_unterminated_group_is_answered_by_no_form_of_its_rule` 并写明理由。

    - [x] **M8.2b 验收达成：归一化降为零**。`two_trees` 的 `TEXT_INTERIORS`
          区间栈投影已**整体删除**，`both_trees()` 现在就是“两边各自记录事件
          序列”，`assert_one_sequence()` 直接比较——无任何裁削、无任何过滤。
          钩子集：生成侧 41 个是手写侧 62 个的**严格子集**，多出的 21 个
          = 20 个 punct 终端（两个 pass 对其类型引用站点为 0，记录器也不记）
          + `visit_escape_digit`（在 `UnicodeEscape` 内部，而 `LitStr` 已是
          文本，故两侧都不可达）。
          注释家族测试改名为 `a_block_comment_is_one_token_of_text_in_both_trees`，
          它现在是断言“无投影也相等”的正向用例。
    - [x] **M8.2b-6 转义与字段数（生成器改造）**：用户改 spec 后提出硬规则
          ——**产生式右侧有多少个子表达式，就生成多少个 Field**。
          根因定位（实验证据，非推断）：`lexer U -> 'ab' H{1,6} 'c'` 单独生成时
          是 struct 多字段，一旦被另一个规则的内部匹名选择引用（如 `LitStr`
          的 body）就坡缩成单个 `read_*` 文本字段。因为折叠（`.or()` 链）
          要求所有分支回答同一类型，而 `reference()` 对非节点规则发
          `.map(|node| node.0)`——于是 `node_rules` 把被折叠引用的规则全部
          降级成 Text。

          修法：生成一个文件级 `text_of<P, I>(parser, input) -> I` 辅助函数，
          在折叠处把“回答节点”的分支包成 `|input| { text_of(p, input) }`，
          并删掉 `atom_forced` 里 `Reference if folds` 的降级传染。
          **只包需要的分支**（`branch_yields_node`：仅“单独一个名字引用且它是
          节点”），否则词法最热的逐字符循环会每个字符多一次 `input.clone()`。

          效果（生成侧字段数）：`RawIdent` 1→2、`UnicodeEscape` 1→3、
          `LitUnicode` 1→2、`InnerBlockDoc` 1→3、`OuterBlockDoc` 1→4、
          `BlockCommentOrDoc` 文本→enum(3)。`LitDec`/`LitStr`/`NonKeywordIdent`/
          `QuoteEscape`/单字符集本来就是 1 个子表达式 → 1 字段 ✓。

          附带修两个真 bug（都是“降级取消后首次被走到”的代码路径）：
          ① 有界重复发 `else { break }`，rustfmt 会改写成 `{ break; }` →
            保留文件与新生成文件 token 不等，freshness 失败；
          ② 下界为 1 时发 `taken.len() < 1usize` → clippy `len_without_is_empty`，
            改为 `taken.is_empty()`。
          又：`is_sequence` 不看穿 `concat`，所以 `LitUnicode -> 'U+' concat
          HexDigit{4}` 被当成单子表达式；新增 `operand_count`/`operands_of` 沿
          `concat` 链展开计数后修为 2 字段。

          手写侧同步：`RawIdent` 回到 `{sigil, name: IdentOrKeyword}`、
          `UnicodeEscape` 改为 `{marker, digits: Vec<HexDigit>, close}`（删
          `EscapeDigit` 与 `'_'*`）、`LitUnicode` 改为 `{prefix,
          digits: Vec<HexDigit>}`、`Range::Unicode` 去掉两个 junction（spec 已改
          `concat`）并与 `Range::Str` 同形。

          **又找到并修了一个活的分歧**：`unsyn.us` 全文没有 `r#`，所以双树
          比对从来没走到 `RawIdent`——手写侧读成一整段文本、生成侧读成
          “sigil + 名字节点”，用 `syntax r#where -> 'a';` 一探就 `EQ false`。
          已修并新增 `a_name_written_raw_is_two_parts_in_both_trees` 钉住。
    - [x] **M8.2b-7 lexer 侧 root-or 一律成 enum + 混合备选警告**：用户定的硬
          规则：**除非右侧是一个基础 literal 表达式，否则第一层必须是 struct
          fields 或 enum**；多种复杂表达式混合时，该原则只适用于第一层，
          并给出警告。

          生成器：`node_rules` 的形状判定从 `named_choice(body).is_some()`
          改为 `!body.is_single()` —— 只要根上是选择就成 enum，不管备选是
          名字、字面量还是序列；`stmt()` 里的 lexer enum 分支从“只能装名字
          备选”扩展为与 `syntax_item` 同构的通用版（名字备选 → `Variant(Ty)`，
          字面量备选 → 由词命名，其余 → `branch fields` + `F{n}` 兑底）。
          `named_choice` 关心被删（其功能已被逐项 `one_reference` 覆盖）。

          警告：新 `LintKind::MixedAlternative`，在 `visit_expr` 根层
          (`expr_depth == 0`) 统计各备选写字段数：全部一个或全部多个 → 不警告；
          两种混在一起 → 警告。判据 `writes_one_field`：单个元素且为
          Reference/Str/Unicode/Set。**注意一个名字引用不管它指向的规则多宽都
          算窄**（变体只装那个节点一个字段），所以 `A | 'c'` 不警告，
          `'a' 'b' | 'c'` 警告。
          `unsyn.us` 命中 4 处（`RepeatBound`、`ExprWithoutSuffix`、
          `AsciiEscape`、`BlockComment`），已钉住计数。

          生成侧结果：`LeadingKeywords`/`FollowingKeywords` → enum(3)，变体名与
          手写侧完全一致（`Lexer/Syntax/Whitespace`、`Concat/Except/Followed`）；
          `AsciiEscape` → enum(6)（首个备选是 3 字段 struct 变体）；
          `BlockComment` → enum(3)（F1 是 4 字段 tuple 变体）。

          验收发现：`lexer A -> 'a' | 'b'` 的变体现在叫 `A`/`B`（由词命名），
          而不是 `F1`/`F2` —— 现有 `variant_name` 规则自然覆到 lexer 层，
          不是 regression。`the_trivia_of_a_lexer_rule_is_not_taken` 已按新形状
          重写（包含“单个字面量仍是 1 字段”的反例）。

          **尚未完的一层**：手写侧的 `BlockComment`/`InnerBlockDoc`/
          `OuterBlockDoc`/`BlockCommentOrDoc` 还是单字段文本（本轮 M8.2b-4
          改的），而生成侧现在分别是 enum(3)/3 字段/4 字段/enum(3)。两边
          事件序列仍然相等（`visit_text` 不被记录，且注释内部结构两侧都
          不可达），所以 `two_trees` 全绿——但字段结构不等，M8.2d 换 AST 时
          必须先把这四个同步回去（且比 M8.2b-4 更简单：生成侧的 head/body
          已经是纯文本字段，不再需要手写侧那三个自造名 enum）。
    - [ ] **M8.2b-8 手写侧注释家族回同步**：见上条末尾。
  - [ ] **M8.2b-6 转义类型收尾（外观性）**：`AsciiEscape`（enum）与
        `UnicodeEscape`（`Vec<EscapeDigit>`）仍是结构化类型，但 `LitStr` 的
        body 变成文本后它们已经**不可达**——两个树的 walk 都不会走到，
        因此不再是对等差异，只是手写侧多余的种类。可顺手改成不透明
        文本并删 `EscapeDigit`，也可以不做。
  - [ ] **M8.2c codegen 去类型化**：`Builder::new` 改收类型无关的视图
        （definitions / branches / operands / atoms / quantity / bound / value /
        text / span），52 个函数签名重指向；删 `ty_shape` 的字符串回转，改为在
        构造 `Field` 时就携带 `TyShape`；`Stmt::decl()` 的关键字→Decl 映射搬到
        pass 侧（对关键字文本做比较）。
  - [ ] **M8.2d 生成 AST 进 crate**：`Builder` 参数化「输入 trait 路径、错误
        kind 路径、外部符号的 crate 根、可选 derive（serde）」，使 `unsyn.us`
        的生成代码能作为 `src/compiler/ast.rs` 在 crate 内对 `UnsynInput` 编译；
        补 codegen 输出 `cfg_attr(feature = "serde", ...)`。
  - [ ] **M8.2e 不动点**：手写 AST 退役，两个 pass 只吃生成 AST；测试断言
        `build()` 读生成树 == 现有生成文件（逐 token），`unsyn.us` 由生成
        parser 解析。
- [ ] **M8.3 门禁补齐**：`two_trees` 已加 `required-features = ["compiler"]`；
      CI 目前不跑 `--all-features`/clippy/rustdoc，可考虑补。