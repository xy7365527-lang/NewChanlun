# Codex diagnose — 2026-07-02 15:47:10 UTC

## 元数据

- **mode**: diagnose
- **subject**: perm_test.rs 多stratum跨进程可复现性bug——已疑似修复(BTreeMap)，裁决修复选型/预注册处置/现行11桶Inconclusive结果是否受影响
- **model**: codex-cli
- **timestamp**: 2026-07-02 15:47:10 UTC
- **context-file**: /tmp/codex-permtest-ctx.md

## Prompt

## 诊断目标

perm_test.rs 多stratum跨进程可复现性bug——已疑似修复(BTreeMap)，裁决修复选型/预注册处置/现行11桶Inconclusive结果是否受影响

## 上下文

# 裁决请求：perm_test.rs 多 stratum 可复现性——正确性 + 预注册联动

## 委托来源

编排者 2026-07-02 明令「要裁决的全问 codex」。本工位（ws-codex-permtest）不改任何代码，
只取证并裁决。取证过程已完整执行，以下是取证事实 + 三项待裁决问题。

## 待裁决三问

(a) **修复方案选型**：多 stratum 下 RNG 消耗序不可复现的问题，正确的修复形态是什么
    （BTreeMap vs 排序 Vec vs 其他）？当前代码状态是否已经是正确修复？
(b) **预注册处置**：如果发现过 bug 又被修复，该如何处置已产出的历史结果——
    视为「实装 bug 修复，不改 estimand」，还是须「标注方法学修订 + 已跑结果作废重跑」？
(c) **现行权威结果是否受影响**：`.chanlun/review-results/wverify-alpha-final-20260702.md`
    报告的 11 桶 Inconclusive 判定（trades=3602, V=0 F=0 I=11）是否踩到了这个 bug？

## 取证事实（本工位已核实，请独立验证推理链是否有漏洞）

### 事实1：bug 在创世提交中确实存在

`git show 8ccbe58137 -- rust/src/theta_v0/backtest/perm_test.rs` 显示该文件创建时：

```rust
pub fn stratified_delta_perm_p(...) -> HashMap<BucketKey, f64> {
    let mut strata: HashMap<(u32, u8), (Vec<i8>, Vec<f64>)> = HashMap::new();
    for &(lv, bc, d, x) in trades {
        let e = strata.entry((lv, bc)).or_default();
        e.0.push(d);
        e.1.push(x);
    }
    ...
    let mut rng = SplitMix64::new(seed);
    for _ in 0..n_perm {
        for ((lv, bc), (ds, xs)) in &strata {   // ← HashMap 迭代，序不定
            let mut perm_ds = ds.clone();
            fisher_yates(&mut perm_ds, &mut rng);  // ← RNG 消耗序依赖迭代序
            ...
        }
    }
    ...
}
```

Rust `std::collections::HashMap` 默认用 `RandomState`（每进程随机化的 SipHash key，
进程生命周期内近似稳定，但**跨进程不保证一致**）。当 `strata` 有 ≥2 个键（多 stratum）时，
`for _ in 0..n_perm { for (...) in &strata { ...fisher_yates(&mut rng)... } }` 每次
外层循环都按同一个（但跨进程不确定的）HashMap 迭代序消耗 RNG——这意味着：**同一份
`trades` 输入 + 同一个冻结种子，在两次不同的进程运行中，可能产出不同的 `perm_p`**
（因为不同进程里 HashMap 迭代序可能不同，导致 RNG 序列被分配给不同 stratum 的方式不同）。

这违反预注册硬约束（`acc-alpha-estimand-prereg-20260701.md` §4：「置换次数 200，
固定种子 Fisher-Yates」——固定种子的意义就是跨进程可复现，否则"冻结种子"是空话）。

