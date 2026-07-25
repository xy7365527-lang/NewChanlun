# V2 N2 rung 级力度门实装卡（typed 装配路径·每级 rung 须为背驰段）

- **日期**：2026-07-20　**工位**：设计文档工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `640609071d`）
- **性质**：**只设计不实装**——本文件是唯一产出；`rust/src` 零改动、零 cargo、零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 全程只读。
- **引用格式**：代码锚 = `rust/src/**.rs:行号`（本工位 HEAD 树内行号，逐处亲读）；教义锚 = `docs/chanlun/text/blog/0XX-第X课.md:行号`（027/061/043/044 承重行本工位亲读）；文档锚 = `chanlun/review-results/*.md §节/:行号`；commit 锚 = 短哈希。

---

## 0. 结论速览

| 必答 | 一句话结论 |
|---|---|
| ①现状 | `extend_typed_upward`（`nest.rs:692-756`）rung 级只门 `side ∧ is_sub`（:720-726），`divergence_confirmed` 只进 sidecar 不参与否决（:731-733）——「背驰段的背驰段」降级为「包含段的包含段」（`doctrine-vs-code-nest-recursion-20260720.md:149-151` G-2）。 |
| ②谓词支系 | **按事件 kind 分域、同字段消费**：Trend rung → `trend_confirm_time` 全合取（T4∧T3∧T2∧T5-OR，`level_view.rs:542-633`）；Consolidation rung → `segments_diverge_or` 力度或关系（027:32，`divergence.rs:334-349`）。二者已**单源物化**于 `NestCandidateEvent.divergence_confirmed`（`level_view.rs:487`），rung 门 = **读字段**，nest 层零重算。 |
| ②复用机器 | provider `provide_nest_candidate_events` + `view.confirm_times` 缓存（wave-1 方案 1 单源化，`level_view.rs:947-949`、:1016-1036、:698-702）——力度真值只有一台机器产出，装配层只读，**不 fork**。p118 关④ 已把该真值送到断链点（`nest.rs:733`），本卡把它从 sidecar 提为门。 |
| ③实装量 | 生产改动 = `extend_typed_upward` 候选过滤加 1 个合取项（`!event.divergence_confirmed → continue`）+ 2 处注释改写（#97 D1 注释 :716-719、`assemble_typed_certificate` 头注 :613-617）+ 1 条 debug 不变量；`n_delta_rec`/`is_sub`/`sel_order`/provider/终端背书/chi_bool **零改**。 |
| ③主要代价 | 多级链收缩：凡含 `confirmed=false` rung 的链消失或重路由到更浅 top；`XiaozhuandaCandidate` 类空（`turn_class.rs:82`）——教义上**自洽**（`doc-level-domain-20260717.md:140`：小转大本来就不产生多级链），但与 p118 关④ 已 ship 的编码前提（`p118-xiaozhuanda-branch-design-20260718.md:42`）冲突，列为裁定项 **R-2**，本卡不自决。 |

---

## 1. 任务输入与教义锚（亲读）

### 1.1 教义承重行（本工位亲读 `docs/chanlun/text/blog/`）

