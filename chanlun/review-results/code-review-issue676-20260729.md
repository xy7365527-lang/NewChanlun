# 双轴评审：#676 N3 链证书尾部硬化

**判定：CONDITIONAL**

- **对象**：`/tmp/wt-676` @ `ticket-676`，diff = `git diff 59c8eb1a79..HEAD`（5 commits：`cdb420b3c6` / `08aef9cb81` / `4886645ed0` / `bec0fa6de3` / `dba098149b`）
- **评审时点**：2026-07-29
- **评审人**：Opus 5（新上下文，与实施零瓜葛；本轮未派任何子代理，全前台自做）
- **轴**：Standards（AGENTS.md + `docs/agents/` 纪律族三文档 + 090 照实 / 判据零重写 / diff 无夹带 / 注释与代码一致 / Fowler）+ Spec（票面五条 Acceptance 逐条）
- **改动面**：5 文件 / +312 −76（`chanlun/review-results/issue641-chain-dualpath-divergence-20260729.md`、`rust/src/bin/issue550_event_battery.rs`、`rust/src/bin/p123_fast_replay.rs`、`rust/src/theta_v0/classifier/chain_cert/tests.rs`、`rust/src/theta_v0/classifier/mod.rs`）
- **评审期间对工作区的处置**：为做反事实负控，临时改过 4 个源文件；每次跑完按 sha256 逐字还原，收尾 `git status --short` 为空、`rust/src/bin/p123_fast_replay.rs` = `bdf08fc1…`、`rust/src/theta_v0/classifier/chain_cert/mod.rs` = `4c11c271…`。**未改任何文件的最终状态，未关 issue。**

---

## 一、判定摘要

票面五条 Acceptance **逐条实测通过，无一条靠采信 commit message**。判据零重写、diff 无夹带、四条 `ISSUE641_CHAIN*` 印出行经 20k 真实数据对拍复做确认逐字节不变。

但本票存在的目的就是"装锁"，而**两把锁经实测没有它们自称的分辨力**（MED-1、MED-2），另有一条现在时的"照实"声明指向本票自己删掉的符号（MED-3）。按仓内 090 纪律（声明=能力，不把读数升格成规律、不把非锁称作锁），这三条须在关票前处置或把措辞照实收窄——故 CONDITIONAL 而非 PASS。

值得单独记一笔的**正面事实**：#676-5 这一刀（把 40 段夹具上的"terminal_projection ⊆ causal"当场证伪、降格为双向差集 golden 锚、把硬锁收窄到唯一站得住的不变量、根因另开 #681）是本 diff 里质量最高的一段，正是 090 想要的形状——发现自己上一轮把巧合写成了断言，就照实撤回并登记，不悄悄放宽。

| 轴 | 结论 |
|---|---|
| Standards | 通过为主：判据零重写、无夹带、印出逐字节实测不变、rustfmt 零新增漂移（27→27）、Fowler #2 重复真消除。扣分集中在"锁的强度"——两处手写等式/单值夹具在扩展或病态实现下静默放宽（MED-1/MED-2/LOW-1/LOW-2/LOW-4），一处改名未扫跨模块引用（MED-3）。 |
| Spec | 五条 Acceptance 全部满足且可独立复现；AC-4 满足的是字面（"机器锁落地"=测试存在并断言结论），不是实质（该测试对自命的失效模式零分辨力）。 |

---

## 二、逐条实测（票面 Acceptance）

