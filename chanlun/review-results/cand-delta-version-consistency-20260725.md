# `Cand^δ` 候选谓词 × 各 map 已落地改动：版本一致性辨析

- 日期：2026-07-25
- 工作面：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- **锚定 HEAD = `140a660944486c0825f4c58fcadefe6e6ab0e9ce`**（`docs(chanlun): #259 盘整背驰教义地位 + 消费路径双查`，2026-07-25 06:07 EDT）
  - 核查期间 HEAD 由 `a73d1938d0` 前进到 `140a660944`（他方 session 并发推进），本文全部行号/结论按 `140a660944` 复核过一遍。
  - `main` = `93abf6496a`（与本分支 merge-base = `7e6bf7c7ea`；本分支 ahead 6 / behind 44）。本文结论只覆盖本分支 HEAD。
- 性质：只读核查，未改任何代码/git 状态；唯一写入 = 本文件。

---

## 摘要（先结论）

**发现一处版本不一致，而且是全文最重的一条：**

> `Cand^δ` 的「趋势域 ∧ 一类点」双重窄化**只存在于旧的 `CandDeltaEvent` 路径上，而该路径不在生产链上**。
> 生产的 `N^δ` 链走 `NestCandidateEvent` 路径，既不读 `cand_delta`、也不按 `kind` 过滤 ——
> 盘整背驰事件（wf7 窗 840/878 = 95.7%）**全部作为基例进入链装配，并实际产出了 72/88 张证书**。

因此：

- Q1 的两处（P1 三类全放开 vs `cand_delta` 只认一类）**不是同一层**，也**不构成空转** —— 它们分属两条彼此独立的代码路径，P1 那条是生产的，`cand_delta` 那条是审计/诊断的。
- 真正的不一致不在代码之间，而在**代码 vs 当前正在流通的判断**：SPEC #258（OPEN，2026-07-25 重写）与 #259 codex 核查报告都把「旧路径的窄化」当成了生产链的窄化，并据此把 wf7 链稀薄（`no_chain` 92.3%）归因于 `Cand^δ` 域窄化。**这个归因与 HEAD 代码不符。**

---

## Q1：P1「Trend 域三类全放开」放开的是哪一层？

### 结论：证书**终端背书**层（点类合法性），不是候选产出层。

**改动落点：** commit `bbbd8f89fa`（2026-07-24 19:22，`feat(classifier): P1→#214→#218 证书索引口径三部曲`），文件 `rust/src/theta_v0/classifier/nest.rs`。
裁定文书：`chanlun/escalate/p1-resupercede-trend-all-three-types-20260721.md`（2026-07-21 编排者签字生效），其 §影响第一条自陈：

> ``terminal_bits_in_book` CWindow 分支 Trend 域移除 `(buy1||sell1)` 限定`

**当前代码（HEAD）：** `nest.rs:694-706`

```
let legal = match kind {
    NestDivergenceKind::Consolidation => true,
    NestDivergenceKind::Trend => {
        // P1 重议（2026-07-21 编排者批准）：Trend 域同向一/二/三类全合法
```

即：`terminal_bits_in_book_core`（`nest.rs:654`）在窗口 `[c_start, t*]` 内**遍历 BSP 账本**，判某个已存在的买卖点能否给这条链的终端背书。P1 放开的是「哪几类账本点算合法背书」。它既不产生候选，也不产生事件。

**`Cand^δ` 侧：** `recursive_tower.rs:2001` `level_cand_delta`，`:2149`

```
let cand_delta = pf.bits.buy1 || pf.bits.sell1;
```

以及 `:2088-2090` 非趋势块 `continue`。这是**事件产出层**：决定这一段是否成为一条 `CandDeltaEvent` 的合格基例。

### 两者的语义关系：不同层 + 不同路径，互不服务

