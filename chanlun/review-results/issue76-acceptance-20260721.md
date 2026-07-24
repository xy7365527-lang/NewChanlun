# #76 出场门真链切换·验收报告

- 日期：2026-07-21
- 验收工位：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717），本工位独占 cargo
- 验收对象：`issue76-impl-20260721.md`（交付报告）
- 纪律：主仓零写 ✓；禁 git mutation（仅 git diff/status 只读）✓；未改 rust/Cargo.toml ✓；090 照实 ✓

## 最终判定：**通过**（两处低严重度文档瑕疵，不影响判定正确性，见 §4）

---

## 1. 代码锚核验（声明 vs 现实）

### exit.rs（rust/src/theta_v0/strategy/exit.rs）

| 声明锚 | 实际 | 判定 |
|---|---|---|
| :40-53 模块头接线状态段（#76 升格登记） | :40-52，内容逐条吻合（typed 真链注入、miss 不准出、v0 退役对照） | ✓ 无漂移 |
| :158-181 `exit_decision_for_nested_cert`，第 8 参 `reverse_cert: &mut dyn FnMut(&VoiceDecision) -> bool` | 函数本体 :167-178（doc :150-166），第 8 参在 :175；声明范围覆盖 doc 尾部+签名 | ✓ 微漂移（约 -9），语义吻合 |
| :197 `reverse_nest_cert_base`（文档改写对照读出、实现零改动） | 精确命中；对照读出文档 :180-196，实现 :197-200 | ✓ 精确 |
| :207-216 `exit_decision_impl` 签名 `Option<&mut dyn FnMut...>` | 精确命中，参数在 :215 | ✓ 精确 |
| :235-239 反向项行 `reverse_signal(...) && match reverse_cert.as_mut() { None => true, Some(q) => q(d) }` | 精确命中（:235-239） | ✓ 精确 |
| 测试 #76-T1 `reverse_cert_gate_typed_is_sole_decision_source` :721 | 精确命中（v0 准真链拒 ⟹ None；v0 拒真链准 ⟹ Some，两方向对照） | ✓ 精确 |
| 测试 #76-T2 `reverse_cert_gate_query_only_on_reverse_candidates` :747 | 精确命中（同向候选零查询调用短路锁） | ✓ 精确 |
| NG-①②③ 调用点补第 8 参 `&mut reverse_nest_cert_base` | NG-① :638/:652/:657、NG-② :682 均已补；断言语义未见回退 | ✓ |

### runner.rs（rust/src/theta_v0/backtest/runner.rs）

| 声明锚 | 实际 | 判定 |
|---|---|---|
| :1540-1613 `ExitNestGateStats` + `report`（NEST_GATE_EXIT，门关不输出） | doc :1538 起，struct :1543-1562，impl 至 :1616；total/admitted/typed_found/typed_none/flat_dir/链深构成/cross 三列齐备，report 在 :1595-1615 | ✓ 微漂移（+2~3），语义吻合 |
| :1615-1696 `ExitNestGateCtx`；`sync_bar` :1655、`reverse_admit` :1676 | doc :1618 起，struct :1636，impl 至 :1698；sync_bar 精确在 :1655，reverse_admit 精确在 :1676 | ✓ 范围微漂移，内锚精确 |
| v1：ctx 构建 :4134 | 精确（`nest_cert_gate_enabled() && n > 0`，门关 None） | ✓ 精确 |
| v1：`sync_bar` :4262 | 精确 | ✓ 精确 |
| v1：注入闭包调 `exit_decision_for_nested_cert` :4275-4282 | 精确；None 分支 :4282 走 `exit_decision_for`（门关原路径） | ✓ 精确 |
| v1：`report("v1")` :4309 | 精确 | ✓ 精确 |
| dual：ctx 构建 :4506 / `sync_bar` :4670 / 注入闭包 :4684-4690 / `report("dual")` :4750 | 全部精确；dual 注入路径传 `parent_invalid_at(&held, depth)` 实值，与 None 分支 :4691 同参 | ✓ 精确 |
| 测试 #76-T1 `exit_gate_reverse_admit_typed_hit_miss_flat` :7248 | 精确（命中/miss/Flat/链断/depth>0 五分支 + 统计分账） | ✓ 精确 |
| 测试 #76-T2 `exit_gate_v1_reverse_exit_suppressed_without_cert` :7308 | 精确（门关 == 基线 bit-exact；门开压掉反向退出 ⟹ 延至强平） | ✓ 精确 |
| 测试 #76-T3 `exit_gate_dual_off_bitexact_on_noop_when_no_reverse_fire` :7384 | 精确（dual 门关 bit-exact + 门开零操作回归） | ✓ 精确 |
| 冒烟 `nest_exit_gate_smoke_real_btc` :7413（#[ignore]） | 精确 | ✓ 精确 |
| 签名零改动（run_theta_v0 / plan_and_fill_mtm / plan_and_fill_mtm_dual 及既有调用点） | `git diff` 中三函数签名行改动数 = 0 | ✓ |