| AC | 票面要求 | 我的独立实测 | 判 |
|---|---|---|---|
| 1 | p123 节拍测试在「永不推进」与「每根都推进」两种病态实现下分别变红 | 抽查复做「每根都推进」（删 `p123_fast_replay.rs:611-613` 整道门）⟹ 变红，`left: [0, 1, 2, 3, 4, 5, 6]` / `right: [2, 5, 6]`，与 commit message 照抄的读数**逐字一致** | PASS |
| 2 | battery 拆分后 `ISSUE641_CHAIN*` 印出行与取态口径逐字节不变（20k 对拍） | ①四条印出块 48 行文本 base↔HEAD `diff` 为空；②真做 20k 对拍：`cargo run --release --bin issue550_event_battery -- btc_1m_full.json 20000 \| grep ISSUE641`，base 版与 HEAD 版 `diff` **为空（0 行）**；③四行读数与 commit 照实登记逐字一致，含 `digest=598662469262944227` | PASS |
| 3 | helper 合并后双路径 + superset 两测试零改动全绿 | 两测试全绿（全量 lib 内）；`git show 4886645ed0` 确认测试体只动了 helper 调用行，断言与夹具零改动；原 `cache` 变量在调用方被整体删除 ⟹ 编译即证明其后无使用，行为保持 | PASS（措辞见 INFO-2） |
| 4 | INFO-1 机器锁落地（Absent + 曾 Confirmed ⟹ 不 Closed 不 Invalidated） | 测试存在、三轮构造成立、前提非摆设（`nodes[0].state == Some(Confirmed)` 自锁）、`absent_node > 0` 探针非真空。但**分辨力实测为零**——见 MED-1 | 字面 PASS / 实质见 MED-1 |
| 5 | lib 计数零新增红（基线 2533/0/138） | `cargo test --release --lib` = **2534 passed / 0 failed / 138 ignored**；diff 新增 `#[test]` 恰 1 个 ⟹ 2533+1=2534 对上，零新增红 | PASS |
| 附 | p123 bin 12/0 照实复跑 | `cargo test --release --bin p123_fast_replay` = **12 passed / 0 failed / 0 ignored** | PASS |

---

## 三、逐项独立核验（评审令 1–6）

### 项 1 — p123 切片

**探针生产零成本**：✅ 成立。字段（`rust/src/bin/p123_fast_replay.rs:551-552`）、初始化（`:562-563`）、写入（`:614-615`）三处全在 `#[cfg(test)]` 下；`cargo build --release --bin p123_fast_replay`（非 test 编译）exit=0。生产面无该字段、无该 push。

**[2,5,6] 是否唯一编码节拍语义**：部分成立。`every=3`、`0..=6`、`is_last at 6` ⟹ 节拍支放行 2/5，末根支放行 6（`(6+1)%3≠0` ⟹ 6 只可能来自末根支）。我脑内构造并**实测**了 5 种病态实现：

| 病态实现 | 探针序列 | 结果 |
|---|---|---|
| 永不推进（门后即 return） | `[]` | 红（commit 声称） |
| 每根都推进（删整道门） | `[0,1,2,3,4,5,6]` | **红（我复做，读数逐字一致）** |
| 相位错一位（`as_of % every`） | `[0,3,6]` | 红（commit 声称） |
| **门里 `self.every` 硬编码成 `3`** | `[2,5,6]` | **绿 — 漏网（我实测，12/0 全绿）** → LOW-2 |
| **保留门与探针，删掉 `book.advance` 整条** | `[2,5,6]` | **绿 — 漏网（我实测，12/0 全绿）** → LOW-1 |

### 项 2 — battery 拆分

- **四条印出格式串与实参逐字未动**：✅ 实测。`git show 59c8eb1a79:…` 取 base 版，取两侧 println 块（base `179-226` / HEAD `219-266`，各 48 行）`diff` → 空。`git diff … \| grep ISSUE641_CHAIN` 里 println 内容行全为上下文行（无 `+`/`-` 前缀）。
- **20k 对拍**：✅ 复做，diff 为空（见 AC-2）。
- **取态顺序约束是否只活在一处**：✅ 结构上成立。`summarize()`（`issue550_event_battery.rs:181`）与 `digest()`（`:186`）在全部 bin 中仅此两处；`ChainReadout`（`:152-163`）**不携带 `book`** ⟹ `print_chain_readout`（`:205`）在类型上不可能改簿或重取。约束本体仍是 `settle_chain_readout` 内的语句顺序 + 注释，无机器锁（INFO-1）。
- **命令查询混用已消除**：✅ 名 `print_*` 不再 `advance`；原 ~80 行混职责函数拆成 35 行取态 + 48 行纯格式化。Fowler「Long Function」「Divergent Change」两条改善真实。

### 项 3 — helper 合一

- **PrefixCacheReuse 两值是否真的唯一变量**：✅ 成立。合一函数（`rust/src/theta_v0/classifier/mod.rs:4423-4447`）内 `reuse` 只控一件事：循环顶是否 `cache = TowerCache::new()`（`:4432-4435`）。其余（前缀序列、`ParseLayer` 构造、`as_of = end`、推进次数与顺序）逐字同路。三个调用点（`:4645` / `:4652` / `:4718`）两变体各有覆盖。
- **两测试零改动全绿**：✅ 全量 lib 内绿；断言/夹具零改动（见 AC-3、INFO-2）。
- 行为保持性有编译级证明：原本由调用方持有的 `cache` 变量在两个调用点被整体删除，若其后仍有使用则编译红。

