# bughunt-fix-v2 修复报告（2026-07-10）

## 基线与范围

- 工作分支：`bughunt-fix-v2`
- 基线：`bf1854d719603d0e257f5998978bf279c8f4c08b`
- 禁止事项：未 push；未改 `STRICT-NEST-CHECK.md`、prereg、`chanlun/goals/`、witness、bit-exact 基线及其他 A 节冻结资产。
- 审查输入：本自包含对象库未在可达树中携带 `chanlun/review-results/bughunt-verify-20260710.md` 文件本体；范围按任务给出的 A/B 边界、源分支提交谱系与旧修复对象交叉核对。只重做任务指定的四项 B 节修复。

## 7 个指定 cherry-pick

按指定顺序执行；下表左列为源提交，右列为本分支新提交。

| 顺序 | 源提交 | 本分支提交 | 结果 |
|---:|---|---|---|
| 1 | `0baaeb301a` | `fc5be83a9b` | 无冲突 |
| 2 | `3e081d07ab` | `d88190130e` | 无冲突 |
| 3 | `238397321b` | `8a0a9e4777` | 无冲突 |
| 4 | `3ea504c185` | `f681bbf681` | 无冲突 |
| 5 | `6ea2ce8ebc` | `f7c9a141ba` | 无冲突 |
| 6 | `0331f0071c` | `6ae6f7be2b` | `rust/src/bin/strict_nest_check.rs` 一处冲突，最小解决 |
| 7 | `fe76289e3e` | `578dd3cf3a` | 无冲突 |

第 6 项冲突原因：源提交的 F-07 测试位于其父提交已加入的 F-04/F-05/F-08 测试之后，而本任务刻意未 cherry-pick 该父提交。处理方式是只保留 `0331f0071c` 自身新增的 F-07 漏斗归因测试，不带入父提交的 F-04/F-05/F-08；F-06/F-07 生产改动保持原意。提交 message 尾部已记录该处理。

## 四项重做

### 1. backtest F-05

提交：`a6fb34f804`

- `FillOutcome` 明确分离 `closed_qty`、`opened_qty`、`rejected_qty` 与总真实成交量。
- `n_orders_executed`、交易轨迹、`voice_qty` 和 `held` 只消费真实成交段，不消费请求量。
- 回归反例覆盖：买入翻转单请求 2 手，先平空 1 手真实成交，开多余量 1 手因现金不足拒绝；结果只计一个执行订单、声部账归零，拒绝余量不生成新仓。
- 相关回归：runner 模块 `49 passed, 0 failed, 11 ignored`。

### 2. cert F-01

提交：`1f1242e905`

- `NestCertificate`、`NestRung` 字段全部私有化。
- `NestRung::new` 与 `NestCertificate::from_parts` 降为 crate 内可见，内部最终落成统一经过私有 builder。
- 对外唯一证书生产路径为基于真实 `CandDeltaEvent` 的 `assemble_certificate` / `assemble_certificates`；只读 getter 保留。
- 两个 `compile_fail` doctest 分别锁死字段字面量伪造与外部调用 `from_parts` 旁路。
- 相关回归：nest 模块 `33 passed, 0 failed`；F-01 doctest `2 passed, 0 failed`。

### 3. cert F-05

提交：`c5813d2cfa`

- 删除 `trades.jsonl` 与 OHLC/date 数组的手写 token 扫描。
- 改用 serde 类型化解码；字段类型、数组语法、数值边界由 JSON/Deserialize 统一校验。
- 文件级回归明确拒绝 `[1 2]` 与 `[1,,2]`。
- 相关回归：`strict_nest_check` `5 passed, 0 failed`；另验证 `cargo build --release --bin strict_nest_check` 通过。

### 4. parser BUG-11

提交：`e514a8158b`

- 旧实现 `round(area / quantum)` 会让不同面积碰撞，并以 `+quantum` 容差冒充 `mono` 原公理；仅校验 quantum 不能解决该问题。
- 新实现将非负有限 `f64` 面积用 IEEE-754 `to_bits()` 精确序嵌入 `u64/Nat`；删除碰撞量化和容差窗。
- `mono` 恢复为精确命题：`strength(a) <= strength(b) -> area(a) <= area(b)`；`faithful` 同样无近似窗。
- 回归反例 `1.2 > 1.1` 必须映射为严格更大的 strength。
- 相关回归：force-conformance 模块 `7 passed, 0 failed`。

## 全量回归

最终新鲜执行：

```text
cd rust
cargo test --release
```

结果：exit 0。

- 主库：`1541 passed, 0 failed, 127 ignored`
- `strict_nest_check`：`5 passed, 0 failed`
- 集成测试：`43 passed, 0 failed, 3 ignored`
- doctest：`2 passed, 0 failed, 3 ignored`
- 合计：`1591 passed, 0 failed, 133 ignored`

全量首轮暴露两个与四项修复无关的测试卫生问题，已用独立提交 `64e19ad651` 最小处理：纯数学公式的 rustdoc 代码块改标 `text`；OPSEM dump 测试固定临时目录改为进程/时间唯一目录。两项均不改生产判据。

## 剔除与待裁定

- `BUG-03`：触及 A 节冻结判据，剔除；未 cherry-pick `1f7dd342e5`。
- `F-11` / `F-13`：触及 `chanlun/goals/` 等 A 节冻结范围，剔除；未 cherry-pick `8de8441bab`。
- `F-15` / `F-16`：触及 ceremony/冻结裁定范围，剔除；未 cherry-pick `aa9bac3ee2`。
- `BUG-10`：按任务要求等待人工裁定；未 cherry-pick `8911c647f8`。

## A 节边界审计

基线到报告前代码头的变更仅涉及 Rust/Cargo 源码与测试文件；路径审计未发现 `STRICT-NEST-CHECK.md`、prereg、`chanlun/goals/`、witness 或其他冻结基线文件进入 diff。报告文件本身仅新增于 `chanlun/review-results/`。
