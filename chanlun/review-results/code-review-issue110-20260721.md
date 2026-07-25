# Code Review — issue #110 投影层骨架 + 级别身份标签（20260721）

- Reviewer: orchestrator（两轴代码评审，只读）
- Scope: worktree `/tmp/kimi-nest-mainline` 未提交 diff 中 **#110 相关改动**：
  - `rust/src/theta_v0/classifier/projection.rs`（新增，123 行，untracked）
  - `rust/src/theta_v0/classifier/bsp.rs`（`BspPoint.level_origin` 新字段 + PartialEq 扩展）
  - `rust/src/theta_v0/classifier/mod.rs`（`pub mod projection`、`LevelState.level_projection`、两条分类路径 stamping + gate 构建、4 个新测试）
  - `rust/src/theta_v0/config.rs`（`ThetaConfig.level_projection` gate，默认关 + 默认关断言）
- Scope note：本 worktree diff 混有大量**其他票**的改动（wave-1 5a/5b 水位暴露面、nest_lifecycle、nest_index、v2/v3 等）。本评审只裁 #110 改动面；其余改动不在本报告评分内。
- 输入缺失：ticket 指定的 impl 报告 `chanlun/review-results/issue110-impl-20260721.md` **不存在**（见 Finding S-4）。
- 评审约束遵守：未改任何源码、未跑 cargo、未做任何 git 变更、主仓未触碰。

---

## 轴一：STANDARDS（仓库规范 + Fowler smells）

### S-1 [Low] Duplicated Code — 投影构建块两处逐字复制
- 位置：`rust/src/theta_v0/classifier/mod.rs:~453-467`（classify_impl）与 `mod.rs:~2124-2148`（classify_with_tower_incremental）
- 事实：`for point in &mut bsp { point.level_origin = level_idx as u32; }` stamping 循环 + `if config.level_projection.enabled { Some(LevelProjectionLayer::from_tower(level_idx as u32, Rc::clone(&bsp), tower_snapshots.last().expect(...))) } else { None }` 构建块，在全量与增量两条路径逐字重复（含同一 expect 文案）。
- 推理：两路径本已有"bit-exact 等价"契约，重复块将来一处改一处漏即分叉。可提取 `fn build_level_projection(config, level_idx, bsp, snapshot)` 小助手（零行为变化）。非阻塞。

### S-2 [Info] Speculative Generality — `CrossLevelConfirmationQuery` 当前零消费者
- 位置：`rust/src/theta_v0/classifier/projection.rs:62-109`
- 事实：`cross_level_query` 只产查询描述，无任何调用方。
- 推理：通常算 Speculative Generality，但 ticket #110 明文要求 expand 阶段落"跨级确认查询入口"骨架、#111 再接判定源，且模块头注释已声明此意图。**不判违规**——这正是 expand/contract 纪律的 expand 半步。已核验无发现（按票面豁免）。

### S-3 [Info] `BspPoint` 手写 `PartialEq` 扩入 `level_origin`
- 位置：`rust/src/theta_v0/classifier/bsp.rs:170-172`
- 事实：手写 eq 新增 `level_origin` 比较。
- 推理：核查 `classifier/mod.rs` 无 `dedup`/无按整点相等去重的生产谓词；两条路径对同级点 stamping 值恒一致（memo 命中路径共享已标注 buffer，miss 路径写入后才入缓存），等值判定结果不因新字段分叉。无第二查法、无判定改变。已核验无发现。

### S-4 [Medium] 过程物缺失 — impl 报告不存在
- 位置：`chanlun/review-results/issue110-impl-20260721.md`（缺）
- 事实：评审任务指定读取的实现报告在 worktree 中不存在（`ls chanlun/review-results/ | grep issue110` 为空；同目录其他票的 impl/implementation-card 均在）。
- 后果：`cargo test --release --lib` 1819/0 基线声明无任何书面 attestation 可查（本评审被硬规则禁跑 cargo，无法自证）。按"声明=能力，照实否定合格"纪律，此项记缺，转为 PASS 条件。

