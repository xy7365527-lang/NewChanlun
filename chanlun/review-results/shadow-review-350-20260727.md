# 影子评审报告：#350 restore fixup 同类时序孔（工作区改动，未 commit）

- **评审票**：#350（Part of #59，#347 影子评审 LOW-3 浮出；对应 `shadow-review-315-20260727.md` T8/LOW-3）
- **对象**：`rust/src/theta_v0/strategy/coverage.rs` + `rust/src/theta_v0/backtest/runner.rs` 工作区未提交改动
- **worktree**：`/tmp/kimi-nest-mainline`，HEAD=`19aea33a26`，禁止 git mutation（交付停在工作区）
- **日期**：2026-07-27
- **评审方式**：两轮 `/code-review`（Standards 轴 + Spec 轴各自独立并行子代理，opus，未共享上下文），
  轮间按浮出问题修复后复评

## 0. 结论

**核实结论已从"孔不存在"订正为"孔存在，已修复"。** 第一轮 code-review（针对"只核实、结论孔不存在"
版本）的 Spec 轴评审员指出初版测试方向搞反——构造的是"父由更早一次调用物化"而非 issue 描述的"父由
更晚一次调用物化"，据此推理出真实可达的触发路径（见 §1）。写 RED 测试实测坐实（`cargo test` 实跑
FAILED）后按 #315 同形状修复；第二轮 code-review 对修复后的 diff 复评，浮出 1 项硬性（probe 声明与
实际不一致）+ 若干判断性问题，已在本次一并修复。**这一曲折过程本身就是结果的一部分**——不抹平为
"一次就得出正确结论"的叙事，符合诚实记录原则。

## 1. 真实时序孔（第一轮 Spec 轴浮出，实测坐实）

- **触发路径**：非 issue 字面"父由另一次 restore 调用物化"，而是"父由同 bar 更晚一次**非 restore 的
  直接 push**（`Closed|Invalidated`+`is_boundary_root` held 腿分支）物化，而该父此前已被一次更早的
  restore 调用尝试解析、因 registry 侧 `invalidated` 导致 `registry_lost` 断链"——与 issue 字面所指
  相关但不完全等同，同属 #247 缺口类。
- **RED 证据**：`restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`
  修复前实跑 `left: Ambient, right: Ambient`（`assert_ne!` panic，即修复前 role_v 恒为 Ambient）。
- **GREEN**：同一测试修复后 `role_v==ShortDiff`、`q_units==300`。

## 2. 修复形状（按 #315 同形状）

`restore_ancestor_chain_from_registry` 不再在函数内自行修补 `parent`/`attached_dir`——本轮新 push 的
idx 追加进调用方持有的累加器 `pending_parent_fixup`（与 held-leg 占位共用同一累加器，原名
`placeholders` 已改名），由 `coverage_step_from_buckets_sep` 在两个物化循环（prev_active held 腿 +
open 候选父链恢复，含循环内触发的全部 restore 调用）**全部结束后**统一执行 fixup。生产循环与测试
helper 共用同一新抽出的 `resolve_pending_parent_fixups`（消除两轮评审均指出的重复逻辑）。

## 3. 第二轮 code-review 浮出与处置

| # | 发现（评审轴） | 处置 |
|---|---|---|
| H1 | probe 字段/报表标签（`AncokProbe::restore_break_registry_lost` 文档 + `coverage.rs:77-78/98-99` + `runner.rs` 697 ceiling 判据文档 + `eprintln!` 标签）仍声称 `restore_break_registry_lost>0 ⟹ 声部树非严格"——已被 #350 自己的修复+测试推翻（Standards H1，Spec (c)-1，最重） | 已订正：三处文档改为"真实判据是 `placeholder_parent_unresolved − placeholder_pruned_by_ancok`"；`runner.rs` 697 ceiling 测试标签同步更新 + 补打印差值 |
| J1 | 测试 helper `resolve_pending_parent_fixup` 与生产 fixup 循环逐行同构，重复维护（Standards），四个既有 restore 测试断言的是 helper 行为而非生产行为（Spec c-3） | 抽取 `resolve_pending_parent_fixups`，生产/测试共用同一实现；测试 helper 改为委托调用 |
| J2 | 新 #350 测试无 probe 增量断言（Standards） | 已补：`restore_break_registry_lost>=1` + `unresolved==pruned` 交叉核对 |
| — | 缺 shadow-review 文件（Spec (a)-1，issue 验收项） | 本文件 |
| — | 「恒」措辞核查范围过窄（Spec (c)-1，只查了新增代码，未查姊妹措辞） | 已扩大：`AncokProbe` 结构体头文档 + 字段文档 + `runner.rs` 697 ceiling 文档三处一并订正 |
| nit | `///累加器` 缺空格、繁体字"統一" | 已修 |