### 项 4 — Absent × 曾 Confirmed 机器锁

三轮构造真覆盖了目标状态（①`Open` 基线 → ②头 `Confirmed` 且 `nodes[0].state` 自锁非摆设 → ③头 `Absent` ⟹ `Open` / `invalidation_cause == None` / `absent_node > 0`）。但**该测试对它自命的失效模式没有分辨力**——详见 MED-1（含两级注入实测）。

### 项 5 — superset 收窄（重点）

**豁免面是否恰好=候选全集计数全集**：✅ 成立。`SkippedLevel`（`chain_cert/mod.rs:168-174`）恰 3 个字段：`level` / `alive_at_level`（"该级别的存活候选总数"）/ `inside_parent`（"其中区间被父端点包含者的个数"）。比对函数（`classifier/mod.rs:4598`）只比 `level`，豁免面 = 后两枚，**正是且仅是候选全集上的两个计数**。豁免理由（候选全集由驱动决定：因果簿 append-only 累积 vs 终态投影每步 fresh）与两字段的 doc 定义严格对应，站得住。

**未豁免字段清单是否足以咬住链身份/生命史/边拓扑分叉**：✅ 在 HEAD 这一时点完备，无一字段被悄悄漏比。逐字段点数核对：

| 类型 | 字段总数 | 参与比对 | 豁免 |
|---|---|---|---|
| `TowerChainCertificate` | 15 | 15（`edges` 转交边比对 + 其余 14 直比） | 0 |
| `ChainEdge` | 7 | 7（`skipped_levels` 转交 + 其余 6 直比） | 0 |
| `SkippedLevel` | 3 | 1（`level`） | 2（`alive_at_level` / `inside_parent`，有理由） |

即：链身份（`key`/`extends`）、生命史（`observed_at`/`closed_at`/`invalidated_at`/`invalidation_cause`/`revision`/`supersedes_revision`/`revision_at`）、节点全史（`nodes` 的 `Vec` 整体等值，含每节点 `key`/`level`/`status`/`state`）、边拓扑（`parent`/`child`/`kind`/`crossed_nodes`/`predicate_holds`/`breach` + 被跳级别的 `level` 序列与长度）全部在比对面内。

**doc 里 40→120 的实测读数复跑**：✅ 全部复现。

| 读数 | doc 声称 | 我实测 |
|---|---|---|
| `cargo test --release --lib causal_and_terminal` | 绿 | **1 passed / 0 failed** |
| causal / terminal / 共有 | 38 / 31 / 20 | 绿断言即 `(38, 31, 20)` 通过 ✅ |
| `causal_only` / `terminal_only` | 18 / 11 | 绿断言即 18 / 11 通过 ✅ |
| 共有 20 key 中证书分叉数 | 4 | **撤销豁免（改 `a.skipped_levels == b.skipped_levels`）⟹ 变红，顶层 tuple 恰 4 对** ✅ |
| 4 分叉全在豁免字段 | 是 | **变红输出里差异全为 `SkippedLevel { level: 1, alive_at_level: 6/5, inside_parent: 2/1 }`——`level` 两侧相同，仅两枚计数不同** ✅ |

"其余全部字段逐一相同"这条由逻辑补集确证：HEAD 版（除两枚计数外全比）绿 ⟹ 其余字段全同；严格版（连两枚计数一起比）红 ⟹ 分叉恰在那两枚。

**#681 存在性**：✅ `[research] 终态投影驱动 terminal-only 链身份根因…（120 段实测 11 条）`，OPEN，parent #529。归因外挂而非在本票强行下结论，符合 090。

### 项 6 — 测试门

| 命令 | 实测 |
|---|---|
| `cargo test --release --lib` | **2534 passed / 0 failed / 138 ignored** |
| `cargo test --release --bin p123_fast_replay` | **12 passed / 0 failed / 0 ignored** |
| `cargo build --release --bin p123_fast_replay`（生产面） | exit=0 |
| `cargo run --release --bin issue550_event_battery -- … 20000` | exit=0，四行读数与 commit 逐字一致 |
| `rustfmt --check --edition 2021`（各改动文件自身 `Diff in` 计数） | `classifier/mod.rs` base 27 → HEAD 27；`p123_fast_replay.rs` 20（commit 声称 20）；`chain_cert/tests.rs` 0；`issue550_event_battery.rs` 0 ⟹ **零新增漂移，commit 的数字精确可复现** |

---

## 四、Findings（H/M/L）