|  | P1 放开的那层 | `cand_delta` 那层 |
|---|---|---|
| 对象 | `BspPoint`（账本里的买卖点） | `Segment`（塔上的段） |
| 角色 | 终端背书合法性（`Conf^δ_e` 的载体） | 候选谓词（`Cand^δ_ℓ`） |
| 事件类型 | 服务 `NestCandidateEvent` | 服务 `CandDeltaEvent` |
| 是否在生产链上 | **是** | **否** |

关键：**两者不在同一条事件流上**，所以「索引收三类但候选只产一类 ⟹ 三类索引永远收不到候选」这种空转**不成立**。

生产链的候选侧另有其人：`NestCandidateEvent`（`level_view.rs:480`），其头注写明：

> `Cand` 已由 provider 固定为 `dir ∧ Comparable ∧ Extreme`；`divergence_confirmed` 是独立的②力度层真值，不进入 Cand。

生产装配的实际门（**已逐行核对**）：

- 基例门 `nest.rs:855`：`if top_level < exec_level || !base.divergence_confirmed { return None; }` —— 无 `kind` 过滤、无 `cand_delta`。
- 攀升门 `nest.rs:933-940`：注释「#97 初筛只看结构（D1 裁定：Cand=纯结构宽候选…）rung 级不再要求 `divergence_confirmed`」，实际判据只有 `event.side != side` 与 `is_sub(child, &parent)`。
- 索引构建 `nest_index.rs:258-307`：`for exec in 1..events_by_level.len() { for base in &events_by_level[exec] { … } }` —— 全量遍历，唯一过滤是 exec 从 1 起扫（不听 L0）。
- 事件入账 `admission.rs:719`：`if !event.divergence_confirmed { continue; }` —— 同样无 `kind` 过滤。

而 provider 两个分支都在产事件：`level_view.rs:798` `kind: NestDivergenceKind::Trend`，`level_view.rs:872` `kind: NestDivergenceKind::Consolidation`（后者的入料口注释自陈「#97 进料口（⑤『盘背入链』落地缺口补齐）」）。

**实测坐实（#214 装置读数，`chanlun/review-results/endorsement-failure-instrument-readings-20260724.md:44-50`）：**

wf7 窗 `base(Trend, Pan)` = **(38, 840)**，`assembled(Trend, Pan)` = **(16, 72)**。

这两组数字由 `build_nest_certificate_index` 内部的 `instrument.observe_base` 产出（`nest_index.rs:281`），即 840 个盘整事件**确实作为基例被逐个考察过**，并且**其中 72 个成功装配出证书**——占全部 88 张 assembled 的 82%。

> ⟹ 盘整背驰不但进了生产 `N^δ` 链，还是链证书的主要来源。

---

## Q2：`level_cand_delta` 当前实现属于哪个时期？

`git blame -L 1993,2215 rust/src/theta_v0/classifier/recursive_tower.rs`（HEAD 口径）：

| commit | 日期 | 行数 | 说明 |
|---|---|---:|---|
| `b48e8d9b11` | 2026-07-09 | 117 | `feat(theta): P1 谓词闭包——per-level Cand^δ 背驰段谓词`（函数主体奠基） |
| `ba32291eff` | 2026-07-12 | 37 | `feat: P47 c_p 闭合生命周期 + P0 cand_delta 命名/双时点裁决落盘` —— **`:2149` `cand_delta = pf.bits.buy1 \|\| pf.bits.sell1` 归属此 commit** |
| `f681bbf681` | 2026-07-10 | 22 | `cert F-02: 盘整背驰诊断可达性修复`（非趋势门段独立产纯诊断事件） |
| `21f0296025` | 2026-07-12 | 15 | P50 证书实装 |
| `99ce554b8a` | 2026-07-10 | 15 | 闭合 D_parent 诊断与纯结构排序 |
| `640609071d` | 2026-07-19 | 13 | 收口提交（`a_seg_entry` / `judge_pan_div` 签名机械适配，非判据改动） |
| `490b450199` | 2026-07-10 | 4 | 拆除 `confirm_src≡interval.end` 绑定 |

