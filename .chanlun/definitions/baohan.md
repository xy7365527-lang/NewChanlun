# K线包含处理（Baohan / Inclusion Merge）

> 正本：本文件是 K线包含处理 的教义正本。
> 最后裁定：[#1565](https://github.com/xy7365527-lang/NewChanlun/issues/1565)（2026-10-20，未定前缀上 W 对象的发布与 `pending_since_cut`）；前次 [#1342](https://github.com/xy7365527-lang/NewChanlun/issues/1342)（2026-10-13，初始方向与全包含前缀）
> 受影响代码清单：见正文「#1565 受影响代码清单」与「#1342 受影响代码清单」（后者工作草稿副本 `.chanlun/review-results/issue1323-grilling-20261013/1342-AFFECTED-CODE.md`，票关后随归档出仓）

**版本**: v1.7
**状态**: 已结算
**最后更新**: 2026-10-20
**变更**:
- v1.7 落 [#1565](https://github.com/xy7365527-lang/NewChanlun/issues/1565)：新增「未定前缀上 W 对象的发布与 `pending_since_cut`」节——定义为对象携带的到达事实、逐代原样延续；写明 S3「生成态不写生命周期元数据」的出处并就 W 所需一面正式化；附受影响代码清单
- v1.6 补录 [#1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)：组内极值根是**集合**（按到达序），组锚≠极值根且极值根可多根；集合语义进笔的 raw 间隔（见 `bi.md` §7.1），本文件不另立判据
- v1.5 落 [#1342](https://github.com/xy7365527-lang/NewChanlun/issues/1342)：初始方向改为「等待首个 raw 非包含严格对」（W）；新增「未定前缀的输出与身份」节与受影响代码清单；未结算问题 3 关闭
- v1.0 初版
- v1.1 尝试用等价性证明结算方向判定问题（已回退）
- v1.2 回退为生成态，增加原文谱系，重新定位方向判定差异
- v1.3 结算：方向判定双条件与单条件在非包含前提下数学等价（审计验证）
- v1.4 补录「合并后 bar 保留组内首根原始序号」与三形态载体关系（SPEC #847 S9 / #817 N-1 裁定四）
**原文依据**: 博文第62课、第65课、编纂版 §四

---

## 原文谱系

包含处理概念在缠师108课中的演化：

### 第62课 (2007-06-30) — 首次引入

> "相邻两K线可以出现包含关系，也就是一K线的高低点全在另一K线的范围里"

- **包含关系**：首次定义，纯几何概念
- **合并规则**：向上取 max(high), max(low)；向下取 min(high), min(low)
- **方向判定**：未形式化，只用"在向上时"/"当向下时"的表述
- **顺序原则**：未在此课明确提出
- **特点**：操作性描述，假设读者"看图就知道"什么是向上向下

### 第65课 (2007-07-24) — 形式化补完（关键课）

> "假设，第n根K线满足第n根与第n+1根的包含关系，而第n根与第n-1根不是包含关系，那么如果gn>=gn-1，那么称第n-1、n、n+1根K线是向上的；如果dn<=dn-1，那么称第n-1、n、n+1根K线是向下的。"

- **方向判定形式化**：单条件，`gn>=gn-1 → 向上`，`dn<=dn-1 → 向下`
- **等号问题**：原文用 `>=` 和 `<=`，有注释"应去掉等号"（可能是编者注，非缠师原话）
- **缠师自问自答（L26）**：

  > "有人可能又要问，如果gn<gn-1且dn>dn-1，算什么？那就是一种包含关系，这就违反了前面第n根与第n-1根不是包含关系的假设。同样道理，gn>=gn-1与dn<=dn-1不可能同时成立。"

  缠师在此明确指出：交叉情况（高降低升）本身就是包含关系。这是从包含的定义出发的推论，不是方向判定的规则。
- **顺序原则**：结合律、不遵守传递律、先左后右
- **唯一性宣言**：处理结果"客观、唯一、不以任何人的意志为转移"

### 第67课 (2007-08-02) — 概念迁移

- 包含关系从K线扩展到**特征序列元素**
- 特征序列的包含处理规则同K线
- "标准特征序列" = 经过包含处理后的特征序列

### 第71课 (2007-08-23) — 作用域限制

- **关键约束**：特征序列元素的包含关系，前提是元素在**同一特征序列**里
- 不同特征序列之间的元素讨论包含关系无意义
- 转折点前后两元素不存在包含关系（不同性质）
- 转折点后的分型元素可以应用包含关系（同一性质）

### 第77课 (2007-09-13) — 操作巩固

- "按定义，只要把包含关系搞清楚，相信连孔男人都应该能描红一番"
- 强调先做包含处理和结合律处理，再标记分型

### 第82课 (2007-09-19) — 市场心理扩展

- 包含关系的市场含义：**犹豫、不确定的观望**
- 完全无包含的分型 = 市场双方直截了当
- "直接把阳线以长阴线吃掉"是最坏的包含
- 从分型力度角度分析包含的不同模式

### 谱系小结

| 阶段 | 课号 | 性质 |
|------|------|------|
| 直觉引入 | 62 | 告诉你怎么做 |
| 形式化 | 65 | 告诉你为什么这么做 |
| 概念迁移 | 67 | 同样的定义用到更高层 |
| 作用域 | 71 | 什么时候不能用 |
| 巩固 | 77 | 再次确认操作流程 |
| 深化 | 82 | 从市场心理角度理解包含 |

---

## 定义

### 包含关系（自足定义）

两根K线存在包含关系，当且仅当一根K线的高低点区间完全在另一根K线范围内。

形式化：K1 包含 K2 当 `K1.high >= K2.high AND K1.low <= K2.low`。
反之 K2 包含 K1 当 `K2.high >= K1.high AND K2.low <= K1.low`。
任一成立即为"存在包含关系"。

> 包含关系是纯几何定义，不依赖方向、不依赖时间顺序。两个区间的包含关系是它们之间固有的。

### 合并方向

当相邻两根K线存在包含关系时，需要确定合并方向。方向由此前最近一对**无包含关系**的相邻K线决定：

**原文（第65课 L24）**：单条件
- `gn >= gn-1` → 向上（注：有编者注建议去掉等号）
- `dn <= dn-1` → 向下

**代码实现**：双条件
- `high_i > high_{i-1} AND low_i > low_{i-1}` → dir = UP
- `high_i < high_{i-1} AND low_i < low_{i-1}` → dir = DOWN
- 否则 dir 保持不变

**差异记录**（不急于结算）：
- 原文用单条件形式表述，缠师在第65课 L26 从包含的定义出发解释了为什么交叉不可能
- 代码用双条件，是一种防御性实现
- 两者在非包含前提下数学结果相同，但**概念路径不同**：原文从"包含的定义→推论交叉不可能→所以单条件够用"出发；代码直接检查双条件
- **等号问题**也仍然开放：原文用 `>=`/`<=`，代码用 `>`/`<`

**初始方向（#1342 裁定，`[新缠论:选择]`）**：dir 初始为未定。在首个相邻 raw 非包含严格对 (k, k+1) 出现之前，全部原始 K 线保留为「未定前缀」：逐 raw 原件可查、带未定方向状态，不输出合并组、不伪造方向。该对出现的 cut，以其方向为初始方向，自序号 0 起按上文左折叠处理并发布；之后每步仍按实际 acc 与 incoming 的非包含关系更新方向。外部「历史 seed」不另立档：有 raw 证据即属本规则，无 raw 证据的方向是发明。用随后首个非包含对的方向折叠此前的全包含前缀，是本仓的选择；相邻全包含但非链式的输入（如 `[0,10],[2,8],[1,11]`）长期没有非包含对，就保持未定，不另造默认方向。

**原文边界**：`065-第65课.md:24`【正文】以「第n根与第n-1根不是包含关系」为前提（行尾「(注：应去掉等于号)」为编者注，不取）；`065-第65课.md:28`【正文】只对已走出的 K 线下结论。原文未给无前提时的方向。

**发生史**：本文件 v1.4 及以前记「dir 初始为 None，遇包含默认按 UP 合并」，旧 Python `a_inclusion.py` 以 close/open 推断未定期方向；二者均无原文依据（v1.4 未结算问题 3 自记），不得作为授权。

### 合并规则

- **向上合并**：`new_high = max(h1, h2)`, `new_low = max(l1, l2)`
- **向下合并**：`new_high = min(h1, h2)`, `new_low = min(l1, l2)`
- **OHLC 工程处理**：`open = 左K.open`, `close = 右K.close`

### 顺序原则

**先左后右**递推（第65课 L20）：
1. 从第1根K线开始，依次检查后续K线
2. 若当前 merged bar 与新K线存在包含关系 → 按当前方向合并
3. 否则 → 更新 dir，新K线作为新的 merged bar
4. 完成后序列无任何相邻包含关系

**性质**（第65课 L20）：
- 包含处理遵守结合律
- 不遵守传递律（A 包含 B 且 B 包含 C，不代表 A 包含 C）

## 输出

- `df_merged`: 包含处理后的K线序列（无相邻包含）
- `merged_to_raw: list[tuple[int, int]]`: 每根 merged bar 对应原始K线的位置范围（闭区间）

`merged_to_raw` 是新笔间距计算的关键数据来源。

### 组锚与序号关系（SPEC #847 S9 / #817 N-1 裁定四，v1.4 补录）

- **合并后 bar 保留组内首根原始序号**：每根 merged bar 的原始序号锚 = 该合并组内**第一根**原始 K 的序号（Rust 实装 `rust/src/theta_v0/parser/inclusion.rs:23/:57` 的 `merge()` 保留 `acc.source_index`；教义 ADR `chanlun/escalate/adr-chain-confirmation-and-invariant-identity-20260722.md:19` 裁定 3：「合并组锚 = 组内首根序号，非极值发生根」）。这些锚**严格递增** ⟹ 第 i 组覆盖原始序号 `[src_i, src_{i+1})`，成员集合与双向映射可二分推出（数学上没丢信息）。
- **组锚的单一来源**：Rust 侧 `merged_group_anchor`（`inclusion.rs:171`）已是「原始序号 → 组锚」的统一入口（自称「单一来源、禁第二查法」，生产消费者在案）。此前缺的是「原始序号 → 组号（merged 下标）」一步——已由稠密标注补上。
- **极值根集合（#1343）**：每组的 `high_sources`/`low_sources` 是取到组极值的成员集合（`inclusion.rs:673-675/:690-702` 建组即以首根为元素、归并只替换不清空，故对已成组非空）。多根并列是事实不是等待态；`equal_extreme_identity` 只作证据标签，不得作为任何准入门。消费口径（raw 间隔取极值区）在 `bi.md` §7.1，本文件不重复定义（#799：同一判断只有一份）。

### 三形态载体关系（SPEC #847 S9，以 bit-exact 对拍为准）

同一件事（原始序号 ↔ 合并组）在仓内有三种形态，取代/并存/导出关系已定：

| 形态 | 位置 | 关系 |
|---|---|---|
| Python 稀疏区间表 `merged_to_raw` | `src/newchan/a_inclusion.py :: merge_inclusion` | **并存（待跨语言对拍）**：稀疏投影，产自 Python 自己的合并算法（初始方向用 close/open 推断、无开头方向扫描），与 Rust 侧**当前零对拍**（算法不同源）。 |
| Rust 稀疏锚点 + 二分 `merged_group_anchor` | `rust/src/theta_v0/parser/inclusion.rs:171` | **保留（同源）**：生产消费者在案。稠密标注由它同源的 `merged` 导出、同趟二分，不并存第二查法。 |
| 稠密标注（每根原始 K 挂组号 + 组标准化高低 + 已确认/暂定 + 组内逐根高低） | `rust/src/theta_v0/parser/inclusion.rs :: dense_annotation` | **Rust 侧正本载体（本票新增）**：与 `merged_group_anchor` 同源。bit-exact 验收门：由标注重建的合并序列与 `process_inclusion` 现役输出逐字段相同。 |

**取代/导出结论**：稠密标注是 Rust 侧正本载体；`merged_group_anchor` 保留并与它同源（读同一份 `merged`，不并存第二查法）；Python `merged_to_raw` 与 Rust 载体**当前零对拍**（算法不同源），登记为**并存待对拍**——跨语言对齐（Python 改读 Rust 载体或逐位对拍）不在 S9 本票范围，须单独立票。

## 未定前缀的输出与身份（#1342 裁定，`[新缠论:选择]`）

- **观察载体**：逐 raw 原件 + `direction_state=Undetermined` + cut / first_known；不输出 envelope（它不是任何已裁方向下的合并结果）。
- **身份变化**：方向出现后，原始身份永久保留；新建或修订的组对象显式 `replaces` 被替代的未定记录；旧 cut 不回填，旧 AsKnown 返回旧对象。组锚 = 组内首根 raw 序号，内部极值发生根另记（见上文「组锚与序号关系」）。
- **版本**：初始化政策版本、raw 修订、结构版本、object revision 分开记；只改政策也须新规则版本。
- **不在本票但已裁**：同价极值身份（RD-02 [#1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)，2026-10-14）——集合语义，见 `bi.md` §7.1。

## 未定前缀上 W 对象的发布与 `pending_since_cut`（[#1565](https://github.com/xy7365527-lang/NewChanlun/issues/1565) 裁定，2026-10-20，`[新缠论:选择]`）

本节补上节「观察载体」里 `cut / first_known` 的那一格：未定原件对象 `CC-004.undetermined_raw` 的 `pending_since_cut` 是什么、从哪来，以及 S3 生成态在不读、不写生命周期元数据的前提下怎样发布 `CC-004.initialization`（槽 W）与 `CC-004.undetermined_raw`。用户 2026-10-20 在 #1565 批复采纳方案 A。

### 定义

**`pending_since_cut`（未定发生 cut）**：对象携带的到达事实，记该 raw 原件**首次进入未定状态时所在的 cut**——确切说是本段连续未定期的起始 cut（见下「延续何时断开」）。它是该对象 payload 的一部分，进 typed object_id 的内容寻址；它不是生产者历史，也不是发布链的首获知元数据。

- **首次进入**：某 raw 原件在 cut c 首次以未定状态出现（上一代没有同一原件身份的活动未定对象），取 `pending_since_cut = c`。
- **逐代原样延续**：方向未建立期间，下一代若仍有同一原件身份的未定对象，则从上一代**已核活动对象的 payload** 原样抄入 `pending_since_cut`，不重新取值。原件身份 = 槽（`CC-004.undetermined_raw`, raw 序号）+ 输入引用（含 raw 修订）；与 W 源一致（`de73545cbf:rust/src/bin/s_session_v2/tb02a_facts.rs:168-176`）。
- **延续何时断开**：方向建立后，未定记录按上节由组对象 `replaces`，延续终止；raw 修订使输入引用变化，按新原件处理。此后同一序号再次进入未定状态，属新一次到达，取当时的 cut。旧对象与旧 AsKnown 不回填。
- **只读 payload，不读生命周期字段**：延续的输入只有上一代已核对象的 `payload.pending_since_cut`。`first_known_generation`、`first_known_cut`、`published_generation`、撤回代号等字段**不得**作为取值输入，也不得由生成态写入或抄入对象。

### 为何不从原始 K 线重算

一个 cut 可以包含多根 K 线：cut 按代编号（整合分支 `rust/src/bin/s_structure_session.rs:3349` `structure_cut = cut-{gen}`），一代推进把上次输入前沿之后的全部新 raw 事件一起读入，可以是多根。raw 行（序号、修订、seq、时间戳、回执等）不带任何代号或 cut 字段，没有「它在哪个 cut 第一次被看见」这项事实；而且修订可能让方向重新回到未定，进入未定的时刻本就不等于到达时刻；cut ↔ seq 的对应记在发布历史（清单、时钟、前沿）里，那正是生成态不碰的发布链。所以从当代 raw 序列只能重算「哪些原件处于未定」（`firstPair` 判据），算不出 `pending_since_cut`；要重算就得回头读发布历史，等于把冲突换个位置。到达事实只能在到达那一刻记下，随对象带走。

### 与 S3 生成态约束的关系与出处

**约束原文与出处（照实）**：「生成态不写生命周期元数据」**未见于 ADR、`CONTEXT.md` 或任何 `.chanlun/definitions/` 正本**（在 main `b63a3fd586` 的 `docs/adr`、`CONTEXT.md`、`.chanlun/definitions` 检索 `生命周期元数据`／`首获知`／`first_known`：只命中本文件上节 `cut / first_known` 与 `docs/adr/0037-s-content-bounded-layout.md:12` 按 `first_known_generation` 排快照，二者都不是该约束）。它目前只存在于两处：

1. 代码：`rust/src/bin/s_session_v3/generation.rs:16-17`（#1394 线 `d1441b5d77` 引入，[#1419](https://github.com/xy7365527-lang/NewChanlun/issues/1419) 整合分支 `356df5db72` 同行未变，**尚未进 main**）逐字：「本模块不发出生许可，不写任何生命周期元数据，也不把 `wit-`/`window-` 之外的任何 `*_id` 当发布身份」。
2. 其规范根据：#1394 采纳件 `.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:200`（同上两提交；采纳记录 `ADOPTION.json`，用户「照办 批准」，`recorded_at_utc` 2026-09-15T23:44Z§5.1–5.9）逐字：「生成态元数据与独立索引的首获知元数据继续分开」；`:201` 把「实际首获知／撤回元数据」划入 `publication_template`。该目录属工作草稿名分，不是正本层。另见 #1419 issuecomment-6265160238 待裁第 3 条、issuecomment-6265209957 主控裁定第 3 条。

**本裁定一并正式化（以 W 对象发布所需为限）**：S3 生成态的输入是同一读集的 raw 全修订史、当前生效账，以及上一代**已核发布内容**（与既有 `generation.rs:111` 读上一代已核 CC-010.bi / CC-056.version 记录根同一口径）。生成态**不读**首获知、发布代号、撤回等生命周期字段作为取值输入，**不写**这些字段；它们归 root 的独立出生／撤回索引。理由：生命周期字段是发布链对「何时被看见」的记账，若进入生成输入，生成结果就依赖发布历史，§5.4 的无循环派生与「独立索引独立核出生」都会失去意义。正式化只覆盖本节需要的这一面；S3 其余生成／发布边界仍以 #1394 采纳件为准，后续若立 ADR 收编，本节随之改为引用。

**为何方案 A 不违反该约束**：`pending_since_cut` 在 payload 里，延续读的是上一代已核内容，不是生命周期字段。由于 typed object_id = F({kind, fact_key, payload, input_refs, profile_id, rule_revision})（`APPROVED-SOURCE.md:171`），方向未建立且原件不变时，每代算出的未定对象内容身份**逐字相同**；root 的独立出生索引据此判为既有对象，首获知元数据由索引自己保持。生成态无需抄写 `first_known_*`（W 源 `tb02a_facts.rs:184-193` 的抄写在 S3 中归索引）。

### 与 W 原语义一致、无宽严两档

- **取值相同**：W 源 v2 写者取 `since = 上一代同身份对象的 first_known_cut`，首次取当代 cut（`de73545cbf:…/tb02a_facts.rs:174-176`）。按定义，该对象首次发布时 `pending_since_cut = first_known_cut = 当时的 cut`，此后两者逐代同值延续；合同侧对此有硬核验（整合分支 `s_session/s_tb02_contract.py:399`「未定发生 cut 与首次获知不同」即拒）。所以「读 payload.pending_since_cut」与「读 first_known_cut」在一切合法历史上输出逐位相同——这是同一个判断换了取值来源，不是第二套判据。
- **只有一条通道**：生成态只按本节取值；**不得**在 payload 缺失或不可核时退回读 `first_known_cut`，也不得改用当代 cut 补值。上一代对象不可核即按旧状态审计失败处理，不降级放行（AGENTS.md「收敛通则」#799）。合同 / 前端对 `pending_since_cut == first_known_cut` 的比对是发布后的核验，不参与生成取值。
- **与 Lean 正本**：`formal/Origin/RawInclusionInitialization.lean`（`de73545cbf`）只形式化初始化几何（`firstPair` / `initializeRaw`；其头注 `:8` 明写不覆盖 S 持久身份），本节不改其任一定义；`pending_since_cut` 属 S 持久身份层，Lean 侧无对应项，也无冲突。

### #1565 受影响代码清单（基线：#1419 整合分支 `356df5db72`；W 源 `de73545cbf`）

| 侧 | 文件:行 | 处置 | 归属 |
|---|---|---|---|
| Rust | `rust/src/bin/s_session_v2/tb02a_facts.rs:414/:461-469/:477-486`（`add_initialization`） | v2 写者：取值改读上一代同身份对象 `payload.pending_since_cut`（与现读 `first_known_cut` 输出逐位相同）；`:477-486` 抄 `first_known_*` 留在 v2（v2 写入口按 #1419 主控裁定第 2 条退役，只读 AsKnown 时不再执行） | #1419 |
| Rust | `rust/src/bin/s_structure_session.rs:3350-3351` | `previous_active` 作 W 延续输入：注释改为「读上一代已核对象 payload」 | #1419 |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:659/:982/:1002`（`Cc004Producer`、`build_step`、`build_step_object`） | S3 构造器：补发 `CC-004.initialization`（槽 W）与逐原件 `CC-004.undetermined_raw`，`pending_since_cut` 按本节取值；等待期不再把 waiting 步发成 `CC-004.inclusion_step`（W 移植审查 B2） | #1419 |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1379/:1416-1436`（`build_knowledge_state`） | 已带 W 合同域（`:1436` `W_CONTRACT.knowledge_domain`）与 `initialization_policy_revision`／`direction_state`（`:1420`/`:1424`，W 移植审查 B1 已处理），保持 | #1419 |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:4032/:4109/:6105-6106` | 三处「阻塞于 #1565」的永不建立样例（CC-004 记录对拍、CC-005 对拍、等待期对象数）解除阻塞；补 W 对象逐代 `pending_since_cut` 不变、与 v2 对拍、恢复（reopen/recover）前后逐字段相同 | #1419 |
| Rust | `rust/src/bin/s_session_v3/generation.rs:16-17/:111` | 约束注释加票号 #1565 指路；`prior` 增上一代已核活动 W 未定对象根（与 bi/version 同口径），只读 payload | #1419 |
| Rust | `rust/src/bin/s_session_v3/source_audit.rs:1715-1723/:1817-1822` | 来源审计 kind 白名单补两种 W kind（现为 CC-004 步／005／007／054、tb02b kinds 与 `:1723` `source_structure::is_kind`）；`:1818` 按 rule 取合同保持 | #1419 |
| Python / JS | `s_session/s_tb02_contract.py:192/:399`、`s_session/browser/q3-typed.js:126`、`s_session/browser/tb01c-client.js:2729` | 不改判据：保留为发布后核验，不作生成输入（`q3-typed.js` 缺 `cut-[1-9][0-9]*` 形式检查，见 W 移植审查 N2） | #1419 / [#1393](https://github.com/xy7365527-lang/NewChanlun/issues/1393) |
| Lean | `formal/Origin/RawInclusionInitialization.lean`（`de73545cbf`） | 不改 | — |

## #1342 受影响代码清单（基线 main `465174240b`）

| 侧 | 文件:行 | 处置 | 归属 |
|---|---|---|---|
| Rust | `rust/src/theta_v0/parser/inclusion.rs:41/:52/:80` | 不改：`contains`/`merge`/`strict_dir` 为已裁原语 | — |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:110/:129/:430` | 旧 batch / `IncrInclusion` raw 扫描与激活重折：收编为本规则的实现基础或写明旧用途；不得在失败时切回 | TB-02-U [#1393](https://github.com/xy7365527-lang/NewChanlun/issues/1393) |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:555/:563/:588/:603/:627/:761` | 方向证据、组成员/极值根、未定门：补 pending → direction-established 的完整分支 | #1393 |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:806/:821/:843/:920/:926` | 紧凑状态的永久 unsettled（`:843` 遇包含即停）→ 非包含对出现即解锁并自起点重折；恢复材料同源 | #1393 |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:163/:216` | 原始序号 ↔ 组锚、稠密标注：未定材料不伪造正式归组 | #1393 |
| Rust | `rust/src/theta_v0/parser/mod.rs:152/:168/:197/:328` | 正式装配与全量 facts 观察同源 | #1393 |
| Rust | `rust/src/theta_v0/parser/stroke.rs:180/:356/:608` | 初始化未定时不偷产端点或新笔成功 | #1393 |
| Rust | `rust/src/bin/s_session_v2/tb02a_facts.rs:113/:334` | 同源观察：未定对象、waiting 原因、`replaces`、AsKnown | #1393 |
| Lean | `formal/Origin/SegmentFeatureSeq.lean:143/:153`、`formal/Origin/SegmentAutoConstruct.lean:69` | 不改：二者是特征序列的 processUp，不是 raw 初始化 | — |
| Lean | （新文件） | 补 raw 初始化规则的具名定义与分支证明（空 / 单根 / 首对 / 全包含） | #1393 或 Lean 跟进票 |
| Python | `src/newchan/a_inclusion.py:23/:54/:90/:125/:138` | #1286 冻结对照；记录与本规则的差异，不升为生产重判 | — |

## 当前实现

- **文件**: `src/newchan/a_inclusion.py :: merge_inclusion`
- **方向判定**: 双条件
- **初始方向**: 见「合并方向」节 #1342 裁定（上列 Python 文件是 #1286 冻结的历史对照，其 close/open 推断不是正本）
- **输出**: `(df_merged, merged_to_raw)`

## 边界条件

- 空输入 → 返回空 DataFrame 和空列表
- 单根K线 → 原样返回，`merged_to_raw = [(0, 0)]`
- 相邻K线 high 或 low 恰好相等 → 视为包含（`>=`，非严格大于）
- 连续多根包含 → 持续合并到同一个 merged bar 中

## 未结算问题

1. **方向判定：单条件 vs 双条件**。原文（65课）用单条件，代码用双条件。数学结果相同但概念路径不同。需要决定：是否将代码改为单条件以贴合原文语义，还是保留双条件作为防御实现？
2. **等号问题**：原文用 `>=`/`<=`，代码用 `>`/`<`。编者注"应去掉等号"的来源和理由不明。`>=` 包含判定下等号场景不影响方向判定结果，但需要确认这个编者注是否有缠师本人的依据。
3. ~~**初始方向 None → UP 的合理性**。原文未明确给出初始方向的处理方式。默认 UP 是工程选择，尚无原文依据。~~ **⟹ 已闭（[#1342](https://github.com/xy7365527-lang/NewChanlun/issues/1342)，2026-10-13）**：改为等待首个 raw 非包含严格对，见「合并方向」节。

## 谱系引用

- 间接关联 001：包含处理的正确性影响分型识别 → 影响笔划分 → 影响退化段
- 间接关联新笔：`merged_to_raw` 是新笔间距计算的数据来源

## 影响声明

包含处理是整个管线的第一步。所有后续概念（分型、笔、线段、中枢、走势类型）建立在包含处理后的序列上。`merged_to_raw` 的正确性直接影响新笔间距计算。