**关键细节（易被忽略）**：单进程内重复调用（如现有测试 `same_seed_reproducible`，
在同一个测试函数里连续调用 `stratified_delta_perm_p` 两次）**不会**暴露这个 bug——
因为同一进程内的 `HashMap::new()` 通常共享同一份随机化 hash key（`RandomState`
的随机源在进程/线程首次使用时初始化并复用），所以同进程内两次调用会得到相同的
迭代序、相同的结果。只有**跨进程重跑**（例如今天跑一次 `wverify_full`，
明天在新进程里重跑同一份数据）才会暴露不可复现。这就是为什么已有测试套件
全绿、但 bug 仍然存在——测试套件本身无法检出跨进程不可复现问题。

### 事实2：bug 在下一个提交中已被修复

`git show 7682aa4024 -- rust/src/theta_v0/backtest/perm_test.rs`（提交时间
2026-07-02 00:53:14，提交信息含「perm_test strata BTreeMap(迭代序确定性)」）：

```diff
-use std::collections::HashMap;
+use std::collections::{BTreeMap, HashMap};
...
-    let mut strata: HashMap<(u32, u8), (Vec<i8>, Vec<f64>)> = HashMap::new();
+    let mut strata: BTreeMap<(u32, u8), (Vec<i8>, Vec<f64>)> = BTreeMap::new(); // 确定序⟹跨进程 perm_p 可复现(prereg §4)
```

BTreeMap 按键排序，`.iter()` 迭代序完全由键值决定（`(level, bsp_class)` 数值序），
与进程、hash 随机化无关——跨进程恒定。这个提交同时新增了 `multi_stratum_reproducible`
测试（虽然如上所述，这个测试本身测不出跨进程问题，只能测出"同进程两次调用一致"）。

**这个修复提交（7682aa4024）本身也是那次"L0三买 VALIDATED"结果（trades=74/5桶）
的产出提交**——commit message 里同时包含了 wverify_run.rs 入口改动和这次跑批的
结果数字。这里有一个本工位无法仅凭 git 历史确证的时序问题：**这次 74-trade 跑批
是在 BTreeMap 修复代码落地之前跑的（然后才补 BTreeMap 修复，未重跑），还是修复
落地之后才跑的？** 两种可能都符合"改动和结果在同一个 commit"这一观察。

### 事实3：当前 HEAD（perm_test.rs）已是 BTreeMap 版本，且工位 D1（性能重构，
已 completed 但未 commit）保留了这个不变式

当前文件 `rust/src/theta_v0/backtest/perm_test.rs` 第86行（未提交的 D1 性能重构后）：

```rust
let mut grouped: BTreeMap<(u32, u8), (Vec<i8>, Vec<f64>)> = BTreeMap::new(); // 确定序⟹跨进程 perm_p 可复现(prereg §4)
for &(lv, bc, d, x) in trades {
    let e = grouped.entry((lv, bc)).or_default();
    e.0.push(d);
    e.1.push(x);
}
// BTreeMap 序 → Vec：锁定 stratum 迭代序（= RNG 消耗序，不可变），并内联 obs/ge。
let mut strata: Vec<Stratum> = grouped.into_iter().map(|(key, (ds, xs))| { ... }).collect();
```

`BTreeMap::into_iter()` 按键排序产出——`grouped.into_iter()` 转成 `Vec<Stratum>` 后，
后续 `for _ in 0..n_perm { for s in &mut strata { ...fisher_yates... } }` 遍历的
是这个已经按键排序固定的 Vec，顺序与原 BTreeMap 迭代序完全一致。**D1 的性能重构
（缓冲复用、单遍融合累加、结构体内联）没有改变分组容器类型或迭代序**——只是把
`&BTreeMap` 迭代改成了 `&mut Vec<Stratum>` 迭代，Vec 本身的顺序是从 BTreeMap 继承来的。

### 事实4：从 `trades` 输入的上游生产路径全程排查，未发现残留 HashMap 迭代序风险

调用链：`wverify_run.rs::wverify_full` → `build_mu_from_bars`（l3_delta_r_alpha.rs:124）
→ `MuEstimator::observe`（mu_estimator.rs:260）→ `MuEstimator::trades()`（mu_estimator.rs:266）
→ `wverify_run.rs` 第22-26行投影成 `Vec<(u32,u8,i8,f64)>` → 喂给
`stratified_delta_perm_p`。