- **027:22**（背驰段定义）：「在某级别的某类型走势，如果构成背驰或盘整背驰，就把这段走势类型称为某级别的背驰段。」——背驰段的身份 = 构成背驰的走势类型，力度确认是定义性成分，不是附加属性。
- **027:38**（万科例）：「……在月线上，可以找到针对月线最后中枢的背驰段，而这背驰段，**一定在季度线的背驰段里，而且区间比之小**……」——子背驰段严格在父背驰段内；被套对象双方都是背驰段。
- **027:44/027:46**（程序定理）：「某大级别的转折点，**可以**通过不同级别背驰段的逐级收缩范围而确定。」「先找到其背驰段，然后在次级别图里，**找出相应背驰段在次级别里的背驰段**，将该过程反复进行下去……」——递降程序的每一级对象都是背驰段。
- **061:32/061:34**（四重标准图解，亲读）：「65开始的走势是第一重背驰段，69开始的是第二重背驰段，**也就是65开始背驰段的背驰段**……也就是第四重的背驰段出现了。」「72点这个背驰点的精确定位，是由65开始背驰段的背驰段的背驰段的背驰段构成的……」——四重逐重皆背驰段，无「只包含不背驰」的中间重。
- **043:34/044:16**（情况二，亲读）：「明确显示没有出现30分钟的背驰，也就是背驰段最终不成立，但却出现一个1分钟级别的背驰」「如果c是一个1分钟级别的背驰，最终引发下跌拉回B里」——小转大的定义性特征恰是**父级无背驰段**。
- **关键分域裁定锚**（`doc-level-domain-20260717.md:136-140`，其第 5 条逐字）：「027:46 的逐级收缩程序适用于情况一（父级有背驰段）；小转大下定位对象就是小级别背驰点本身，逐级套无从谈起（无父级背驰段可套）。∴ 验收『深度≥2 的链』时，小转大样本**本来就不产生多级链**。」——**多级区间套链的教义对象域 = 情况一**；对情况一要求每级皆背驰段（027:38/46、061:32）是严格读法的直接推论，不与小转大教义冲突（见 §5.2）。

### 1.2 调研结论输入（在案文档）

- `doctrine-vs-code-nest-recursion-20260720.md:99`：typed 路径「rung 级不再要求 divergence_confirmed」（nest.rs:716-719 注释逐字），力度确认只留在基例门——G-2（:149-151）判为「语义降级：结构嵌套链 ≠ 区间套」。
- `nest-chain-existing-inventory-20260720.md:55`：econ 门 BTC 300K 实测 **95.36% 通过门信号 rungs 空**（退化 `Conf^δ_e`，max 深度=1，`econ_positive.rs:349-352`）；:97-98：sidecar 严格装配 66 张证书**单级占 72.7%**。
- `p105-cert-level-spectrum-20260717.md:16`：66 张证书（A=41/B=25）链深 1/2/3/4 = 48/12/5/1（A：24/11/5/1，B：24/1/0/0，:44-45），最深 4 级，塔顶 max_lvl=4。
- E2E-N2 定义（`chanlun/plans/mainline-merged-roadmap-20260717.md:72`）：「N2 跨级包含谓词缺｜`is_sub` 只在 nest 管线｜塔原生 `LineageEdge` 保存 `child.divergence_interval ⊆ parent.divergence_interval` 的逐边见证、同 `side` 及 skip 级别｜验收 `E2E-S3`」；依赖序 `E2E-N0→N1→N2→N3`（:79）。**本卡是 V2（typed 装配路径）的 rung 级严格化，不宣称结算 E2E-N2**（塔原生 LineageEdge 仍是缺口，见 §8 边界）。

---

## 2. 现状机制亲读核对（设计的事实底座）

### 2.1 装配核：`extend_typed_upward`（`rust/src/theta_v0/classifier/nest.rs:692-756`）

- DFS 骨架：级 `level` 越顶即成功（:705-707）；候选按 `(typed_interval(...).sel_key(), turn_source, index)` 升序排序（:711-715），逐候选尝试、失败回溯 pop（:749-753）。
- **现行 rung 门只有两项**：方向一致 `event.side != side → continue`（:720-722）；闭包含 `!is_sub(child, &parent) → continue`（:723-726，`is_sub` 定义 :67-69，闭口径）。
- 力度真值现状：#97 D1 注释（:716-719）逐字「初筛只看结构（D1 裁定：Cand=纯结构宽候选，力度留在②基例生产门）：rung 级不再要求 divergence_confirmed」；`divergence_confirmed` 仅在 rung push 同位被 `confirmed.push(...)` 恢复进 sidecar 向量（:731-733，p118 关④ D-1），**不参与否决**；`NestRung::assembled(..., true)` 的 `cand` 恒 `true`（:727）。
- 基例门已有力度：`assemble_typed_certificate` 前置 `!base.divergence_confirmed → None`（:637-639）+ 终端背书 `confirm_side`（:641-644）。**力度门缺的位置恰是且仅是 rung 级**——本卡的落点。
- 对照旧路径：`extend_upward`（:953-1011）每级门 `!ev.cand_delta → continue`（:984）——「每级力度门」在仓内有先例，但旧路径已 `#[deprecated]`（:861），其 `cand_delta` = 面积单通道严格 `curr < prev`（`nest.rs:779-791` 定义注），且被套对象口径 `I(A_child)⊆D_parent` 混入 A 段（`nest.rs:830-837` vs :804-810；G-4，`doctrine-vs-code:157-159`）。**V2 不是复活旧路径**（理由见 §3.3）。

