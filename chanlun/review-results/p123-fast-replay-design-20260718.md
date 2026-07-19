# p123：快速端到端重放设计（sparse dirty-driven，2026-07-18）

**状态**：设计定稿（编排者综合四路深研：p122 实测热点 / 增量定理审计 / 并行确定性 / 架构候选）
**目标**：全量重放（BTC 1m 4,613,599 bar）墙钟从 ~16,400s 降到塔底座量级 1–2×（区间依据见 §6），语义零偏差。
**铁律**：与现行实现全量双跑，验收面 diff=0（090：禁简化实装；验收面见 §5）。

## 1. 成本形态（实测闭合，p122_replay_profile @500k/2M）

- terminal pass（增量塔）：4.6M bar ~730s，O(n) 摊还（TowerCache 全链前缀稳定复用）。
- prefix pass 税 ≈ 15,670s（95.5%）：`provide_events` 占税 72%（2M 实测 1,317.9/1,706s），其中 **L1 = 16,763 views × 76.7ms = 1,286s（82%）**。
- 形态 ≈ O(n²)：views=148,043（trigger ~0.9–1% bar × arrived (level,run) ~3.4）× 单 view O(L0 段数)（pan 分支 `level_view.rs:722` 扫全级下级段）。单 view 成本随 bar 超线性（4.2ms@500k → 76.7ms@2M）。
- 固定税：每次 trigger 对全部级做 run 分区全扫 + `lower_legs_from` 全量重建（跨 trigger 零缓存，`p116:690-712`）。
- 关键实测：**终态一次全量快照装配仅 0.29s@2M**——成本全在 148,043 次重复，且绝大多数产出与上次逐位相同。
- R1–R3 双算（trend_confirm_time 在 assemble/provide 各算一遍）把税较旧口径推高 ~7.5×（2,200s→16,400s）。

## 2. 决策：候选 C 主线（per-run dirty 驱动稀疏重算），B 叠加备选，A 否决

| 候选 | 估算 | 判定 |
|---|---|---|
| A 单趟+时钟重建 | ~1.05× | **否决**：收益近零，EventKey 演化归并风险（未确认事件 key 随 seg_c.1 漂移），路 2 判「部分可行+5 条件」前不值 |
| B 进程级 run 分片 | ~5×（S=8） | **叠加备选**：零生产改动、构造性 bit-exact，但 S×1–2GB 内存与运维成本；C 落地后若仍需再加 |
| **C per-run dirty 稀疏重算** | **~5–12×（区间，Phase 0 收拢）** | **主线**：直击 pending×全局 trigger 乘积结构；两段制骨架/targets 对齐/终态兜底全保留，语义偏差面最小 |

## 3. C 架构（端到端数据流）

```
pass 1（terminal）：原样不动（增量塔 ~730s，TurnBook.observe 逐 bar → TURN 行）
pass 2（prefix，新）：
  同一增量塔推进每 bar ──→ per-(level,run) 评估缓存：
    { tower[level] Rc 哨兵 + holding 引用（见 §4.1）、tower[level-1] 哨兵、
      legs 端点升序水位线、上次评估结果（投影/pairs/events）、TERM 命中集 }
  每 bar 后 dirty 判据：
    (i)   tower[level] 哨兵 ptr 变（frontier 重折/追加）
    (ii)  tower[level-1] 哨兵 ptr 变（lower legs 源）
    (iii) as_of 越过 legs 端点水位（O(越过数) 弹出；provide_divergence_pairs/pan 支/trend_confirm_time 的 as_of 依赖全经端点越过——已逐处核验 level_view.rs:829/:722/:562）
    (iv)  bsp append 流 → 仅对未命中 pending 做 TERM 反查（O(新增 bsp)，不重投影；signal_signature 对账本属过度触发，p116 仅 TERM 消费）
  dirty run → 现行同一批 lib 函数同一输入重估（零新判据路径）
  非 dirty → 复用上次事件集做 pending 匹配
  run 分区 + lower legs 按 tower[level] 哨兵缓存（消除每 trigger 固定税）
终态：快照兜底 observe_snapshot + 证书装配 + BASELINE/BIT_EXACT 对账——原样
CKPT 物理全量装配：不走稀疏，原样保留（验收时关闭，见 §5）
```

## 4. 关键实装锚（全部 bin 侧，零生产源码改动）