**结论：**

- **`cand_delta` 置位语义最后一次实质修改 = `ba32291eff`，2026-07-12，票号 P47/P0**（裁定书 `chanlun/escalate/p0-cand-delta-naming-dualtime-ruling-20260712.md`）。
- 函数整体最后一次触碰 = `640609071d`（2026-07-19），13 行，**纯签名适配，判据零改动**。
- **它停留在 2026-07-12 的版本，早于下列全部改动**：
  - 2026-07-16 `nest-migration-ruling-20260716.md`（裁决⑤盘背入链 + D1「Cand=纯结构宽候选」）
  - 2026-07-21 P1 重议
  - 2026-07-24 `bbbd8f89fa`（P1/#214/#218）、`d2312352f9`（knot 合版含 T3/T4/T5a/T5b）
- 与 #126 线的 P1/owner 改动相比：**早 12 天，且从未跟随**。

---

## Q3：逐条检查「某层已按新裁定改、`Cand^δ` 仍是旧版」

前置事实（全文反复用到）：`level_cand_delta` / `CandDeltaEvent` 的**全部消费者**（HEAD 全仓 grep）：

| 消费者 | 位置 | 性质 |
|---|---|---|
| `opsem_dump.rs:56` | `cand_delta_tower_cached` | **env-gated**：`THETA_STRICT_NEST_SIDECAR=1` 才启（`opsem_dump.rs:135-139`），默认 no-op（`runner.rs:495-501`） |
| `strict_nest_check.rs` | bin | 审计校验二进制 |
| `p92/p107/p116/p123/p124*` | bin | 审计/重放二进制，头注均自陈「旧事件路径（`CandDeltaEvent`，P1 基线对账）」 |
| `cp_capability_smoke.rs:389` | bin | 能力冒烟 |
| `runner.rs:1010` | `#[cfg(test)]`（test mod 起于 `:907`） | 单测 |

生产 backtest 的 nest 链**没有一条**读 `cand_delta`。

| # | 改动 | 动了什么 | 是否隐含要求 `Cand^δ` 侧配合 | 实际有无配合 | 后果 |
|---|---|---|---|---|---|
| 1 | **P1 Trend 域三类全放开**（#126 / `bbbd8f89fa` / escalate `p1-resupercede-…-20260721.md`） | `nest.rs:694-706` 终端背书点类合法性：Trend 域从 `(buy1\|sell1) ∧ owner=B` 放宽为 `同向一/二/三类 ∧ owner=B` | **否**。作用对象是 `BspPoint` 账本，不是段候选；作用路径是 `NestCandidateEvent`，与 `CandDeltaEvent` 不共享任何数据结构 | 无 | **无影响**（对代码而言）。但见「不一致清单 ①」——它在**文档层**留下了一处口径分叉 |
| 2 | **owner 归属修正路线 B / 两族判同**（#218/#219，`bbbd8f89fa`） | `BspPoint.center` 由 `Option<Center>` 改为 `Option<OwnerRef>`（`signal.rs`，二类载 `Type1Anchor`、一/三类载 `Center`）；`nest.rs:698-745` 判同换两族精确等值 | **否**。`CandDeltaEvent` 不携带 owner 载体，也不做 owner 判同 | 无 | **无影响** |
| 3 | **证书索引口径三部曲**（`bbbd8f89fa`：P1→#214→#218） | 改 `bsp.rs`/`nest.rs`/`nest_index.rs`/`signal.rs`；`recursive_tower.rs` **一行未动**（`git show --stat` 已核） | 否 | 无 | **无影响** |
| 4 | **T3 严格链（#172）/ T4 旧桥退役（#173）/ T5a 方向退役（#207）/ T5b 出场迁链（#208）** | 均落在 `d2312352f9`（2026-07-24）内 `admission.rs`/`runner.rs`：`typed_lookup`/`typed_lookup_multi`/`by_end`/`has_bridge_key`/`seg_c_full` 删除，身份键改两元锚 `by_triple_anchor`（`admission.rs:729-737`），出场改 `chain_lookup` | **否**。这四项全部作用于 `NestCandidateEvent` 的身份键与桥；`CandDeltaEvent` **从来就没有身份桥** | 无 | **无影响** |
| 5 | **knot 合版**（`d2312352f9`，新增 `chain_driven_level_projection`） | `admission.rs:67` 新增；在 `nest_cert_gate_enabled() && !config.level_projection.enabled` 时派生 `level_projection` 配置（生产唯一写入点 `runner.rs:493, 665`） | 否（config 派生层） | 无 | **无影响**。但它使 2026-07-24 03:04 产出的 wf7 链 dump 过期 16 小时 —— 已由 `chain-attribution-caliber-check-20260725.md` 自行登记 |