### 2.2 力度谓词族（单源物化层，`rust/src/theta_v0/classifier/`）

- **Trend 支**：`trend_confirm_time`（`level_view.rs:542-633`）= 全合取首个全成立时点 t\*：T4 回拉 0 轴（`dif_crosses_zero`，:554-558）∧ T3 c 含对 B 三买（:565-566）∧ T2 c 包络破 b 包络（:622-630）∧ T5 力度或关系（同色面积 ∨ 黄白线峰 ∨ 同向柱峰，:607-621）。教义锚见函数头注 :526-541（025:761/024:24/037:18/037:20/027:32/033:26）。
- **Pan 支**：`segments_diverge_or`（`divergence.rs:334-349`）= 027:32 力度或关系（同色面积 C<A ∨ 黄白线峰 C<A ∨ 同向柱峰 C<A），复用 `same_color_area`/`segment_dif_peak`/`same_dir_hist_peak`（`divergence.rs:291-327`）。
- **单源物化**：provider `provide_nest_candidate_events`（`level_view.rs:646-655` 起）把两支行分别算好后写入**同一字段** `NestCandidateEvent.divergence_confirmed`（:487）——Trend 分支读 `view.confirm_times[pair_idx]`（:697-702，`Some(t) ⟹ confirmed=true` 且 `interval_b/turn_source` 收束到 t\*）；Pan 分支 :768-776 调 `segments_diverge_or`。
- **单一裁决源证据**：`view.confirm_times` 是 wave-1 方案 1 的 per-pair 缓存（:947-949 字段注、:1016-1036 逐 pair 单次计算）；`wave1-trend-confirm-double-compute-evidence-20260719.md:27` 实证 assemble 侧与 provide 侧对同一 pair **逐字同入参调用同一纯函数**，结果复用不改语义。**仓内不存在第二台产出 Trend 力度真值的机器**；rung 门读 `event.divergence_confirmed` 即与基例门（:638）同源同值，物理上不可能分叉。

### 2.3 递归判定核（消费侧，本卡不动）

- `NestCertificate::n_delta_rec`（`nest.rs:325-341`）：递归步 `top.cand ∧ is_sub(child, &top.interval) ∧ 递归`。门后 `cand` 仍由构造保证恒 `true`（:727），`n_delta` 语义不变；新增不变量「rungs 全部来自 confirmed 事件」由装配层保证（§4.3 debug 断言），证书本体无字段可复验该前置——与既有诚实边界同款（`nest.rs:291-295` cert F-01）。
- strategy 层 `chi_bool`（`strategy/nest.rs:122-145`）：递归步已有 per-level 槽位 `head.candidate_ok ∧ 次级严格低一级 ∧ sub_b(next, head) ∧ 递归`（:135-138；`NestLevel.candidate_ok` 定义 :86-90）。教义上 `candidate_ok` 即「该级是背驰段候选（含力度）」的 Bool 标记——**V2 门使 classifier 侧链语义与 `chi_bool` 的谓词形状对齐**；`interp.rs:382-395 nest_confirm` 只喂单级链（基例），不在本卡范围（喂多级链属 E2E-N3）。

### 2.4 消费域（影响面的边界）