### MED-1（Standards / 项 4）— `#676-4` 的"机器锁"对其自命的失效模式零分辨力，doc 归因也不成立

**锚**：`rust/src/theta_v0/classifier/chain_cert/tests.rs:467-521`（doc `:468`，断言 `:509-514`）

doc 与测试名称把它立为「Absent 链头即便曾 Confirmed 也不得误判 Closed」的**机器锁**，并给出归因「状态只读当前这一轮的事件视图（status/state 同源），不带上一轮的记忆」。两级注入实测：

**注入 A** — 在 `chain_cert/mod.rs:599` 把 `head_confirmed` 改为
`nodes[0].state == Some(Confirmed) || nodes[0].status == ChainNodeStatus::Absent`
（这正是"曾确认残留"这一失效模式的直接物化：Absent 的头被当成 Confirmed）：

> `cargo test --release --lib` = **2534 passed / 0 failed / 138 ignored — 全 lib 无一条变红**，新测试与既有 `absent_head_stays_open_and_is_not_invalidated` 均绿。

**注入 A + B** — 再叠加去掉地板条款（`:615` 的 `&& has_segment`）：

> 两条测试**同时**变红，`left: Closed / right: Open`。

⟹ 真正把关的是**地板条款**，不是 doc 声称的「status/state 同源」：链头 `Absent` ⟹ 存活节点 <2 ⟹ `alive_positions.windows(2)` 空 ⟹ 零边 ⟹ `has_segment == false` ⟹ Closed 支不可达。新测试相对既有测试的**增量分辨力为零**（任何能让新测试红的破坏都同时让老测试红）。

后果不只是命名问题：doc 把因果指错了对象，日后若有人重构地板条款（它在别处有独立的监视格与理由），这条"锁"不会响。

**缓解（照实登记）**：commit message 自己写了「一次通过，未触发任何断言失败」，票面 INFO-1 也自承「当前实装结构上不可能违反」——这不是隐瞒，是**live code doc 的用词与归因超出了实际见证面**，恰是 090「声明=能力」针对的形状。

**建议**（任一）：(a) doc 改照实——「覆盖补全用例，非分辨性锁；Closed 支的实际封堵者是地板条款 `has_segment`，见 `mod.rs:602/615`」，并从名义上去掉"机器锁"；(b) 换成 3 节点链夹具，使头 Absent 后仍有 ≥2 存活节点与一条判过的边（`has_segment == true`），届时 `head_confirmed` 单独就是唯一封堵者，锁才真成立。

### MED-2（Standards / 项 5）— 硬锁的字段完备性只靠手写等式，新增字段被静默豁免

**锚**：`rust/src/theta_v0/classifier/mod.rs:4578-4612`（`certificates_agree_ignoring_alive_candidate_universe_counts`）

三层比对（证书 15 字段、边 7 字段、被跳级别 1/3 字段）全部是手写 `a.x == b.x` 合取链，无解构。实测：

1. 给 `SkippedLevel` 加第四字段 `review_probe_field: u32` ⟹ 编译错误**只有 3 条 E0063，全在 `chain_cert/tests.rs:160/165/180` 的构造点**，比对函数**零错误**。
2. 补齐构造点、并让新字段取**已知两驱动分叉的量**（`alive_at_level`）⟹ `causal_and_terminal_projection_drives_agree_on_common_chain_keys` **仍然全绿**。

⟹ doc 里「除这两个数字外……逐一参与比对，不豁免」在 HEAD 为真，但**不是机器保证**：任何后加字段（包括真会分叉的字段）自动进豁免面，硬锁静默放宽。

**建议**：改用解构开头，新增字段即编译红——
`let TowerChainCertificate { key, extends, root_level, …, revision_at } = causal;`
同一 diff 的 `print_chain_readout`（`issue550_event_battery.rs:205-212`）已经在用这招，作者知道这个手法，此处未用是不一致。

### MED-3（Standards / 项 2 附带）— `#676-2` 改名未扫跨模块引用，一条现在时"照实"声明指向已删符号

**锚**：`rust/src/theta_v0/classifier/chain_cert/mod.rs:754`

> `/// **同源范围（照实，#641 S-2 收窄）**：只有 `issue550_event_battery::print_chain_summary` 真迁到了本结构体（`summarize()` + `digest()`）。`

`print_chain_summary` 已由本票 `08aef9cb81` 删除，现为 `settle_chain_readout`（`issue550_event_battery.rs:171`）。这是一条**现在时**的照实声明指向不存在的符号，属"注释与代码一致"直接违反。

