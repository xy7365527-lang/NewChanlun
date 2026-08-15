# #984 N7 LEE 对接门（Consume_ℓ 确定性路由）收尾评审报告

- 票：<https://github.com/xy7365527-lang/NewChanlun/issues/984>
- 日期：2026-08-15
- worktree：`/tmp/wt-n7-lee`（`git worktree add --detach ... main`，base = `1cfaefe8c3`）
- 变更面：`rust/src/theta_v0/strategy/consume_router.rs`（新增）+ `rust/src/theta_v0/strategy/mod.rs`（+2 行注册）
- 判定链：`consume_at.rs` / `level_ledger.rs` / `SepLeg` / 递归塔 **零改**

## 一、Spec 轴（逐条对照票面验收）

### Spec-1 `cargo check --lib` + `cargo check --features backtest_bin` 绿

- `cargo check --lib`：**通过**（exit 0；61 条 warning 全部为仓库既有 dead_code/命名告警，无 consume_router 相关、无 error）。
- `cargo check --features backtest_bin`：**通过**（exit 0；无 error、无 consume_router 相关告警）。

### Spec-2 判定链零改

`git status --porcelain` 仅两项：

```text
 M rust/src/theta_v0/strategy/mod.rs
?? rust/src/theta_v0/strategy/consume_router.rs
```

`git diff --name-only`（已跟踪）＝ `rust/src/theta_v0/strategy/mod.rs`；新增文件 `consume_router.rs` 未跟踪故不进 `git diff`。两文件合计＝票面允许面：`consume_router.rs`（新增）+ `strategy/mod.rs`（`pub mod consume_router;` 注册）。`consume_at.rs` / `level_ledger.rs` / `SepLeg` / 递归塔 零改（diff 内无任何这些文件）。

### Spec-3 单文件测试全绿（4/4）

`cargo test --lib theta_v0::strategy::consume_router::`：

```text
running 4 tests
test ...::route_empty_input ... ok
test ...::level_invariant_holds_on_routed_ledger ... ok
test ...::route_is_deterministic ... ok
test ...::route_buckets_by_formation_level ... ok

test result: ok. 4 passed; 0 failed
```

| 测试 | 验收点 | 断言方式 |
|---|---|---|
| `route_buckets_by_formation_level` | 分桶正确性 | level ∈ (1, 2, 3) 各若干，`bsp_at(level)` 内容逐项（`key.level` + `created_at`）断言，`total()==7`，`levels()==[1,2,3]` |
| `route_is_deterministic` | 确定性 | 同一批 creations 两次 `route_consume` → `assert_eq!(a, b)`（BTreeMap 保序 + 插入序） |
| `level_invariant_holds_on_routed_ledger` | 分桶不变量 | `level_invariant_holds()` 为真；空账本真空成立 |
| `route_empty_input` | 空输入 | 空 creations → `total()==0`、`levels()` 空、不变量真 |

测试构造全部用现役对象公开字段 struct literal：`ManagedBsp{key, created_at}`（consume_at.rs:65-67）、`BspStructuralKey{rule_version, level, parent, side, class, anchor}`（bsp_bridge.rs:149-158）、`ParentFingerprint{center_start, zd, zg}`（cand_event/key.rs:32-37）。**零 mock 私有**。

### Spec-4 `cargo fmt --check` 绿

通过（exit 0；首次 check 报 1 处换行 diff，已 `cargo fmt` 归一后复跑绿）。

## 二、Standards 轴（逐条对照票面验收）

### 反模式三条

1. **实现耦合**：只读现役公共字段。路由只读 `c.bsp.key.level`（consume_router.rs:60）——`ManagedBsp.key`（consume_at.rs:66，pub）→ `BspStructuralKey.level`（bsp_bridge.rs:151，pub）。不 mock 现役对象私有、不测私有路径、不改 bsp_bridge / chain_cert / parser / 递归塔。
2. **同语反复**：测试断言分桶内容逐项（`key.level` + `created_at` 逐项），非「喂 X 得 X」——`route_buckets_by_formation_level` 逐桶逐项断言，且确定性/不变量/空输入为独立性质的独立断言。
3. **水平切片**：只做「确定性路由」一层（Consume_ℓ 分桶）。不碰 sizing（M2/M4）、fill、policy 级别族过滤（票面明写「可」为可选扩展）、声部腿接线（SepLeg）。

### 模块头三件

模块头（consume_router.rs:1-7）写：
- **seam**：`Consume_at 产出 → 确定性路由（按 BspStructuralKey.level 分桶）→ 受管 BSP 级别持有账本`；
- **认识论等级**：L1（纯分桶，确定性路由，零信息增量，不声明 alpha）；
- **冻结语义来源**：LEE 设计 §C.3 支柱 3（`multi-level-native-execution-design-20260719`）。

## 三、承重断言 file:line（逐字）

1. **分桶键 = `BspStructuralKey.level`（≡ formation_level）**：
   - `rust/src/theta_v0/strategy/consume_router.rs:60` — `.entry(c.bsp.key.level)`（route_consume 体内逐字）。
   - `rust/src/theta_v0/classifier/bsp_bridge.rs:151` — `pub level: u32,`（BspStructuralKey 字段逐字）。
   - `rust/src/theta_v0/classifier/consume_at.rs:66` — `pub key: BspStructuralKey,`（ManagedBsp 字段逐字）。
2. **分桶不变量机器化**：
   - `rust/src/theta_v0/strategy/consume_router.rs:49` — `.all(|(&lvl, bsps)| bsps.iter().all(|b| b.key.level == lvl))`（逐字）。
3. **冻结语义来源（票面引用点，设计文档原句）**：`git show 2330ea6a3e:chanlun/review-results/multi-level-native-execution-design-20260719.md:83` — 「LEE 在其后加一步确定性路由：`ManagedBspCreation` 按其 `BspKey.formation_level` 投入对应 Ledger_ℓ；…**LEE 不改变 `Consume_at` 的生产语义，只改变其输出的持有方式**——这是对 E2E 架构的纯下游扩展，不触碰八道 E2E-S* 缝合线（roadmap:79）。」；同文件 `:119` — 「LEE 的消费路由只读这两个既有字段，不新增口径」。

## 四、边界声明（不擅自扩）

最小面按票面落地为纯路由：`route_consume` 只做 `BspStructuralKey.level` 分桶，无 policy 级别族过滤、无 sizing/fill、无声部腿接线。该最小面**不含不确定性**（BTreeMap 键序 + Vec 插入序 ⟹ 同输入 bit-exact 同输出，测试 `route_is_deterministic` 机器化锁定），故无需停下上报。

## 五、命令回放（父会话独立复跑用）

```bash
cd /tmp/wt-n7-lee/rust
cargo fmt --check
cargo check --lib
cargo check --features backtest_bin
cargo test --lib theta_v0::strategy::consume_router::
```