### S-5 其余 Fowler smells 清单
- Mysterious Name / Feature Envy / Data Clumps / Repeated Switches / Shotgun Surgery / Divergent Change / Message Chains / Middle Man / Refused Bequest：已核验无发现。
- Primitive Obsession：`level: u32` 已包 `LevelIdentity` newtype 化，且与 `ElementId.level`/`Classification.levels` 坐标系在文档中锚定——已核验无发现。
- 090（不得简化）：投影为 additive 旁路，未删未简化既有路径——已核验无发现。
- 备注（非发现）：SPEC #109 story 5 要求"完整级别身份（级别、源级别链、方向）"，本票 `LevelIdentity` 只含 `level`。ticket #110 验收只要求"级别身份信息"，骨架合票面；源级别链/方向留待 #111+，需在后续票补齐，勿在 #111 验收时遗漏。

---

## 轴二：SPEC（ticket #110 验收标准逐条）

### AC-1 LevelProjectionLayer 数据结构落地 — ✅ 已核验无发现
`projection.rs:72-77`：`identity: LevelIdentity` + `bsp_candidates: Rc<Vec<BspPoint>>`（与 `LevelState.bsp` 共享账本，测试 `Rc::ptr_eq` 锁不复制）+ `cross_level_query` 入口 + `time_span: LevelTimeSpanEstimate`（由本级输入塔元素跨度派生中位数，无固定时间窗口，合 SPEC "结构派生非固定"）。每级一实例：两路径均在 per-level loop 内构建，`tower_snapshots` push（mod.rs:405 / mod.rs:2039）先于构建点（mod.rs:~462 / ~2144），`last()` 即本级快照，下标同构不变量（mod.rs:351-352）成立。偶数样本中位数用 `lower + (upper-lower)/2` 防溢出，语义正确。

### AC-2 塔输出买卖点携带 level_origin — ✅ 已核验无发现
`bsp.rs:115-117` 新字段；`mod.rs:~453`（全量）与 `mod.rs:~2124`（增量 miss 路径，入 memo 前标注，命中共享不重扫）两处 stamping，值 = `level_idx as u32`，与 `Classification.levels`/`ElementId.level` 同坐标系。测试 `tower_output_bsp_level_origin_matches_classification_index` 断言逐级逐点相等。`from_tower` 的 `debug_assert!` 交叉锁层身份一致（debug_assert 非判定谓词，不违禁令）。

### AC-3 门关不构建（零开销 bit-exact 回归锁） — ✅ 已核验无发现（lib 层）
- gate 默认关：`config.rs:337` + 默认关断言 `config.rs:368`。
- 门关路径：`level_projection = None`，不 clone Rc、不扫塔、不分配（两路径注释与代码一致）。测试 `level_projection_gate_defaults_off_and_constructs_nothing` 锁全级 `is_none()`；`level_projection_gate_toggle_preserves_existing_level_outputs` 锁开/关两跑既有级输出一致（lib 层回归锁）。
- 注：`level_origin` stamping 为**无条件**（门关也写）。这是 AC-2 的要求（塔输出携带身份），O(点数) 写一个 u32，不改任何排序/去重/判定；runner 级门关 bit-exact（#76-T2/T3）属 SPEC 端到端 seam，不在本票验收面。

### AC-4 cargo test --release --lib 全绿零变红（基线 1819/0） — ⚠️ 未验证
本评审受硬规则约束不得运行 cargo；impl 报告缺失（S-4），无第三方 attestation。**照实记：未验证**，列为 PASS 条件。

### AC-5 禁第二查法：只改数据结构不改判定谓词 — ✅ 已核验无发现
`cross_level_query` 只产 `CrossLevelConfirmationQuery` 描述体（`level_origin`/`source_index`/`side`），"不执行查找、不持有判定结果"（projection.rs:62-63, 98）。全仓 grep `level_projection` 消费点：仅 config 接线 + 各 backtest 测试 fixture 的 `level_projection: None` 结构体字面量补齐——admission.rs/nest.rs/runner.rs 判定路径零消费。跨级链判定唯一来源（nest.rs n_delta 核）未被本票触碰。

---

## 结论

**VERDICT: PASS WITH CONDITIONS**

代码本体两轴均干净：数据结构、双路径 stamping、gate 纪律、禁第二查法全部合票面，测试覆盖四象限（默认关/开构建/身份对齐/开关不扰既有输出）。条件：

1. **[必须] AC-4 补 attestation**：由实现方或验收方在允许跑 cargo 的工位执行 `cargo test --release --lib`，确认 1819 基线全绿零变红，并落书面记录（补交 `issue110-impl-20260721.md` 一并解决 S-4）。
2. **[建议] S-1**：合并两处逐字重复的投影构建块为单一助手函数（零行为变化，可并入 #111）。