未处置（判断性，记录不阻塞）：
- 文档四处近似重复时序孔成因叙述（Standards J3，同 `shadow-review-315` LOW-2 性质）——跨函数头/调用点/
  测试 docstring 各有本地读者，暂不收敛，与 #315 先例一致处置。
- `work/raw/id_idx/overlay_seen/pending_parent_fixup` 五参数 Data Clump（Standards J4）——超本票范围，
  值得未来聚为物化上下文结构体，暂不在本票展开（no-patch 与 speculative-generality 的平衡：本票范围
  是时序孔，非架构重构）。
- 既有历史文档（`TARGET_STRATEGY_MAXFULL.md`、旧 `.chanlun/review-results/*`）中的行号引用漂移——历史
  快照不回改。

## 4. 复跑

| 项 | 结果 |
|---|---|
| `cargo test --release --lib restore_` | 12 passed / 0 failed（含 2 个新增 #350 测试） |
| `cargo test --release --lib` | **1889 passed / 1 failed / 133 ignored**，唯一失败 `extract_signals_bit_exact_digest_guard`（#115 线在案，未碰未归因） |
| `cargo build --release --lib --tests` | 无 error（含 `#[ignore]` 的 697 ceiling 测试同步编译通过） |
| diff 面 | `coverage.rs` + `runner.rs` 两文件，均在 #350 授权范围内（runner.rs 改动仅限 697 ceiling 判据文档订正，无生产逻辑变更） |

## 5. 结果包六要素

1. **结论**：#350 核实确认孔存在（触发路径与 issue 字面描述相关但不完全同构，实测坐实），已按 #315
   同形状修复；修复引出的 probe 语义扩展导致 3 处历史文档失准，已订正；生产/测试 fixup 逻辑重复已
   抽取消除。
2. **定义依据**：#350 票体（"存在 → 按 #315 同形状修"分支）；#315 先例 `shadow-review-315-20260727.md`
   T8/LOW-3（本票起点）；#247 缺口二裁定（角色输入丢失的定义）；090 `no-patch-mentality.md`（声明与
   实际一致）。
3. **边界条件**：结论翻转条件——(a) 若未来改动在 `pending_parent_fixup` 统一 fixup 与
   `ancestor_close_by_id` 之间新增任何 `raw.push`，本票的"父必在统一 fixup 时点终态可见"论证失效；
   (b) 若 `ancestor_close_by_id` 改用 `parent`（idx）而非 `parent_id`（结构）判据，本票坐实的孔的
   触发条件改变；(c) 697 ceiling 真实判据（`unresolved−pruned`）若在真实 BTC 数据观测到 >0，则孔在
   生产窗口被命中，须启动建议6 三选一严格修复（`ancok_l2_ceiling_exposure_real_btc`，`#[ignore]`
   默认不跑）。
4. **下游推论**：命中本票场景（同 bar 内 registry 侧 invalidated 祖先经更晚 boundary-root 直接 push
   物化）的窗口，其 p̃ 计算会因本修复改变（本票未附生产对拍，场景在已测窗口未观测——诚实记录，同
   `shadow-review-315` 惯例）；`restore_break_registry_lost` 探针不再单独作为 697 ceiling 判据，
   已订正为旁路诊断。
5. **谱系引用**：090（声明与实际一致）；#284 MED-1（#315 起点，本票同形状延续）；#247 缺口二（形状
   来源）；#301 探针族（probe 惯用形状）；`shadow-review-315-20260727.md` T8/LOW-3（本票来源）。
6. **影响声明**：`coverage.rs`（`restore_ancestor_chain_from_registry` 签名 + 3 处文档 + 2 处测试文档
   + 新增 2 测试 + 抽取 `resolve_pending_parent_fixups`）、`runner.rs`（697 ceiling 测试文档 + 报表标签，
   无生产逻辑改动）。工作区未 commit，交付停在工作区。