1. **Rc 哨兵 holding trick**：跨迭代持有上一 bar 的 tower 克隆（每 bar 每级 1 次 `Rc::clone`，O(1)）⟹ cache 内 `Rc::make_mut` 强制写时复制换新分配 ⟹ **ptr 变 ⟺ 该级发生过写**（重写同字节 = over-dirty，安全方向；漏报不可能）。已核验 make_mut 写入站点（mod.rs:1693/1743/1815/1905）。bin 注释声明此依赖。
2. **复用既有等价断言，零新证明负担**：投影 `centers.first()` O(1) 携带核直读（#89 carried≡detect bit-equal）；`decompose_resume` 前缀冻结（输出≡全折叠，decompose.rs:100-101/105-142）；「per-run≡全局 decompose」恒等（p117-s1a 设计附录 A :201-210，边界=切片首中枢永不消费）。
3. **trend_confirm 游标驻留**：env 包络/acc_hi/area/dif/hist 极值 per-pair 驻留（该函数内部本就是增量游标，per-prefix 被丢弃重扫）；t3 未命中保持**未决**（延迟三买 max 1,199,728 bar，禁结算为终假）；T5 终假即归档（level_view.rs:613 真→假单调）。
4. **trigger 语义冻结**：时钟=「trigger (forest_epoch, signal_signature) 快照上的首次观察」，非逐 bar（C5 条件）——首证钟归集 bar 取 trigger bar，与现行 `or_insert` 逐位对齐；「事件可证 ⟹ 同 bar trigger bump」双跑阶段以 diff 直接裁决。
5. **trend_confirm_time 单源化**：assemble/provide 双算消除（确认结论随 view 缓存由 provide 复用）——属性能优化不动判据，双跑对账锁等价。

## 5. 验收面（最小一致面，现在就钉死）

- **必须逐位**：stdout 门行全套（YIELD/CERT/D3/BASELINE/SNAPSHOT/BIT_EXACT/R7/MISSED(+EVENT)）+ dump 的 CERT（judge_at 向量逐字）/DIV/TERM/TURN/TURN_CLASS/FALLBACK（含行序）。
- **白名单剔除**：`P116_PROVIDER views=`（物理装配次数，稀疏化必变；字段余部 snapshots/too_short/invalid_seed/... 仍逐位）、`unresolved_targets`（候选 A 语义，C 保留可留——双跑前定）；**CKPT 侧信道验收关闭**（物理时机耦合，代码自注只写不判）。
- 首证钟对账唯一主键 = CERT `judge_at` 向量 + MISSED missed=0。
- stderr 进度行（elapsed）不在验收面。

## 6. 验证阶梯（双跑协议）

1. **Phase 0 插桩收拢**（先行，不改语义）：p122_replay_profile 已在 worktree（500k/2M 已测）——补测 trigger 精确次数、每 run 重估次数分布、run 分区/lower legs 固定税占比、L0 段数；用实测收拢 §2 的加速比区间（090：无此数据不承诺数字）。
2. **截段双跑**：`P116_MAX_BARS` 10 万（秒级迭代，专抓 dirty 漏判/归并乱序）→ 50 万 → 全量，三档 diff=0。
3. **shadow 强制对拍**：抽样 bar 强制全量重估 vs 缓存输出逐字相等（保留为长期回归开关）。
4. **强制前置签字**：`nearest_confirmed_center_idx`/`locate_pan_div_structure*`/`center_block_kind` 的隐式时间依赖逐一签字（架构候选报告列的遗留核验项）——未签字不实装。

## 7. 新 bin 形态

- `rust/src/bin/p123_fast_replay.rs`（autobins）：复用 P116_DUMP/P116_MAX_BARS/P116_CKPT env 协议与 dump 行格式逐字不动（双跑 diff 工具零改动）；模块头照 p116 格式写口径声明（含 §5 验收面与白名单）。
- 生产源码零改动（theta_v0/ 只读调用）；全部缓存在 bin 侧。

## 8. 风险登记

- dirty 判据完备性是全部证明责任：漏 dirty ⟹ 首证钟延后 ⟹ CERT judge_at diff（验收必抓到但定位贵）——§6.3 shadow 模式兜底 + §6.4 前置签字。
- 加速比区间 5–12× 未经 Phase 0 收拢，禁对外报点估计。
- C×B 叠加（~15–25× 算术组合）只在 C 落地且仍不达标时立项——届时先 Rc→Arc 独立换型（636 处/33 文件）单独双跑验收。