- typed 装配的消费方 = 探针 bin（`p92_nest_replay_postruling.rs:838/:927`、`p123_fast_replay.rs:1345/:1413`、p102/p107/p108/p109/p111）+ runner sidecar（env `THETA_STRICT_NEST_SIDECAR` 门控）——**生产入场门不消费 typed 证书**（`nest-chain-existing-inventory-20260720.md:35` A 行）。econ 生产门 `build_nest_certificate`（`econ_positive.rs:890-971`）是另一条塔外近似链，**本卡不动**，其 95.36% rungs 空基线不受影响。
- sidecar 下游：`turn_class.rs:99-125 classify_certificate_turn` 读 `confirmed()[0]`（:103/:112-113）分流 `NestedConfirmed` / `XiaozhuandaCandidate` / `ExecEvidenceOnly`；p92 TURN_CLASS dump 行（`p92_nest_replay_postruling.rs:888-908`）与 CERT 行同主键配对。

---

## 3. 力度谓词选择论证

### 3.1 选哪一支：按事件 kind 分域，与基例门同族同支

027:22 的背驰段定义含「背驰**或盘整背驰**」——rung 级力度谓词必须覆盖两类，而仓内两类各有已裁定的谓词机：

| rung 事件 kind | 力度谓词支 | 机器锚 | 教义锚 |
|---|---|---|---|
| `Trend` | `trend_confirm_time` 全合取（T4∧T3∧T2∧T5-OR，t\* 收束坐标） | `level_view.rs:542-633`；消费 :697-702 | 025:761/024:24/037:18/037:20/027:32（函数头注 :526-541） |
| `Consolidation` | `segments_diverge_or` 力度或关系（面积∨黄白线∨柱峰） | `divergence.rs:334-349`；消费 :768-776 | 027:32/026:521/025:38（函数头注 :329-333） |

**判别式对 nest 层不可见**：两支都已物化为 `event.divergence_confirmed` 一个 bool（§2.2），rung 门写法是 kind-blind 的字段读取——这与基例门（`nest.rs:638` 同样读 `base.divergence_confirmed`）**同族、同支、同值域**，链上每一级（基例 + rungs）的力度判据严格同一来源。

### 3.2 复用哪台机器：provider + confirm_times 缓存，零重算

- **决策**：rung 门 = `extend_typed_upward` 内读 `event.divergence_confirmed`。**不在 nest.rs 引入任何 MACD/结构重算**——nest.rs 当前签名无 `hist/dif/close_src` 入参（:692-704），引入即判据层与装配层混合，且必然产生第二查法。
- **单源保证**（三证）：① Trend 真值的唯一产出点是 `view.confirm_times`（wave-1 单源化，`level_view.rs:1016-1036`），provide 侧只查表（:698）；② 双算证据文档确认同入参同纯函数、复用不改语义（`wave1-trend-confirm-double-compute-evidence-20260719.md:14-27`）；③ Pan 真值唯一产出点是 provide 内 :770-776 的单次 `segments_diverge_or` 调用。**装配层只消费字段 ⟹ 不存在可 fork 的第二台机器**。
- p118 关④ D-1 已把该字段值送到 rung push 同位（`nest.rs:733`）——本卡的改动是**同一行代码位置上的身份提升**（sidecar → 门），数据流零新增。

### 3.3 排除项（为何不选其它支）

