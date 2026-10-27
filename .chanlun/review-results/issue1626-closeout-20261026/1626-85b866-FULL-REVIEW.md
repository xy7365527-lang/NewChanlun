# #1626 / PR #1693 整票非作者终审

- 候选：`85b866b2ca26c4a1784452511d8bdba433b1a2b9`（审查时 PR head 相同）
- 固定点：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：Q-4 trend_scope、Q-2 selector_keys、Q-6～Q-8 tower_base、X-38 tangency，以及直接调用者、IO、默认构建与验证证据。
- 方法：对照 #1626、ADR 0041、`qujiantao.md` §14、`xianduan.md` §6.3/X-38，检查完整 37 文件 diff、实际生产/证明调用链和测试；Standards 与 Spec 分轴独立复核。未修改代码、PR 或 Issue，未运行本机 cargo/rustc。

## 代码审查报告

### 概念层质询结果

**通过。** Q-4 的 profile 预注册边界、Q-2 同一记录贯穿真实证书链、tower_base 的构造前拒绝，以及 X-38 的 `HasGap`/`Overlaps`、Claim10/Claim5 参数化均与正本一致。未发现新的第二配置源、失败兜底、级别间混值或新增公理。

本次阻断不是生产定义方向错误，而是 **X-38 的 open 变异验收没有执行“记录 → 行为”链**，因此不能据此宣称 ADR 0041 的改签重推导已经验证。

### 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 1 | fail |
| MEDIUM | 2 | 维护性提示 |
| LOW | 1 | note |

**结论：FAIL / BLOCK。当前 head 的 X-38 验收仍未闭合，不具备合入或关闭 #1626 的资格。**

## Standards

未发现凭据泄露、新增 `sorry`/`admit`/公理或绕开生产判据；`git diff --check origin/main...HEAD` 通过。

两项维护性提示不单独构成行为阻断：

1. `rust/src/theta_v0/policy_records.rs` 从 671 增至 874 行，承载多类 policy；后续宜按领域拆分，避免继续扩大 Divergent Change。
2. `formal/Origin/IntervalNestCertificate.lean` 从 719 增至 865 行，通用 selector 证明与证书链同文件；后续宜评估拆出独立选择器模块。

另有一项 LOW：`SelectorPolicyIO.lean:9-15` 与 `TangencyPolicyIO.lean:8-14` 的 `jsonString/jsonNat` 完全重复，可复用小型 IO helper。以上是结构与可维护性问题，不用于替代或放大下面的 Spec 缺陷。

## Spec

### HIGH-1：open 新版本只校验字段，没有验证记录驱动行为重推导

**位置**：`formal/Origin/VersionedTangencyTests.lean:107-115`。

当前 `#eval` 在读取 version 3 的 open 记录后，仅检查：

```lean
p.recordId == "segment-second" && p.version == 3 && p.tangency == Tangency.open
```

满足便打印 `TANGENCY_OPEN_MUTATION_OK`。它没有把 `p.tangency` 传给 `wellFormedV0` 或 `classifyTermination` 求值。上方独立的 open/closed 示例虽然证明定义在两值下行为相反，却不能证明这个具体记录被消费后确实触发该行为。

同文件 `:97-105` 已对现役 closed 记录实际计算 `classifyTermination p.tangency`，所以此处缺的不是生产参数化能力，而是 **open 变异验收的执行链**。这正好落在本票“改字段须改变指定行为、按新版本重推导”的要求上。

**必须返修**：在同一个 `#eval` 中用该 open 记录断言实际结果，例如：

- `classifyTermination p.tangency ... = .secondKind`；或
- `¬ Segment.wellFormedV0 p.tangency tangentClaim5Segment`。

测试应能在实现忽略 `p.tangency` 时失败，而不是仅凭记录字段就打印成功。修复后按新 head 做 X-38 尾部复审。

## 分项结论

| 范围 | 结论 | 依据 |
|---|---|---|
| Q-4 trend_scope | **PASS** | 三 profile 各自绑定预注册值，跨 profile 组合在 `run_fusion` 前 typed reject；无第三祖先臂 |
| Q-2 selector_keys | **PASS** | `NestLevel.chosen` 与 `nestCertB`/`chiBool` 实际消费同一 `SelectorPolicy`；默认等值、chosen 变异、混合记录及 IO 负控均成立 |
| Q-6～Q-8 tower_base | **PASS** | LevelConfig/classify/compose/resume/Python 递归底消费同一记录；segment 默认等值，kline3/trade3 在构造前 typed reject |
| X-38 tangency | **FAIL（验收）** | 三笔相切 open/closed 定义断言、Claim5/Claim10 对照、默认根目标与四类 IO 负控已成立，但 open 记录→行为变异未实际执行 |
| 全套 CI | **IN_PROGRESS** | run `40132467260` 尚未终结；fixture-drift、rust-check、sandcastle-ts、s-observer、test 已绿，其余 job 仍在运行 |

## 独立验证

- `cd formal && lake env lean Origin/VersionedTangencyTests.lean`：实际输出 `TANGENCY_POLICY_IO_OK`、`TANGENCY_OPEN_MUTATION_OK`、缺记录/null/AwaitingPolicy/非法值四类拒绝标记。该输出同时证明 HIGH-1：open 标记可在没有行为断言的情况下取得。
- `cd formal && lake build Origin.VersionedTangencyTests`：成功，10 jobs，明确 `Replayed Origin.VersionedTangencyTests`，确认该文件已进入实际 Lake 构建路径。
- `cd formal && lake env lean Origin/VersionedSelectorTests.lean`：默认 chosen、记录变异、`chiBoolWithRecord`、混合记录与四类 IO 负控均通过。
- GitHub `fixture-drift` 已为 SUCCESS；`git diff --check` 通过。未重复 Rust 重型测试，未运行本机 cargo。

## 下一步

1. 补 open 记录驱动 Claim5/Claim10 的实际结果断言，推新 head。
2. 对新 head 复核 X-38 后直接依赖与默认 fixture 字节一致，等待完整 GitHub CI。
3. 完整 CI 与当前 head 独审通过后，再进入用户 main 合入闸与关票门；#1323 全图不随本票局部通过而关闭。