---

## Q4：「裁决②」（盘整背驰不入链）的出处与效力

### 出处

- **文书**：`chanlun/escalate/strict-nesting-rulings-20260708.md`，「裁决 2：盘整背驰不入链」
- **日期**：2026-07-08
- **性质（原文自述）**：「**adopted-default 裁决——可复议**」，副标题「定义分叉的默认采用裁决——用户已授权自主推进…按调研唯一自洽候选取默认并落盘备查；任何一项被复议推翻则 P1-P4 相应产物重跑」
- **依据（原文）**：混合。教义引用 = 0027 区间套语境为趋势递归、0016:62 跨中枢 C/A 配对口径；工程理由 = 「盘整背驰入链会放宽必要门，属判据软化」。依据文件 = `strict-nesting-divergence-survey-20260708.md §B`
- **原文自留复议通道**：「诊断位保留复议通道。」

> ⚠️ **取证瑕疵（须登记）**：该文书在 **HEAD 与 main 的工作树中都不存在**。
> `git cat-file -e HEAD:chanlun/escalate/strict-nesting-rulings-20260708.md` → 不存在；main 同。
> 全仓唯一含有它的 ref = 分支 `gap3-rework-codex9-fix`（commit `4f94b3fdc2`，2026-07-10），**该 commit 不是 HEAD 的祖先**。
> 本文引用的原文由 `git show 4f94b3fdc2:…` 取得。这与 `chanlun/review-results/bughunt-verify-20260710.md:34` 记录的同一取证瑕疵一致。
> ⟹ **当前分支上，裁决②只以源码注释形式存在，没有在库的文书载体。**

### 是否被 supersede：**是，且走了正式文书**

- **supersede 文书**：`chanlun/escalate/nest-migration-ruling-20260716.md`（在库，HEAD 可读）
- **标题**：`裁定：严格 C2 pair 进入 N^δ 证书域（#91）`
- **状态行原文**：「**已裁定生效（2026-07-16，人裁口头批准：『可以 按你的来』= 1批 2codex 3codex 4是）**」，签字位 §五已勾选
- **裁决⑤原文**：

  > | ⑤ 背驰域 | **盘整背驰入链** | N^δ 链背驰段域 = 趋势 ∪ 盘整（027 整课 + 万科旗舰例顶层即盘背）；typed 分流保留，盘背证书不冒充同级 B1/S1（#145 承接门不动） |

- **同一文书 D1 另裁**：「**codex 不含**——Cand = 纯结构宽候选（dir ∧ Comparable ∧ Extreme），力度判据留在②」
  —— 这一条直接推翻了 `cand_delta = buy1||sell1`（把力度/背驰判据塞进 Cand）的做法，在新路径上已实装（`level_view.rs:480-485` 头注、`nest.rs:933-935` 注释均引 D1）。