- `MuEstimator.trades: Vec<(MuClass, f64)>`——`observe()` 里 `self.trades.push(...)`，
  纯 Vec append-only，顺序 = 调用 `observe()` 的顺序，不涉及任何 HashMap。
  （`MuEstimator.buckets: HashMap<MuClass, Welford>` 是另一个字段，只用于 `mu()`/`cv()`，
  **不**被 `trades()` 消费，perm_test 路径不碰这个 HashMap。）
- `build_mu_from_bars` 里 `signals: Vec<(usize, MuClass, VoiceSide)>` 的构建：
  单线程顺序扫描 `for i in 0..n { ... for (lvl, ls) in cls_i.levels.iter().enumerate()
  { for p in &ls.bsp { ...signals.push(...) } } }`——`cls_i.levels` 是 Vec（级别下标
  0..N，固定序），`ls.bsp` 是 Vec（该级别本 bar 新确认的买卖点，因果扫描顺序产出）。
  循环里唯一的 HashSet（`seen: HashSet<(usize,usize,u8)>`）只用作 `.insert()` 去重
  membership check，从不被迭代，不影响外层循环顺序。
  随后 `for (idx, &(entry_bar, z, dir)) in signals.iter().enumerate() { ... est.observe(...) }`
  也是对这个确定序 Vec 的顺序遍历。
- `wverify_run.rs` 第22-26行：`est.trades().iter().map(|(c,x)| (c.level, c.bsp_class(),
  c.delta, *x)).collect()`——1:1 投影，保序。

**本工位未能排查 `IncrementalClassifier::classify_at(i)` 内部实现**（递归塔/笔段/中枢
识别的具体递归算子）是否在构造 `ls.bsp`（某级别本 bar 新确认买卖点列表）时内部
使用了顺序敏感的 HashMap——这部分代码量大（递归塔实现），本工位判断超出「perm_test
正确性半边」这一具体工位的取证范围，但如果 codex 认为这是裁决所需的关键缺口，
请明确指出，本工位可以补充排查。

### 事实5：当前权威结果（11桶 Inconclusive）的产出时序

`.chanlun/review-results/wverify-alpha-final-20260702.md`（task #17，ws-geyer）是
在 `decontam.rs::effective_n` 改为 Geyer 配对 IPS 校正后重跑的结果（trades=3602，
11桶，V=0 F=0 I=11）。这个工位**只改了 decontam.rs，没有改 perm_test.rs**——而
decontam.rs 的改动发生在 `7682aa4024`（perm_test.rs 的 BTreeMap 修复提交）之后
（`git log` 序：8ccbe58137 → 7682aa4024 → 之后的 geyer 修复提交）。因此这次 11 桶
的权威跑批，其 `perm_test.rs` 依赖必然是**已经修复（BTreeMap）之后**的版本。

11 桶表中的 perm_p 列（例：L0/3/+1 perm_p=0.000, L0/3/-1 perm_p=1.000, L1/2/-1
perm_p=0.190 等）——由于 5 个 level × 2 个 bsp_class 桶同时存在（多 stratum
场景，正是这个 bug 会触发的条件：level 0-5，多个 (level,bsp_class) 键），
如果这次跑批用的是**修复前**的 HashMap 版本，这些 perm_p 数字将不可跨进程复现。
但如上所述，代码提交顺序表明这次跑批理应用的是修复后的版本。

### 事实6：算法优化计划文档（同日撰写）仍把这个 bug 描述为"未解决"

`.chanlun/review-results/algo-opt-plan-20260702.md` 第54行（泳道 D 描述）：

> **正确性半边单列**：多 stratum 下 HashMap 迭代序 × RNG 串行消耗 = 冻结种子
> 不可复现（违反预注册硬约束）——修复（BTreeMap/排序迭代）会改多 stratum 输出，
> **走 /escalate 连同预注册一起裁决**，不与性能半边捆绑落地。