- **排除旧路径 `cand_delta`**（`nest.rs:984`）：面积单通道严格 `curr < prev`——p113 实测 30.4% 聋度（转引 `level_view.rs:768-769` 注释与 `divergence.rs:280-284`：面积单通道必要门窄于 027:32 原文）；且旧路径已 deprecated（`nest.rs:861`）、口径混入 A 段（G-4）。V2 用 R1/R2 全谓词族，不是退回单通道。
- **排除在 nest.rs 新造「rung 专用」力度谓词**：任何新判据 = 新的不严格实现 + 第二裁决源，违反本 goal 严格化本体与 090。
- **排除用 BSP 终端背书当 rung 力度门**：终端背书（`nest.rs:596-611 terminal_bits_at_event`）是基例 `Conf^δ_e` 的查法，p105 实证高级 rung 的 turn_source 上同级根本没有 BSP（`p105-cert-level-spectrum-20260717.md:85-86`「区间套的高级环节是背驰事件嵌套，不是 BSP 嵌套」）——拿 BSP 门 rung 等于把多级链灭门，超出教义要求（027:46 只要求每级是背驰段，不要求每级有该级买卖点）。
- **排除把 `cand` 字段改载力度真值**：`NestRung.cand` 在门后由构造恒 true，改载 `divergence_confirmed` 是同值重写、无语义增量，反而模糊「门在装配层」的责任分界——保持 :727 原样。

---

## 4. 实装卡（前/后伪码 + 行号锚）

### 4.1 落点 1（唯一生产语义改动）：`extend_typed_upward` 候选过滤

**前**（`nest.rs:716-733`，逐字摘录现行逻辑）：

```rust
for index in order {
    let event = &events[index];
    // #97 初筛只看结构（D1 裁定：Cand=纯结构宽候选，力度留在②基例生产门）：
    // rung 级不再要求 divergence_confirmed；力度真值仍在事件字段上独立可查。
    if event.side != side {
        continue;
    }
    let parent = typed_interval(event, caliber);
    if !is_sub(child, &parent) {
        continue;
    }
    rungs.push(NestRung::assembled(event.judge_at, *child, parent, true));
    kinds.push(event.kind);
    clocks.push(event.judge_at);
    ids.push(NestEventIdentity::of(event));
    // p118 关④断链点修复：……
    confirmed.push(event.divergence_confirmed);
```

**后**（伪码；新增 1 个合取项 + 注释改写，其余逐字不动）：

```rust
for index in order {
    let event = &events[index];
    // V2 N2 rung 级力度门（v2-rung-force-gate-implementation-card-20260720 §4；
    // supersede #97 D1 初筛口径，裁定登记见该卡 R-1）：多级链 = 027:46 情况一的
    // 「背驰段的背驰段」（027:38/061:32），每级 rung 除方向一致与闭包含外，须过
    // 背驰力度门——谓词 = provider 单源物化的 event.divergence_confirmed
    //（Trend = trend_confirm_time 全合取 / Consolidation = segments_diverge_or，
    //  与基例门 :638 同字段同源，本层零重算、不 fork）。
    if event.side != side || !event.divergence_confirmed {
        continue;
    }
    let parent = typed_interval(event, caliber);
    if !is_sub(child, &parent) {
        continue;
    }
    rungs.push(NestRung::assembled(event.judge_at, *child, parent, true));
    kinds.push(event.kind);
    clocks.push(event.judge_at);
    ids.push(NestEventIdentity::of(event));
    // p118 关④断链点修复：……（门后恒 true，不变量见 §4.3）
    confirmed.push(event.divergence_confirmed);
```

### 4.2 落点 2（注释对齐，090 声明=能力）

- `nest.rs:613-617` `assemble_typed_certificate` 头注「Cand 由 provider 构造即真；力度在 divergence_confirmed 独立合取」——改写为「力度在基例门（:638）与 **rung 级门**（`extend_typed_upward`）各合取一次，同字段同源」。
- `nest.rs:716-719` #97 D1 注释——按 §4.1 伪码改写，登记 supersede（裁定 R-1）。
- DFS 排序（:711-715）、回溯 pop（:749-753）、`level > top_level` 终止（:705-707）**逐字不动**。

### 4.3 落点 3（不变量断言，零行为改动）

`assemble_typed_certificate` 在 `confirmed_low_to_high.reverse()` 之后（:671 后）追加：

```rust
// V2 N2 门后不变量：confirmed 向量全真（基例门 :638 + rung 级门双重保证）。
// release 编译掉，与 cert F-01 builder 断言（:355-370）同款纪律。
debug_assert!(confirmed_low_to_high.iter().all(|&flag| flag));
```