- **落地缺口自陈**：该文书 §三附带发现已预告「⑤裁『盘背入链』后此处需要扩 provider，属实现缺口非裁定项」，随后由 #97 补齐（`level_view.rs:819-885` 盘整分支，注释「#97 进料口（⑤『盘背入链』落地缺口补齐）」）。

### 有无再被重议

- 2026-07-25 有两份新材料（HEAD 顶 commit `140a660944`）：`pan-divergence-doctrine-20260725.md`（教义侧：第27课「背驰段」= 背驰 ∪ 盘整背驰）与 `pan-divergence-consumers-20260725.md`（消费路径侧）。两份都是**研究/核查报告，非裁定文书**，按项目规则不构成 supersede 效力，方向上也是与裁决⑤同向而非反向。
- 未发现任何文书把裁决⑤再翻回裁决②。

**Q4 小结**：裁决②（2026-07-08，自主推进默认采用、原文自留复议通道）已于 **2026-07-16 由在库正式文书 + 人裁签字 supersede**，辖域为 `N^δ` 链背驰段域。旧路径 `recursive_tower.rs:2088/2149` 与 `nest.rs:1002` 的「裁决②」注释是 supersede 之前的遗留，**未随裁决⑤更新**。

---

## 不一致清单

### ① 【文档级冲突】`nest.rs:996-1009` 仍以旧路径宣称 `Cand^δ_ℓ` 的定义式

- **位置**：`rust/src/theta_v0/classifier/nest.rs:996-1009`（`d_parent_interval` 的文档注释）
- **原文**：

  ```
  /// `Cand^δ_ℓ` 定义式（P2 落定，补 spec 疑点2 的缺口；裁决① strict-nesting-rulings-20260708）：
  ///
  /// > **`Cand^δ_ℓ(x) ≔ 塔上 per-level 背驰段谓词`** = [`CandDeltaEvent::cand_delta`]
  /// - 裁决②：盘整背驰**不入谓词**（只出 [`CandDeltaEvent::pan_div_diag`] 诊断位，不参与本装配）。
  ```
- **性质**：**语义冲突（文档 vs 生效裁定）**。它以无限定语气宣称 `Cand^δ_ℓ` 的定义式，而
  (a) 该定义式在 2026-07-16 被 D1 裁为「不含力度判据」；
  (b) 裁决② 在同一天被裁决⑤ supersede；
  (c) 它引用的裁决书 `strict-nesting-rulings-20260708.md` 在本分支不在库。
- **可观察后果**：任何按注释找 `Cand^δ` 的读者会被导向旧路径。**这正是已经发生的事** —— 见 ③。

### ② 【文档级冲突】`recursive_tower.rs` 裁决②注释群

- **位置**：`:1198`（`pan_div_diag` 字段注释「裁决②：**不入谓词**」）、`:2085-2090`（「守住裁决②『不入链』边界」+「非趋势块 ⟹ 无第一类候选 ⟹ 无 `Cand^δ` 事件」）、`:2282` `:2317`（测试断言文案）
- **性质**：**局部为真、整体误导**。就 `CandDeltaEvent` 这条路径而言，注释描述的行为与代码一致（可执行断言也确实通过）；但它们不加限定地说「不入链」，而生产链早已收盘整背驰。
- **可观察后果**：同 ③。

### ③ 【判断级不一致 —— 本次核查最重的一条】SPEC #258 与 #259 核查把旧路径的窄化当成了生产链的窄化

- **#258**（OPEN，标题「📋 SPEC：`Cand^δ` 窄化是否正确」，正文标注「2026-07-25 重写」）正文原文：

  > 区间套背驰的递归判定 `N^δ` 是当前**唯一**的 nest 判定源（T3 #172 起，L2 旧臂只作对照读出）
  > `Cand^δ_ℓ` 在 rust 侧**有操作定义**：**`Cand^δ_ℓ` = 该级「趋势块」上的「第一类背驰段谓词」**
  > 约 96% 的事件在盘整域，而链只收趋势域的一类点
  > ⟹ **链稀薄的直接机制已经找到了**：…`Cand^δ` 的域窄化把 96% 的事件挡在链外

