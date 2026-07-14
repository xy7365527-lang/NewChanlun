# perm_test.rs 多 stratum 可复现性——正确性半边 + 预注册联动裁决

工位 ws-codex-permtest（task #27）| 委托来源：编排者 2026-07-02 明令「要裁决的全问 codex」
完整 Codex 交互记录：`.chanlun/review-results/codex-diagnose-20260702-1547.md`
本工位仅取证 + 转达裁决，未改动任何代码。

## 取证结论（本工位自行核实，git 历史 + 全链路代码走读）

1. **bug 确实存在过**：`stratified_delta_perm_p` 创世提交（`8ccbe58137`）用
   `strata: HashMap<(u32,u8),...>` 分组，随后 `for _ in 0..n_perm { for (...) in &strata { fisher_yates(&mut rng) } }`
   把 RNG 消耗序绑定到 HashMap 迭代序——违反预注册「固定种子 Fisher-Yates」隐含的跨进程可复现要求。
2. **bug 已在下一提交修复**：`7682aa4024`（同日 00:53:14，与"L0三买 VALIDATED"结果同一提交）
   把 `strata` 改为 `BTreeMap`（键序确定，跨进程恒定），并新增 `multi_stratum_reproducible` 测试。
3. **当前 HEAD（含未提交的 D1 性能重构 task #21）延续了这个不变式**：`grouped: BTreeMap` → `.into_iter()`
   保序转 `Vec<Stratum>`，RNG 消耗序未变。D1 的缓冲复用/单遍融合/结构体内联不触碰分组容器类型。
4. **全上游链路排查未发现残留 HashMap 迭代序风险**：`build_mu_from_bars`（l3_delta_r_alpha.rs）
   的 `signals: Vec` 单线程顺序扫描产出，`MuEstimator.trades: Vec` append-only，
   `wverify_run.rs` 1:1 投影保序。唯一未排查的是 `IncrementalClassifier::classify_at` 内部实现
   （递归塔，代码量大，判断超出本工位取证范围）。
5. **现行权威结果（11桶 Inconclusive，`wverify-alpha-final-20260702.md`）产出于 `7682aa4024` 之后**
   （task #17 ws-geyer 只改 decontam.rs），故其 perm_test 依赖必然是已修复版本。
6. **`algo-opt-plan-20260702.md` 第54行仍把此 bug 描述为"未解决、须 /escalate"**——从代码事实看是过时描述。

## Codex 裁决（独立验证后，一处修正 + 三项明确裁决）

**修正**：本工位取证中"同进程内 HashMap::new() 通常共享同一随机化 hash key"不准确——
Rust `RandomState` 每次创建会 increment seed，同进程内不同 HashMap 实例迭代序也可能不同。
这个修正**加强**而非削弱结论：HashMap 迭代序更不能作为固定种子置换算法的一部分。

**(a) 修复选型：合格。** `BTreeMap` 分组 + `into_iter()` 保序转 `Vec` 是正确修复
（排序 Vec 等价正确；HashMap 不合格，因为语义绑定到 hash 表内部遍历）。当前 HEAD 已是修复态。

**(b) 预注册处置：实装 bug 修复，不改 estimand。** 不改变桶定义、置换次数、固定种子、
检验方向，只是让"固定种子"真正生效。**建议留一条审计/errata 记录，不标为方法学修订。**
但补充条款：任何**已知**由修复前版本产出的权威结果应 rerun 或标为 superseded——这是 artifact QA。

**(c) 11 桶 Inconclusive：不受影响。** 产出时序在 BTreeMap 修复之后。
`IncrementalClassifier::classify_at` 未排查不推翻此判定，只构成另一个独立的、未确认的复现性缺口
（不在本次裁决范围，若要收口需单独取证）。

**诊断分类判定**：实现错误（非定义冲突）。根本原因：把分组容器当纯存储结构，
未意识到串行单 RNG 设计下 stratum 遍历序已成为随机化算法的一部分。