同文件 `:751` 的 `print_chain_summary` 是过去时叙述（"此前散在两个诊断 bin 里各写一遍"），保留正确，不必改。全仓其余命中均在 `chanlun/review-results/*.md` 历史评审报告内，属历史记录，正确保留。

**建议**：`:754` 改为 `issue550_event_battery::settle_chain_readout`（可附「原 `print_chain_summary`，#676-2 拆分后改名」以保留可追溯）。

### LOW-1（Standards / 项 1）— 探针见证的是"过门"而非"推进"，doc 与测试名超出见证面

**锚**：`rust/src/bin/p123_fast_replay.rs:545`（doc「每次**真推进**」）、`:614-616`（探针在 `advance` **之前**）、`:4267`（测试名 `..._advances_on_beat_and_on_last_bar`）

实测：把 `:616` 的 `let delta = self.book.advance(&cache.candidate_streams(), as_of);` 整条替换为空 `Vec`（即 dump 开灯路径上链簿**从不推进**），保留门与探针 ⟹ **12 passed / 0 failed 全绿**。

原因：探针记录点在 `advance` 之前，且 bin 内唯一验证行格式的 `chain_dump_meta_and_line_format_are_stable`（`:4289`）绕开 `observe`、直调 `chain_dump_line`（`:4339-4340`）并手写 sink。⟹ `observe` 的推进臂在整个 bin 里**没有任何非空事件流覆盖**。

**缓解**：票面 LOW-2 明确给了「加节拍探针」与「换非空 cache 夹具」两条并列实修路，AC-1 只要求区分两种病态实现，已满足。这条是残留缺口，不是违约。

**建议**：doc 里「每次**真推进**」改「每次越过节拍门」（照实即可，代码本身没问题），或补一条用 `chain_dump_meta_and_line_format_are_stable` 那套非空 `CandidateEventBook` 夹具走一遍 `observe` 的用例。

### LOW-2（Standards / 项 1）— 单一 `every` 值 ⟹ `every` 参数未被证明真的驱动节拍门

**锚**：`rust/src/bin/p123_fast_replay.rs:611`、`:4270`（唯一夹具 `every=3`）

实测：把门里 `self.every` 硬编码成字面量 `3` ⟹ **12 passed / 0 failed 全绿**。测试锁住了节拍**形状**，没锁住"节拍由参数决定"。

**建议**：加第二组（如 `every=2`、`0..=6`、末根 6 ⟹ `[1, 3, 5, 6]`），一行即闭。

### LOW-3（Standards / 项 5）— 断言口径措辞丢了精度限定

**锚**：`rust/src/theta_v0/classifier/mod.rs:4671`

旧 msg：「共有 key 的**最新 revision** 逐字段分叉」→ 新 msg：「共有 key 的**证书**在候选全集计数以外的字段分叉」。比对面实为 `heads()`（每 key 最新 revision），未覆盖 revision 全史。措辞回填「最新 revision」即可，比对面本身无问题（全史同驱动一侧由 `chain_certificate_book_incremental_equals_full_replay` 另锁）。

### LOW-4（Standards / 项 3）— 两值 enum 用 `==` 而非 `match`，扩展时静默落入 shared 支

**锚**：`rust/src/theta_v0/classifier/mod.rs:4432`

`if reuse == PrefixCacheReuse::FreshPerPrefix { … }`。两档时无差别；加第三档会静默走 shared 分支，不编译红。与 MED-2 是同一「扩展时静默放宽」形态，在同一个 diff 里出现两次。类型为测试局部、当前仅两档，故 LOW。

### INFO-1（项 2）— 取态顺序约束仍是注释守，battery bin 零单测

`summarize`/`digest` 必须取在幂等 `advance` 之前（#653 LOW-1 / #641 S-3）这条约束，拆分后从 80 行印出函数**局部化**到 35 行取态函数（真改善，也是票面 LOW-3 的原意），但仍无机器锁；battery bin 无单测（commit 自承 `0 passed / 0 failed`），护栏只有真实数据对拍。属既有状态，票面未要求，登记不扣分。

### INFO-2（项 3）— commit message「两测试零改动」措辞

`4886645ed0` 的「双路径 + superset 两测试零改动全绿」：测试体内的 helper 调用行确有改动（helper 合一必然如此），断言与夹具零改动。宜写「断言/夹具零改动」。不影响 AC-3 判定。

### INFO-3（项 2 / Fowler）— `print_chain_readout(bars: &[Bar], …)` 只用 `bars.len()`