- **与 HEAD 代码对照的结果**：
  - 「链只收趋势域的一类点」——**否**。生产索引 `nest_index.rs:258-307` 全量遍历 `events_by_level`，无 kind 过滤；基例门 `nest.rs:855` 只查 `divergence_confirmed`；攀升门 `nest.rs:933-940` 只查 `side` 与 `is_sub`。
  - 「96% 的事件被挡在链外」——**否**，且用的正是同一批数字的反面：`endorsement-failure-instrument-readings-20260724.md:44-50` 显示 wf7 `base(Trend, Pan) = (38, 840)` —— 840 个盘整事件**全部进了基例考察**；`assembled(Trend, Pan) = (16, 72)` —— **72 个盘整基例成功产出证书**。
  - #258 引用的 38/840 这组数字，其来源装置 (`NestEndorsementInstrument`) 本身就架在 `build_nest_certificate_index` 内部（`nest_index.rs:281` `observe_base`）。**能被这个装置数到，就已经在链里了。**

- **#259 的 codex 核查报告**（`pan-divergence-consumers-20260725.md:37, 62`）同源：

  > **没有业务门读取它。** `N^δ` 装配只认 `cand_delta`：基例拒绝 `nest.rs:1135-1138`、攀升跳过 `:1198-1203`、批量基例过滤 `:1261-1269, 1289-1301`。
  > **但对 `N^δ` 区间套链而言，排除是彻底的**——`cand_delta=false` 在四处被过滤，无任何回旋。

  该报告引的四个位置**全部是 `CandDeltaEvent` 签名的旧函数**（已逐行核：`nest.rs:1126-1136` `assemble_certificate_*`、`:1195-1201` `extend_upward`、`:1255-1261` / `:1276-1289` 批量装配，四处入参均为 `&[Vec<CandDeltaEvent>]`）。它漏掉了 `assemble_typed_certificate`（`:844`）/ `extend_typed_upward`（`:908`）/ `build_nest_certificate_index`（`nest_index.rs:258`）这条 typed 生产链。

- **性质**：**归因错误**（不是代码 bug）。
- **可观察后果**：
  1. map #250「wf7 跨级链死活判定」的核心机制假设失效 —— 「链稀薄 = `Cand^δ` 域窄化挡住 96%」这条因果链在 HEAD 上不成立，`no_chain` 92.3% 必须另找机制。
  2. #258 的三步方案（提取 → 形式化 → 判定「趋势块 ∧ 一类点」是等价/真子集/不可比）**提取对象选错**：要形式化的应是生产路径的 `dir ∧ Comparable ∧ Extreme`（+ `divergence_confirmed` 基例门），不是旧路径的 `buy1||sell1`。按现方案做出来的 Lean parity fixture 会锁住一个不在生产上的谓词。
  3. #258 的 US-5「盘整背驰被排除这件事有形式化依据」在生产链上无对象可依据 —— 生产链没排除它。

### ④ 【死代码风险，非冲突】旧路径与新路径共存且旧路径口径已冻结在 2026-07-12

- **位置**：`recursive_tower.rs:2001-2215` + `classifier/mod.rs:526-694`（`cand_delta_tower` / `cand_delta_tower_cached` / `cand_delta_entry_tower`）
- **性质**：**空转（对生产而言）**。该路径全部消费者为审计 bin / env-gated sidecar / 单测（清单见 Q3 前置表）。
- **可观察后果**：`strict_nest_check` 等审计二进制报出的「Cand^δ / 区间套」读数与生产链读数**不同源**，不可互换引用。头注已在 `p92/p116/p123/p124` 四个 bin 里自陈「旧事件路径…P1 基线对账」，但 `recursive_tower.rs` 与 `nest.rs` 侧未做对称标注。

---