**测试缺口（Codex 指出）**：现有 `same_seed_reproducible`/`multi_stratum_reproducible`
都是同进程连续调用，不构成"跨进程可复现"的充分证明。正确的护航测试应是父进程 spawn 子进程
两次独立运行，对同一多 stratum fixture 输出逐字节比较。**本工位不实现此测试**（越权改代码）。

**`algo-opt-plan-20260702.md` 第54行 /escalate 的闭环状态**：本次 Codex 调用已完成该闭环。
若代码事实确认为 `7682aa4024` 已修复（本工位已验证），该计划项应标为"过时/已由既成修复解决"，
不需要再走一次矛盾上浮，只需留存审计记录。

## 遗留待办（errata 候选，非本工位裁决范围，转交 Lead/genealogist）

`7682aa4024` 提交自身报告的"L0三买 VALIDATED"（trades=74/5桶）结果，其 perm_p 数字究竟产自
BTreeMap 修复代码落地**之前**还是**之后**，仅凭 git 历史（改动与结果同提交）无法确证——
两种时序都符合观测。**该结果目前已被更晚、更严格的 Geyer 配对 IPS 校正跑批
（`wverify-alpha-final-20260702.md`，11桶 Inconclusive）取代**，acc-alpha 当前的官方结论口径
是 Inconclusive 而非 VALIDATED，因此这个溯源缺口对当前 acceptance 判定**无实质影响**。
按 Codex 建议的 artifact QA 标准，若要完全闭环，应在 errata 记录中显式标注这一点
（"曾经产出但已 superseded 的历史数字，其可复现性未经验证"），而非重跑（因为已被取代，重跑无意义）。

## 结果包六要素

1. **结论**：perm_test.rs 多 stratum 跨进程可复现性 bug 确实存在过，已在 `7682aa4024`
   通过 BTreeMap 修复，当前 HEAD（含未提交 D1 性能重构）延续修复。(a) 修复选型合格；
   (b) 预注册处置=实装bug修复不改estimand，留errata记录；(c) 现行11桶Inconclusive结果不受影响。
2. **定义依据**：`acc-alpha-estimand-prereg-20260701.md` §4 冻结常量「置换次数200，固定种子
   Fisher-Yates」——固定种子的规范意义蕴含跨进程可复现，修复前的 HashMap 分组使该常量名不副实。
3. **边界条件**：若日后发现 `IncrementalClassifier::classify_at` 内部存在顺序敏感 HashMap，
   或发现除 `7682aa4024` 外还有其他权威结果产自 BTreeMap 修复之前的 commit range，
   则 (c) 的"不受影响"判定需要重新核实对应结果。
4. **下游推论**：acc-alpha acceptance 工位可继续以 `wverify-alpha-final-20260702.md`
   的 11桶 Inconclusive 作为权威结论，无需因本次裁决重跑。`algo-opt-plan-20260702.md`
   第54行的 /escalate 需求视为已闭环，D1（task #21）性能重构可正常推进 commit 流程
   （其正确性不变式已在本次裁决中确认未被破坏）。
5. **谱系引用**：预注册文档本身（090号严格性——docstring/常量与实装一致的先例）；
   231号（有效域≠定义域，适用于"固定种子"这一常量声称与其实际实装行为的对齐核验）。
   未发现与既有 settled 谱系矛盾，本次是实现错误裁决，不触发新谱系分离。
6. **影响声明**：本工位未修改任何代码。裁决结果影响：(1) `algo-opt-plan-20260702.md`
   第54行的过时状态可由 Lead/genealogist 更新标注；(2) D1（task #21）的 commit 可视为
   不携带正确性回归；(3) errata 记录（若落地）应追加到预注册文档或 acc-alpha acceptance
   相关谱系，标注 `7682aa4024` 自身内嵌 VALIDATED 数字的溯源不确定性（已被取代，非阻塞）。