### 4.4 明确不改清单（禁越界）

`n_delta_rec`（:325-341）、`is_sub`/`sel_order`/`select_best`（:58-92）、`NestRung`/`NestCertificate` 结构（:155-247）、`typed_interval`（:464-474）、provider 全部（`level_view.rs`）、`segments_diverge_or` 族（`divergence.rs`）、终端背书（`nest.rs:487-611`）、旧 `extend_upward`（:953-1011，保持 deprecated 原样）、`chi_bool`（`strategy/nest.rs:122-145`）、`turn_class.rs`、econ `build_nest_certificate`、runner 生产门、p92/p123 bin 行格式（CERT/TURN_CLASS/CKPT 行契约零改——**集合变，格式不变**）。

---

## 5. bit-exact 影响分析

### 5.1 集合变动的精确机制（只有三条通路）

1. **杀死**：链上任一级只存在 `confirmed=false` 的可包含事件 ⟹ 该 (exec, top) 配对装配失败 → `None`（∃ 语义如实报负，`nest.rs:852-854`「链不完整不出半成品」语义不变）。
2. **重路由（横向）**：某级有多个可包含候选，字典序靠前者 `confirmed=false` 被门跳过，DFS 取次序靠前的 `confirmed=true` 候选——排序键不变（:711-715），跳过只删候选不改相对顺序，确定性保持。
3. **重路由（纵向变浅）**：probe 的 exec..top 全配对循环（`p92_nest_replay_postruling.rs:764-794` 兜底形态）下，同一 base 对更浅 top 仍可成链 ⟹ 原深链消失、同基例浅链新增（ids 前缀共享）。

**bit-exact 核心命题（可实现波须证）**：基线中 `confirmed_vec` 全 `'1'` 的证书，门后 **bit-identical 存续**（ids/kinds/judge_at/CERT 行全等）。论据：门只删除候选；对全 confirmed 链而言，其被选中的每级事件仍可行，且排在它前面的候选在旧语义下已因 side/is_sub 失败（新语义下仍失败）⟹ DFS 首次可行命中不变。

### 5.2 turn_class 语义冲击（本卡最大影响面，裁定 R-2）

- 门后每张证书的 `confirmed()` 恒全真 ⟹ `classify_certificate_turn`（`turn_class.rs:99-125`）恒走 `NestedConfirmed` 分支（:112-113）；`XiaozhuandaCandidate`（:82）与链侧 `ExecEvidenceOnly`（:86）**类空**。
- **教义自洽性**：`doc-level-domain-20260717.md:140` 第 5 条已裁「小转大样本本来就不产生多级链」——043:34 定义性特征即父级背驰段不成立，逐级套无从谈起。严格门把这条文档裁定落成代码事实：**多级链的对象域纯化为情况一**（027:46 域）。
- **与 p118 的冲突（照实）**：p118 关④ 设计前提逐字为「链**可以**穿过未确认父级成证（教义正确：中间各级无需背驰证书）」（`p118-xiaozhuanda-branch-design-20260718.md:42`），其 XiaozhuandaCandidate 编码依赖未确认 rung 链存在。严格门使该编码失去输入——这不是误杀，是**小转大检测对象的重归属**（小转大的合法对象 = 044:24 的 c′ 三类点结构，不是 027 区间套链）。但 p118 已 ship，删除/空置其分类类属裁定事项：**R-2 待 Lead 裁**，本卡建议方向 = 保留 `turn_class` 机器（对合成输入仍正确，防回归测试保留），并在其模块头登记「生产装配自 V2 N2 门起不再产出 top-unconfirmed 链」。
- **DeferOrphan 计数上升**：未被任何链消费的未确认 Trend 事件（c 破极值）在门后**全部**孤儿化（门前往下可被链消费为 rung）⟹ `DeferOrphan`（`turn_class.rs:87-89`，039:34 defer 域）承接该质量迁移——partition 不破（每证/每事件仍恰一类），质量从 XiaozhuandaCandidate 迁往 DeferOrphan。