## 未能判定项

1. **旧 `CandDeltaEvent` 路径是否还应保留。** 本次只判定了它不在生产链上，没有判定它作为「P1 基线对账」的对照价值是否仍然成立（它的口径冻结在 2026-07-12，而对照的另一侧已经过 D1/⑤/P1 重议/#218/T3-T5b 数轮改动）。判定需要：一次当前 HEAD 口径下的双路径事件集对账。

2. **wf7 `no_chain` 92.3% 的真实机制。** 排除了「`Cand^δ` 域窄化」这一归因后，剩下的候选机制（`divergence_confirmed` 基例门的通过率、终端背书失败、`is_sub` 区间包含失败）本次未量化。判定需要：在当前 HEAD 口径下重跑 wf7 链 dump —— 这一步 `chain-attribution-caliber-check-20260725.md` 已经因另一个理由（`d2312352f9` 使 7-24 dump 过期 16 小时）要求过，两条理由指向同一个动作。

3. **`recursive_tower.rs:2146` 趋势分支内 `judge_pan_div` 调用是否可达。** 沿用 `pan-divergence-consumers-20260725.md:27` 登记的未判定项，本次未独立核查。判定需要：`(end_index, zd, zg)` 三元组在中枢序列上的全局唯一性不变量。

4. **裁决② 原始文书为何不在 HEAD/main 树内。** 只确认了它仅存在于 `gap3-rework-codex9-fix` 分支的 `4f94b3fdc2`，未查清是从未合入还是合入后被删。判定需要：`4f94b3fdc2` 与主线的分叉史追溯。

---

## 附：本文全部代码引用位置（HEAD = `140a660944`）

| 主张 | 位置 |
|---|---|
| `cand_delta` 只认一类 | `rust/src/theta_v0/classifier/recursive_tower.rs:2149` |
| 非趋势块 continue | `rust/src/theta_v0/classifier/recursive_tower.rs:2088-2090` |
| 盘整块只产诊断事件 | `rust/src/theta_v0/classifier/recursive_tower.rs:2078-2092` |
| P1 三类全放开 | `rust/src/theta_v0/classifier/nest.rs:694-706` |
| 旧路径 `Cand^δ` 定义式注释 | `rust/src/theta_v0/classifier/nest.rs:996-1009` |
| 旧路径基例门 | `rust/src/theta_v0/classifier/nest.rs:1136` |
| 旧路径攀升门 | `rust/src/theta_v0/classifier/nest.rs:1201` |
| 旧路径批量过滤 | `rust/src/theta_v0/classifier/nest.rs:1261, 1289` |
| **生产基例门（无 kind 过滤）** | `rust/src/theta_v0/classifier/nest.rs:855` |
| **生产攀升门（只 side + is_sub）** | `rust/src/theta_v0/classifier/nest.rs:933-940` |
| **生产索引全量遍历** | `rust/src/theta_v0/classifier/nest_index.rs:271-273` |
| `NestCandidateEvent` 定义 + Cand 语义 | `rust/src/theta_v0/classifier/level_view.rs:474-505` |
| provider 产 Trend 事件 | `rust/src/theta_v0/classifier/level_view.rs:798` |
| provider 产 Consolidation 事件 | `rust/src/theta_v0/classifier/level_view.rs:819-885`（`:872` kind） |
| 生产入账只过滤 `divergence_confirmed` | `rust/src/theta_v0/backtest/admission.rs:719` |
| 生产索引构建调用点 | `rust/src/theta_v0/backtest/admission.rs:774` |
| 旧路径唯一非-bin 消费者（env-gated） | `rust/src/theta_v0/backtest/opsem_dump.rs:56, 135-139` |
| env gate 生产接线 | `rust/src/theta_v0/backtest/runner.rs:495-501` |
| `chain_driven_level_projection` | `rust/src/theta_v0/backtest/admission.rs:67`；`runner.rs:493, 665` |
