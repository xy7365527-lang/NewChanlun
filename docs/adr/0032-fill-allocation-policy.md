# ADR 0032：外效到重的唯一分配——投影清单冻结、累计比例、残量与更正逆规则

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核采纳并补充），走 [[grilling] 净额外效与部分成交费用的分配归属 #1350](https://github.com/xy7365527-lang/NewChanlun/issues/1350)（RD-12 / FU-03，SPEC #1340）
**状态**：已采纳。本 ADR 只定分配政策的形态、适用域与版本约束；实施归 #1406 / TB-04 / TB-06 叶票。
**前置**：[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md)（重间不仲裁）、[ADR 0024](0024-total-ledger-convention.md)（毛净两账、档 B 解锁）、SPEC C06.7/C06.8（去重、更正因果、FIFO 非政策）。

## 裁定

### 一、适用域由投影清单冻结时的 `projection_kind` 决定（第一问 A）

命令投影时冻结 `manifest = {projection_kind ∈ {per_leg, netted}, participants:[(chong_id, command_id, family_id, revision, requested_q_i)], tie_break_order, manifest_hash}`。

- `per_leg`：外效只接受来源逐腿归属（政策 `per-leg-source-attribution/1`）；缺归属 ⟹ Unmatched，不转比例。
- `netted`：外效只走冻结比例（政策 `frozen-manifest-proportional-cumulative/1`）；场所子回报只作证据保存。

`projection_kind` 是冻结 manifest 的参数，两条路径由冻结事实选定，不存在运行时回落（#799）。[ADR 0033](0033-history-simulation-bar-batch-model.md) 共用同一字段定义。

### 二、累计比例函数

`Q = Σ q_i`；`A_i(F) = ⌊F·q_i/Q⌋ + r_i(F)`，`r_i` 由最大余数法给出；性质为 `Σ A_i(F) = F`、`A_i` 单调不减、`A_i(F) ≤ q_i`、`A_i(Q) = q_i`。逐笔分配 ＝ `A_i(F_new) − A_i(F_old)`；逐笔回报与累计回报由同一函数归一。比例权重取冻结请求量（根确认），`[新缠论:选择]`。

### 三、残量与并列（第三问 A）

余数并列时，先给冻结请求量较大者；仍并列，按 manifest 登记的 `tie_break_order`（投影时由 chong_id 字典序生成，随政策版本登记）。代价：并列时序靠前者系统性多 1 量子；TB-06 须统计 tie-break 的发生次数并入报告（根补充）。金额残量同法。

### 四、反向净零不造成交（第二问 A）

`netted` 投影净量为 0 ⟹ 无物理命令、无内部 cross；两重的意图与最大责任各自保留（C05.2、C06.1）。净零域的经营成功路径 ＝ `per_leg` 投影（ADR 0024 裁定六档 B）。

### 五、费用按因果原件归属（第四问 A）

`cause ∈ {fill, command, period}`：fill 类沿该 fill 的原分配比例；command 类全额归发出命令的重；period 类按 ADR 0024 裁定六的坐标，真实场所净扣费 Σc = C 的分配矩阵与虚拟毛归因并列。`du = 0` 的费用合法，只改 K（[ADR 0031](0031-principal-batches-phase-predicate-corporate-actions.md) 裁定二）。fee-before-fill：E 侧 AwaitingDependency，前件到达后经同一 `book::apply` 应用一次。跨 Voice 迟到归原 family/Voice 尾账。

### 六、更正引用原政策版本的逆规则（第五问 A）

更正指向原子分量 ⟹ 反冲该分量；无指向的总量 b ⟹ 在同一 manifest、同一版本下计算 `A_i(F) − A_i(F−b)`。原报与旧 Applied 不删。迁移到新版本只能是带前后政策对照的显式事件（C09.11）。

## 数值口径

数量按 `quantum` 整数；金额为 `i64` 规范十进制文本 + `checked_add`；不出现浮点；分配结果与差量全部是整数。

## 字段名分

#1394 FIN 线的 `allocation_policy_id`（字段表 `finance.rs:205`，取值校验 `:225`）是 RD-11 资金准入政策 id；本 ADR 的外效分配 id 另行命名（`fill_allocation_policy_id`），二者不互代（根确认）。

## 受影响代码清单

见 [`.chanlun/review-results/issue1323-grilling-20261013/1350-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1350-AFFECTED-CODE.md)。Lean 新增义务：`Σ A_i(F) = F`、单调、`A_i(Q) = q_i`、逆规则差量恒等，另立具名文件，不改 `formal/Origin/LedgerReconciliation.lean:456`、`:467`、`:483`。

## 下游票

[#1406](https://github.com/xy7365527-lang/NewChanlun/issues/1406)（T-16/G1 成功路径、D-2/D-4/K9、T-20/T-21）、[#1412](https://github.com/xy7365527-lang/NewChanlun/issues/1412)、[#1428](https://github.com/xy7365527-lang/NewChanlun/issues/1428)、[#1397](https://github.com/xy7365527-lang/NewChanlun/issues/1397)（tie-break 次数）、[#1353](https://github.com/xy7365527-lang/NewChanlun/issues/1353)（批内共享容量分配）。

## 溯源标签

全部 `[新缠论:选择]`；缠师原文没有净额执行。