第81行（不做清单）还提到一个**不同的、确认仍然存在**的同类问题：
`mu_estimator.rs` 的 `pooled_mean`（`shrunk_view` 相关，B²  复杂度）"既有 HashMap
迭代序不定性记入债务，与 D1 裁决同类"——这个函数**不在** perm_test 调用链上
（`wverify_full` 走的是 `est.trades()`，不走 `pooled_mean`／`shrunk_view`），
本工位未详细排查其具体风险面，只标注其存在、且明确不属于本次裁决范围
（该文档自己也把它列在"不做清单"，标记为已知债务、非本次修复目标）。

本工位读到这段文字时，perm_test.rs 的 BTreeMap 修复（7682aa4024）已经落地在
git 历史里——即这份"仍未解决"的描述，从代码事实角度看是**过时的**（很可能
是该文档撰写时沿用了更早的分析结论，未同步核对最新提交）。

## 相关代码全文（当前 HEAD，未 commit 的 D1 性能重构已包含）

```rust
// rust/src/theta_v0/backtest/perm_test.rs（完整文件，见下方 diff 之后的最终状态）
```

见附件路径 `rust/src/theta_v0/backtest/perm_test.rs`（本工位未改动，Codex 若可访问
文件系统请直接读取；若只能吃 context，上面「事实1/3」的代码块已经是关键片段全文）。

## 本工位的初步判断（请独立验证，不要因为本工位已给结论就跳过推理）

基于事实1-5，本工位的初步读数是：

1. **(a) 修复方案**：已经是正确修复——BTreeMap 分组 + 保序转 Vec，是本工位能想到的
   最小、最直接、最符合"确定序"要求的方案。当前 HEAD（含未提交的 D1 性能重构）延续
   了这个不变式，没有退化。**没有发现需要额外裁决的选型分歧**。
2. **(b) 预注册处置**：性质是「实装 bug 修复，不改 estimand」——BTreeMap 分组不改变
   分类格定义（仍是 `(ℓ,bsp_class,δ)`）、不改变置换次数/种子/单边判据，只是让"固定
   种子"这个预注册要求真正生效（修复前，"固定种子"名不副实）。**不需要**标注方法学
   修订或作废结果——除非 codex 认为「即便同进程内数字没变，曾经存在过一个会导致
   跨进程不可复现的 bug」这件事本身就需要在预注册文档里留痕（哪怕不影响任何已发布
   数字）。
3. **(c) 现行权威结果（11桶 Inconclusive）**：**不受影响**——产出该结果的代码路径
   （decontam.rs 改动叠加在 7682aa4024 之上）晚于 BTreeMap 修复提交，因此这次跑批
   使用的必然是已修复版本。

## 请 codex 裁决的具体问题

1. 上述事实链（1-6）和初步判断（1-3）有没有逻辑漏洞？特别是：
   - "同进程内 HashMap 迭代序稳定，跨进程不稳定"这个 Rust `RandomState` 行为的
     理解是否准确？如果不准确，会如何改变整个论证？
   - 有没有本工位遗漏的、仍可能导致跨进程不可复现的路径（尤其是本工位未排查的
     `IncrementalClassifier::classify_at` 内部）？
2. 如果事实链成立（bug 已修复、当前权威结果未受影响），预注册文档是否仍需要一条
   "已知过 bug、已修复、不影响已发布结果"的存档记录？还是这种级别的实装修复不需要
   在预注册层面留痕（因为预注册管的是 estimand/方法学，不是每一行实装 bug）？
3. `algo-opt-plan-20260702.md` 第54行要求"/escalate 连同预注册一起裁决"——如果
   本工位的取证证明这个顾虑已经是过时描述，是否应该判定"这条 /escalate 需求已经
   被 7682aa4024 的既成修复解决，不需要再走一次矛盾上浮"？还是不管代码现状如何，
   既然文档明确要求 escalate，就必须走一次形式上的裁决闭环（也就是本次 codex 调用
   本身）？