### 5.3 不动面（预期 diff=0 的维度）

- P92 五维（`p92_nest_replay_postruling.rs:377`：old_path/tower/moves_centers_bsp_pan/lifecycle_cp_ownership/classification_total）——门只动 typed 装配候选过滤，塔/信号/BSP/分类本体零触。
- MACD 序列、provider 事件集（每个事件及其 `divergence_confirmed` 值）、BSP bits、终端背书、`certificate_key` 去重键、所有 dump 行格式。
- **二阶几何说明（照实登记，非新增影响）**：confirmed 事件的 `interval_b` 收束到 t\*（`level_view.rs:699-702`），比未确认事件的全离开段坐标**窄**；门后父候选只剩窄坐标事件，`is_sub` 更难满足——这不是门的新效应，而是「未确认宽坐标事件退场」的直接后果，教义上正确（027:38 的父背驰段 = 到确认点为止的段）。

---

## 6. 链深分布重测协议（实装波执行；本工位不跑 cargo）

**基线（归档在案）**：`p105-cert-level-spectrum-20260717.md` §2.1/§8——66 张（A=41/B=25），深度 1/2/3/4：A=24/11/5/1、B=24/1/0/0（:44-45）；基线 dump = `/tmp/p92_ckpt_dump.txt`（注意：该 dump 可能早于 p118 TURN_CLASS 侧信道，故协议含 pre-run 重建基线）。

**步骤**：

1. **pre-run**：实装前以 HEAD 二进制定点复跑 p92 全量重放（P92_CKPT 侧信道开），落 `dump_pre`（CERT + TURN_CLASS + CKPT 全行）；p105 探针复跑得 `depth_pre` 表——与归档 p105 表逐格核对（应全等；不等则先查明漂移再动工）。
2. **实装**：§4 三落点。
3. **post-run**：同参复跑得 `dump_post` / `depth_post`。
4. **不变量核验**：
   - **I1**：`dump_post` 全部 TURN_CLASS 行 `confirmed_vec` 恒全 `'1'`（grep 校验，零例外）。
   - **I2**（存续性）：`dump_pre` 中 `confirmed_vec` 全 `'1'` 的 CERT 行，在 `dump_post` 中 bit-identical 在场（§5.1 命题的实证）。
   - **I3**（归因完备）：`dump_pre` 含 `'0'` 的 CERT 行，逐张归入 (a) 杀死（无任何 post 证书共享其基例身份）、(b) 纵向重路由（post 中存在同基例、ids 为其前缀的浅链）、(c) 横向重路由（post 中同 (exec,top) 但 rung 身份替换）——**零未归因**。
   - **I4**（类迁移对账）：`XiaozhuandaCandidate` 计数 pre = X ⟹ post = 0；`DeferOrphan` 增量 = pre 中被链消费为 rung 的未确认事件数（可由 pre CERT 行 ids × confirmed_vec 机械复算）。
   - **I5**：P92 五维（§5.3）diff=0。
5. **产出**：新 review-results 文档归档 `depth_post` 表（对照 p105 基线）+ I1–I5 核验读数 + 逐张归因清单。
6. **预期方向（不给数字，090 不拍收益/损失）**：深度 ≥2 链收缩（p105 的 17 张 A 多级 + 1 张 B 多级中，含未确认 rung 者消失或变浅）、单级占比自 72.7% 进一步上升、总证书数下降；econ 侧 95.36% 基线不动（另一条链）。

---

## 7. 单测设计（实装波落 `nest.rs` `#[cfg(test)]`；构造方式：`NestCandidateEvent` 字段全 pub，`level_view.rs:480-502`，可直接字面量构造；`terminal_of` 用返回 `buy1_bits()` 的闭包桩）