## 2. 红线复核

### ① 门关短路语义 — ✓ 通过

- `exit_decision_for`（exit.rs:128）与 `exit_decision_for_nested`（exit.rs:147）均向 `exit_decision_impl` 传 `None`。
- 反向项 :236-239 `match reverse_cert.as_mut() { None => true, Some(q) => q(d) }`：None 分支不求值任何证书谓词。
- grep 全文件确认 `reverse_cert` 在 `exit_decision_impl` 内的唯一使用点为 :236——无第二处谓词求值。
- runner 侧门关 ⟹ `exit_gate=None` ⟹ 无 sync_bar（:4261/:4669 条件分支）、无 report（:4308/:4749 条件分支）、退出判定走原函数（:4282/:4691）。
- 门关逐字节不变由 runner #76-T2（v1，n_orders/equity/realized 三项 bit-exact）与 #76-T3（dual）锁死——两测试本次实跑均通过。

### ② 禁第二查法 — ✓ 通过（一处报告列举不全，见 §4-B）

- 判定唯一来源：runner.rs:1432 `cert.certificate().n_delta()`（`NestChainGate::typed_lookup` 内，nest.rs 递归核）。
- `reverse_admit`（:1690-1691）只消费 `typed_lookup` 的返回，自身不触碰证书内部。
- 装配复用 `sync_events`(:1658) / `has_bridge_key`(:1665) / `sync_index`(:1668) / `typed_lookup`(:1690)——四方法均被 π 进场门既有代码使用（:1477/:2386/:2398/:2401），非 #76 新增，无新查询路径。

### ③ miss ⟹ false + v0 只进对照 — ✓ 通过

- :1691 `typed.map(|(p, _)| p).unwrap_or(false)`——miss 与 n_delta=false 均 ⟹ false，无 fallback。
- v0 基例 `reverse_nest_cert_base` 仅在 :1684（Flat 分支）与 :1693（depth=0）为 `stats.observe` 的对照读出而调用；`pass` 的计算不依赖 v0。
- cross 三列（agree / v0_pass_typed_rej / v0_rej_typed_pass）仅落账，不进判定——exit.rs #76-T1 两个方向（v0 准真链拒 / v0 拒真链准）构造性锁死。

### ④ nest.rs / level_view.rs / nest_index.rs / nest_lifecycle.rs 相对 #76 零改动 — ✓ 通过（归因方法照实标注）

- nest_index.rs / nest_lifecycle.rs 为 untracked 新文件（#74/#78 产物）；对四文件 grep `#76 / reverse_admit / ExitNestGate / NEST_GATE_EXIT / reverse_cert / exit_decision_for_nested_cert`：**零匹配**。
- nest.rs / level_view.rs 的 git diff 中无任何 exit/门相关行；`fn n_delta` 定义行在整个未提交 diff 中改动数 = 0。
- 归因限制照实：全部改动未提交、混于同一 diff，无法按票精确切分；上述结论基于标识符/关键词证据，强度足够（#76 授权文件为 exit.rs/runner.rs，四文件若被 #76 触碰必留上述标识符痕迹）。