请给出明确裁决：(a) 修复选型是否已经合格（是/否，若否给出具体方案）；
(b) 预注册处置类型（实装bug不改estimand / 方法学修订留痕 / 结果作废重跑，三选一，
说明理由）；(c) 11桶 Inconclusive 结果是否受影响（是/否，说明理由）。


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

结论受一个边界限制：当前 workspace 里没有仓库内容，代码图 MCP 也返回 cancelled，所以我不能独立复核 git 文件和提交时序；我能独立确认 Rust `HashMap`/`RandomState` 行为，并对你给出的取证链做一致性裁决。

**裁决**

(a) 修复选型：合格。`BTreeMap<(u32,u8), ...>` 分组，再 `into_iter()` 保序转 `Vec<Stratum>`，是正确修复。排序 `Vec` 也等价正确；固定 hasher 的 `HashMap` 不如这两者，因为仍把语义绑定到 hash 表内部遍历。当前 HEAD 若确为你给出的 `BTreeMap -> Vec`，则已修复。

(b) 预注册处置：实装 bug 修复，不改 estimand。它不改变桶定义、置换次数、固定种子、检验方向，只是让“固定种子 Fisher-Yates”真正有确定的 stratum RNG 消耗序。建议留一条审计/errata 记录，但不要标为方法学修订。任何已知由修复前多 stratum 版本产出的权威结果，应 rerun 或标为 superseded；这是 artifact QA，不是 estimand 变更。

(c) 11 桶 Inconclusive：按你给出的提交时序，不受这个 perm_test bug 影响。理由是 `.chanlun/review-results/wverify-alpha-final-20260702.md` 的产出发生在 `7682aa4024` 之后，且 `perm_test.rs` 已是确定序版本。未排查的 `IncrementalClassifier::classify_at` 只构成另一个端到端复现性缺口，不推翻“未踩到 perm_test HashMap bug”。

**严格诊断链**

失败现象：同一 `trades`、同一 seed、多 stratum 时，修复前实现的 `perm_p` 没有跨进程确定性。现有同进程测试不是充分否定对象，因为它没有强制新进程复现。

直接原因：创世版本在 RNG 消耗路径中执行 `for ... in &strata`，其中 `strata` 是 `HashMap`，随后每个 stratum 调 `fisher_yates(&mut perm_ds, &mut rng)`。`HashMap::iter` 官方定义为 arbitrary order；`RandomState::new()` 官方源码还说明每次创建会改变 seed，使每个对应 `HashMap` 获得不同迭代序。见 Rust docs: HashMap iter arbitrary order，以及 RandomState source 对 seed increment 的说明。

根本原因：实现疏忽。代码把“分组容器”当成纯存储结构，但在串行单 RNG 设计下，stratum 遍历序已经成为随机化算法的一部分。预注册要求固定 seed，隐含要求 RNG 消耗序确定；这里不是定义冲突。

分类判定：实现错误，不是定义冲突。

精确修复步骤：用确定序容器或显式排序锁定 RNG 消耗序；当前 `BTreeMap` 按 `(level,bsp_class)` 升序分组，再转 `Vec` 后只遍历该 `Vec`，满足要求。补充测试应是跨进程测试：父进程两次启动子进程，对同一多 stratum fixture 输出按 key 排序后的 `perm_p`，逐字节比较；同进程连续调用不足以证明这个性质。

**逻辑漏洞修正**

你取证中的“同进程内 `HashMap::new()` 通常共享同一随机化 hash key”不准确。Rust 官方源码说明 per-thread 缓存初始 key，但每次 `RandomState` 创建会 increment 一个 seed，使每个 `HashMap` 有不同迭代序。这不会削弱结论，反而加强：HashMap 迭代序更不能作为固定 seed 置换算法的一部分。

`algo-opt-plan-20260702.md` 第54行的 `/escalate`：本次裁决已完成这个闭环。若代码事实确为 `7682aa4024` 已修复，应把该计划项标为“过时/已由既成修复解决”，不需要再次上浮；只需留下审计记录。