| # | 测试名（建议） | 构造 | 断言 |
|---|---|---|---|
| T1 | `rung_gate_rejects_unconfirmed_parent` | L1 base（confirmed=true）+ L2 事件同向、包含、**confirmed=false** | `assemble_typed_certificate(top=2)` = `None`；同 base `top=1` = `Some`（基例不受影响） |
| T2 | `rung_gate_accepts_confirmed_parent` | 同上但 L2 confirmed=true | `Some`，链深 2，`identities()` 高→低含基例 |
| T3 | `rung_gate_dfs_reroutes_to_confirmed_candidate` | L2 双候选：sel 序靠前者包含但 confirmed=false，次前者包含且 confirmed=true | 成链且 rung 身份 = 次前候选（横向重路由确定性） |
| T4 | `rung_gate_base_semantics_unchanged` | base confirmed=false | `None`（基例门 :638 行为零改） |
| T5 | `rung_gate_sidecar_all_true_invariant` | T2 链 | `cert.confirmed().iter().all(\|&f\| f)` 且 `confirmed().len() == identities().len()` |
| T6 | `rung_gate_kind_blind_pan` | L2 Consolidation 事件 confirmed=false | 同 T1 拒——门 kind-blind（同字段，不感知机器支系） |
| T7 | `rung_gate_backtrack_pop_symmetry` | L2 confirmed=true 可入、L3 无可行 ⟹ 整体失败 | 返回 `None` 且不 panic（五向量 push/pop 对称的内部不变量由 :749-753 既有结构保证，本测试钉死回归） |
| T8（turn_class 侧，防回归注释） | 既有 `turn_class.rs:399/:520/:772` 合成证书测试 | 不动 | **保留**——`from_parts` 数据载体路径合成的 `confirmed=[false,true]` 输入对分类函数仍合法；测试注释追加「生产装配自 V2 N2 门起不再产出此类证书（R-2）」 |

回归线：`cargo test --lib` 全绿（classifier 既有 317 测试不得改判据性断言；凡断言「未确认 rung 可成链」的旧测试 = 断言旧口径，属本卡授权改写对象，逐条登记进实装波报告）。

---

## 8. 裁定需求与边界声明

### 8.1 待裁定项（本卡均不自决）

- **R-1**：supersede #97 D1「rung 初筛只看结构、力度留在基例门」口径（`nest.rs:716-719`）——登记到裁定链（本卡 §1.1 教义锚 + `doctrine-vs-code-nest-recursion-20260720.md` G-2 为裁料）。
- **R-2**：`XiaozhuandaCandidate` 类空后的处置——保留机器 + 登记「生产不再产出 top-unconfirmed 链」（本卡建议），或按 `doc-level-domain-20260717.md:140` 把小转大检测整体迁出 nest 链载体（另立 c′ 三类点对象）。p118 编码前提与本卡的冲突逐字对照见 §5.2。
- **R-3**：本卡是否只落 typed 路径——建议**是**（旧 `extend_upward` 保持 deprecated，其每级 cand 门不回摘；econ 塔外近似链不动）。

### 8.2 090 / v3 自查

- 全部判定带锚；代码行号为 HEAD `640609071d` 树内亲读行号；教义承重行（027:22/38/44/46、061:32/34、043:34、044:16）本工位亲读；065/037 系转引 `doctrine-vs-code-nest-recursion-20260720.md`（其亲读）。
- 本文一切命题为结构/语义设计（谓词合取、候选过滤、集合归因），零概率推断、零回测验证策略、零 EMH 假设；p105/econ 实测数字只作基线描述。
- 照实否定登记：本卡**不**结算 E2E-N2（塔原生 `LineageEdge` 仍缺，roadmap:72）；**不**解决 G-1 活假设、G-3 时钟倒置、G-6 类背驰地板、G-7 skip edge（`doctrine-vs-code` §5，均属 E2E-D5/D6/F/L 范畴）；**不**改动生产入场路径（typed 装配本就不入生产门，§2.4）。声明 = 能力：本卡交付的是「typed 装配路径 rung 级力度门」的实装设计，仅此而已。