## 3. 测试复核（本工位实跑）

本票 5 个新测试单独实跑（release）：

```
running 5 tests
test theta_v0::strategy::exit::tests::reverse_cert_gate_query_only_on_reverse_candidates ... ok
test theta_v0::strategy::exit::tests::reverse_cert_gate_typed_is_sole_decision_source ... ok
test theta_v0::backtest::runner::tests::exit_gate_reverse_admit_typed_hit_miss_flat ... ok
test theta_v0::backtest::runner::tests::exit_gate_v1_reverse_exit_suppressed_without_cert ... ok
test theta_v0::backtest::runner::tests::exit_gate_dual_off_bitexact_on_noop_when_no_reverse_fire ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1934 filtered out; finished in 0.01s
```

全量 `cargo test --release --lib` 尾行原文：

```
test result: ok. 1807 passed; 0 failed; 132 ignored; 0 measured; 0 filtered out; finished in 0.92s
```

与交付报告声明（1807 passed / 0 failed / 132 ignored）**逐字一致**。

## 4. 发现的问题（按严重度）

- **低 A｜过期注释**：exit.rs:692-693 NG-③ 注释残留旧术语「既有三入口全部委托 **gate=false**」——第 8 参已从 `nest_cert_gate: bool` 升格为 `reverse_cert: Option<...>`，字面过期（语义等价于 reverse_cert=None）。建议后续票顺手改写，不阻塞。
- **低 B｜报告列举不全**：交付报告§红线称「装配只复用 NestChainGate 既有方法（sync_events/sync_index/typed_lookup）」，实际 `sync_bar` 还调用 `has_bridge_key`（:1665，惰性索引重建的前置判据）。该方法同为 #75 既有且被 π 进场门 :2398 使用，**不构成第二查法、非违规**——属报告文字列举不全。
- **信息｜非缺陷**：门开冒烟读数 typed_found=0（0/35 准出），真链准出案例在本截断窗无法评价——报告已照实登记为后续观察项，与 #75 链稀薄背景同源。

## 5. 未决项逐条评价

1. **π 生产路径出场不经 exit.rs**：本票范围 = 票指的 v1/dual 两退出循环（deprecated 诊断路径），π 路径无 `exit_decision_for_nested_cert` 消费点属结构事实，**不影响本票验收**；π 路径真链化确属后续票，登记合理。
2. **dual depth-0 反向项合成夹具结构性难达**：**不影响验收**。dual 门关 bit-exact 由 #76-T3 锁定（本次实跑通过），门开行为差由 T3 零操作回归 + 实数据冒烟（3 例触发）覆盖，语义由共享单测（exit.rs #76-T1/T2、runner #76-T1，同一函数同一闭包形态）锚定——是既有流程结构的照实登记而非测试缺口。
3. **链深构成读数全零**：观察项，**不影响判定正确性**——miss ⟹ false 路径已被 runner #76-T1 (b)(e) 构造性覆盖；链深构成评价待 typed 命中非零窗口，属后续观察。
4. **门开臂内嵌 IncrementalClassifier 重算**：性能项。门关零开销（ctx 仅门开构建，:4134/:4506），诊断 opt-in 路径的一次性成本，**不影响验收**；门开常态化时再议共享即可。

## 6. 结论

交付报告的全部代码锚（exit.rs 7 处 + runner.rs 14 处）核验吻合（仅 3 处范围微漂移，内层锚点全部精确）；四条红线全部通过；5 个新测试与全量 1807 passed / 0 failed 实跑复现。两处低严重度文档瑕疵（过期注释、列举不全）不触及判定正确性。

**判定：通过。**
