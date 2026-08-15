# issue983 N7 consume_at 逻辑实装 — 实装报告（2026-08-15）

票面：`gh issue view 983`（N7 consume_at 逻辑实装，map #529）。
改动文件：`rust/src/theta_v0/classifier/consume_at.rs`（唯一，见 `git diff --name-only`）。

## 一、Spec 轴（逐条对照票面验收）

### S1. `cargo check --lib` + `cargo check --features backtest_bin` 绿

- `cargo check --lib`：绿（Finished dev profile）。
- `cargo check --features backtest_bin`：绿（Finished dev profile）。

### S2. 判定链零改

`git diff --name-only` 输出仅一行：

```
rust/src/theta_v0/classifier/consume_at.rs
```

bsp_bridge / chain_cert / parser / 递归塔 / LEE / S8 零触碰。

### S3. 单文件测试全绿（`cargo test --lib theta_v0::classifier::consume_at::`）

`running 13 tests / test result: ok. 13 passed; 0 failed`。逐条：

| 测试 | 覆盖验收项 |
|---|---|
| `truth_table_creates_per_bsp_and_links_per_rule` | 真值表：creation 数 = 唯一 bsp 数（2）、link 数 = bsp 数 × rule 数（4），字段逐项（key/side/written_at/lineage） |
| `idempotent_prior_bsp_skips_creation_but_still_links` | 幂等：prior 已含 bsp → 不产 creation、仍产 link |
| `idempotent_prior_link_is_not_reproduced` | 幂等：prior 已含 link → 该 link 不重复产 |
| `defensive_dedup_same_bsp_from_two_nodes` | 防御性去重（同一 bsp 被多节点命中只产一次） |
| `non_closed_lineage_returns_no_consumption` | 非 Closed → NoConsumption |
| `closed_after_as_of_returns_late_authorization` | closed_at > as_of → LateAuthorization |
| `empty_rules_returns_invalid_policy` | 空 rules → InvalidPolicy |
| `duplicate_projection_rule_returns_invalid_policy` | 重复投影 → InvalidPolicy |
| `prior_bsp_ahead_returns_inconsistent_state` | prior bsp 超前 → InconsistentState |
| `prior_link_ahead_returns_inconsistent_state` | prior link 超前 → InconsistentState |
| `closed_chain_with_all_missing_events_yields_empty_output` | 空产出：event_to_bsp 全查无 → `Ok((空, 空))` |
| `consume_error_is_copy` | ConsumeError 是 Copy |
| `consume_at_has_frozen_signature` | 签名机器锁（6 参数，含 event_to_bsp） |

测试构造全部用现役对象公开字段/公开构造点（`CandidateKey`/`BspStructuralKey`/`ChainKey`
struct literal、`ChainKey::new`、`ParentFingerprint`），未 mock 私有、未测私有路径。

## 二、Standards 轴（逐条对照票面验收）

### `cargo fmt --check`

绿（`cargo fmt` 后 `--check` 无 diff）。

### 无反模式

- **实现耦合**：`consume_at` 只读现役对象公共字段（`TowerChainCertificate.status/closed_at/key/nodes`、
  `ChainNodeTrace.key/status`、`BspStructuralKey.side`），不 mock 私有、不测私有。
- **同语反复**：删掉 #981 的 `*_constructs_and_fields_read`「喂 X 得 X」式构造回读测试；新测试
  断言**派生产出**逐字段（creation.key 集合、created_at==as_of、link.key 集合、link 的
  lineage/written_at/side 逐项对照独立期望值），非「输出等于输入」。
- **水平切片**：本票只做 consume_at 逻辑一层；未接 LEE/S8、未改任何调用方（模块内无其他调用方）。

### 模块头更新

- seam 已从「冻结签名 stub」更新为「真逻辑」（Closed 门 + 幂等创建 + BspLink 组内写入 +
  四错误分支）。
- 四错误分支模糊判据照实 TODO 注释（`consume_at.rs` 函数体内）：

```
// 2. LateAuthorization：链在本次 as_of 之后才闭合 ⟹ 本次 as_of 时链尚未可消费。
// TODO(精确判据待冻结语义细化)：本版 = closed_at > as_of 最直白推导。
...
// 4. InconsistentState：prior 状态超前于本次 as_of。
// TODO(精确判据待 E2E-O 修订协议落地后细化)：本版 = 超前条目检查。
```

## 三、签名订正（#982 A）与逻辑 spec 落地核对

- 签名加 `event_to_bsp: &HashMap<CandidateKey, BspStructuralKey>`（第 6 参数），调用方预解析、
  consume_at 只读不查簿。
- 四错误分支按票面顺序（先到先返回）：NoConsumption → LateAuthorization → InvalidPolicy →
  InconsistentState。
- InvalidPolicy 判据 = 空 rules，或两条 rule 投影同一 `(policy_id, rule_id, slot)` 三元组。
- InconsistentState 判据 = `prior_bsp_by_key` 任一 `created_at > as_of`，或
  `prior_links_by_key` 任一 `written_at > as_of`。
- 主循环：只授权 `status == Alive`；`event_to_bsp` 查无跳过（Absent 非证伪）；prior 已有
  bsp_key 不重复创建、已有 link_key 不重复产链接；同一 bsp_key 一次调用内
  `created_this_call`/`linked_this_call` HashSet 兜底去重。

## 四、证据与遗留

- 所有承重断言已由单文件测试绿 + 编译绿覆盖；无「查不到」字段。
- 遗留（票面已授权照实 TODO，不擅自扩语义）：LateAuthorization 精确判据待冻结语义细化；
  InconsistentState 精确判据（prior 摘要 vs state_as_of 重算）待 E2E-O 修订协议落地后细化。
- 未提交（worktree detached HEAD，实装报告与 diff 均在工作树内，由父会话裁决后走提交）。