**锚**：`rust/src/bin/issue550_event_battery.rs:205`、`:226`。传整切片只为取一个 `len` 是 Data Clump 边缘；但改签名会动 println 实参 `bars.len()`，与「四行逐字节不变」这条硬约束冲突。照实保留是对的，不建议改。

---

## 五、纪律族逐条对表

| 条 | 判 | 依据 |
|---|---|---|
| 判据零重写 | ✅ | 无任何区间不等式/谓词被 bin 侧或测试侧复算；`settle_chain_readout` 仍只调库内 `summarize()`/`digest()`；比对 helper 只读字段不判真值 |
| diff 无夹带 | ✅ | 5 文件全在票面五件范围内；`.md` 改动是 #676-5 的报告锚订正，属票面第 5 件的必要产物 |
| 注释与代码一致 | ❌ | MED-3（`chain_cert/mod.rs:754` 指已删符号）；MED-1（归因指错对象）；LOW-1/LOW-3（措辞超出见证面） |
| 090 照实（声明=能力） | ⚠️ | 正面：#676-5 主动撤回自己上一轮的假规律、双向差集只作 golden 锚不升格为规律、根因外挂 #681——教科书级。负面：MED-1 把非锁称作"机器锁"，MED-2 的"不豁免"承诺无机器载体 |
| Fowler #2（重复代码） | ✅ | 18 行逐字重复真消除，唯一变量由类型自证 |
| Fowler Long Function / 命令查询混用 | ✅ | 票面 LOW-3 已闭（名 `print_*` 不再改簿；`ChainReadout` 不携 `book` ⟹ 类型级保证） |
| 交付纪律·开票门（对照物钉住） | ✅ | 票面五件均附了文件/测试名指针；AC-5 附了基线数 2533/0/138 |
| 交付纪律·关票门第 4 子句（逐 commit 可编译） | ⚠️ **未验** | 见下方 090 限度 |

---

## 六、090 限度（本评审**没有**核到什么）

1. **逐 commit 可编译性未验**。我只在两个纯态点（`59c8eb1a79` base 与 `HEAD`）真编译过；中间三个 commit 未建。跨点悬空标识符 `git grep` 显示各点自洽（`chain_book_over_prefixes_fresh_cache` / `print_chain_summary` / 旧 superset 测试名的命中数在各点与其应有状态一致），但**这不等于编译通过**。关票门第 4 子句需实施侧或关票人另行落证。
2. **只做了 20k 窗口**。500k / 全量窗口未跑（实施报告称 500k 逐 bar 因果重放会把电池推过 10 分钟）。「印出逐字节不变」这条我只在 20k 上证实。
3. **`#676-4` doc 里「被 resident 机制带入下一轮重评」这条说法未独立追证**。我只验了断言分辨力（MED-1），没去读 resident 路径确认第②轮的 `Open` 确实经由该机制。
4. **`terminal_only = 11` 的根因未查**（票面明确另开 #681，不在本 diff 分辨力内）。我只确认了计数可复现、#681 真实存在且挂在 #529 下。
5. **MED-2 的注入只覆盖了 `SkippedLevel` 一层**。`ChainEdge` 与 `TowerChainCertificate` 的新增字段静默豁免是同构推论（同为手写等式合取链），未逐层实测。
6. **未做异质源二审**。本评审为单一模型（Opus 5）逐项实测，无 codex/Gemini 交叉。
7. **反事实注入的全部改动均已还原**，收尾 `git status --short` 为空、两个关键文件 sha256 与备份一致。若后续发现工作区异常，备份留在 `/tmp/p123_backup.rs`、`/tmp/cc_mod_backup.rs`、`/tmp/cls_mod_backup.rs`、`/tmp/bat_head.rs`。

---

## 七、关票前建议处置（按优先序）

1. **MED-3**（一行）：`chain_cert/mod.rs:754` 的符号名改 `settle_chain_readout`。
2. **MED-1**（措辞或夹具）：doc 归因照实收窄，或换 3 节点夹具让锁真成立。二者取一即可，但**不宜原样关票**——live doc 里"机器锁"三个字目前无对应能力。
3. **MED-2**（一处解构）：比对 helper 首行改解构，把"不豁免"承诺变成编译期事实。
4. LOW-1 / LOW-2 / LOW-3 / LOW-4 可并入下一张尾部票，不阻塞本票。
5. 关票时按交付纪律补「逐 commit 可编译性」声明（本评审未覆盖，见 090 限度第 1 条）。
