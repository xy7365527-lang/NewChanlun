# #115 beta-route 结果包：force_state 生产热路由激活（简化三要素）

- **工位**：原主 ws-route3（Opus 停用令中途停机）；现任 owner ws-g2z（复核+代落+收尾）
- **日期**：2026-07-03
- **规格源**：prereg-beta-div-20260703.md + beta-bucket-design-20260703.md v2 §6 路由⑤
- **commit 链**：批1 `ec16810bac`（TowerCache dif+closes_tick 增量通路，route3 自落）→ 批2
  `f575b8fe60`（classifier 三文件终稿，route3 完成、ws-g2z 复核代落）→ `c3ed1ccb81`（econ 消费
  侧调用点，与 #132 σ_higher 联合形态）→ `ca6f0cd794`（(e) re-scope fill-rate 断言）→
  `594d2ea738`（涟漪补齐：strategy/mod.rs 4 处测试 fixture force: None，恢复 HEAD 自编译）。

## 1. 结论

接线步骤 (a)-(e) 全部落地：

- **(a)** `BspPoint` 加 `force: Option<ForceProxies>`（bsp.rs）——**手写 `PartialEq`/`Eq` 排除
  force**（力度铁律：force 绝不进相等/去重/`class_index`/`BspBits`/分桶 key，纯旁挂力度 proxy）。
  复核确认：唯一消费点是 selector `z_of_candidate_with_force` → `ForceProxies::force_state()` →
  `MuClass.force_state` 第 8 维；结构语义零侵入。
- **(b)** 生产热路径传真 `dif`/`closes_tick`（classifier/mod.rs 全量+增量两路，signal.rs 单一来源
  `extract_signals_with_hist` 6 参；hist/dif 同一 `compute_macd` 单趟产出，无额外 O(n) 扫描；
  增量路 `mem::take` 出借/放回同 closes 模式）⟹ 一类趋势背驰候选（A/C 对）`point.force=Some(4 proxy)`，
  二/三类恒 `None`。级别-N 经 `extract_first_third_for_level` 同源（含 force）。
- **(c)** econ `collect_signals`：`z_of_candidate_with_force(c, p.force, &tower_i, bars)`（4 参
  终态=force 第 8 维 + σ_higher 第 9 维联合，#132 协同）。
- **(d)** dx harness `signals_dx` 同源同参（econ 3629）——signals_dx==signals_prod 逐条对拍断言
  在 `#[ignore]` L2 批测内，随下次 BTC 跑批执行。
- **(e)（re-scope，见边界条件）** fill-rate 断言落 econ `l2_btc_capturable_spread_diagnosis`
  L2 报告（force 真实流经处）：`assert!(n_type1 == 0 || n_force_some > 0)` + fill-rate 报告行。
  δ-共线逐层门随"force 真值进置换管线"的未来 prereg 同批。
- **GOLDEN 翻转（诚实重算，route3 完成）**：`extract_signals_bit_exact_digest_guard`
  0x56ed_dd65_1c59_5733 → **0x90c7_9ee6_17e1_1392**。翻转原因=BspPoint `#[derive(Debug)]` 每点
  尾部均匀多 `, force: None` 常量串（本电池传空 dif/closes ⟹ force 恒 None，非力度值差）；
  六 bit+pivot+center+struct_break_dir 逐字段不变由 `extract_signals_bit_exact_vs_orig_per_case`
  （PartialEq 排除 force）锁定——同 struct_break_dir 先例，不自定义 Debug 隐藏字段假装未变。
- **验证**：`cargo test --lib` **1436 passed / 0 failed / 108 ignored**；rust 树干净（工作树==HEAD）
  ⟹ 干净 checkout 等价全绿（594d2ea738 修复 code-verifier 坐实的 HEAD 不自编译后复验）。

## 2. 边界条件

- **(e) re-scope 的前提**：fullz 置换管线的 records 经 fill loop `z_of_candidate`（`Candidate`
  结构无 force 字段，force 在 Candidate 边界即丢）⟹ 该管线 force_state 架构性恒 None——规格
  原文位置（perm_test fullz）的 fill-rate 断言必然误报。**若未来扩展 Candidate/fill loop 携带
  force**（跨 G4 TypedTradeLedger 域的架构变更），则 (e) 应回迁 fullz 消费点，且 δ-共线逐层门
  （§3.2，δ-纯桶降级仅 μ 分层不置换）必须同批激活——与 σ_higher 进置换分层同一批 prereg
  （三处同批原则：perm base 键/重构闭包/wverify 投影，见 g2-impl-20260703.md 边界条件）。
- **fill-rate 断言的翻转条件**：若某真实窗口存在一类信号但其 force 合法全 None（如全部一类候选
  无先行中枢 ⟹ 无 A 段可配对），断言会假失败——届时应把断言细化为"有 A/C 对的一类"口径，
  而非删断言。
- **GOLDEN 的翻转条件**：BspPoint 再增/删 `derive(Debug)` 可见字段即再翻转——按本次与
  struct_break_dir 两次先例的同款流程诚实重算并留痕历史值。

## 3. 影响声明

- **改动文件**：`rust/src/theta_v0/classifier/{bsp,signal,mod}.rs`（批2）、
  `rust/src/theta_v0/backtest/econ_positive.rs`（(c)(d)(e)）、`rust/src/theta_v0/backtest/perm_test.rs`
  （文档指针订正）、`rust/src/theta_v0/strategy/mod.rs`（4 行测试 fixture 涟漪）。
- **行为不变面**：结构六 bit/去重/分桶/中枢/pivot 全部 bit-exact（PartialEq 排除 force +
  per-case guard 锁定）；perm 置换分层不变（force_state 在其唯一消费管线恒 None）。
- **行为改变面**：econ RawSignal.z 的 force_state 第 8 维现携真值（一类 A/C 对候选）——下游
  按 z.force_state 分层的 L2 诊断/报告自本批起有非 None 数据；GOLDEN 指纹翻转（仅 Debug 串）。
- **谱系/协调**：原主 route3 停机由 ws-g2z 代落的完整实况（含 HEAD 破窗修复）见
  g2-impl-20260703.md 影响声明节；本任务与 #132 在 econ 调用点为联合形态，不可单独 revert
  其一（revert 需两任务调用点同批处理）。
