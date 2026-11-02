# #1626 / PR #1693 整票非作者独审

- 候选：`fe1710dc3a7e35614549ed003adcdda8ed60b753`（审查时 PR head 相同）
- 基线：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：Q-4 trend_scope、Q-6～Q-8 tower_base、Q-2 selector_keys、X-38 tangency 及直接调用者、IO、测试与证明。
- 方法：对照 #1626、ADR 0041、`qujiantao.md` §14 与 `xianduan.md` X-38，检查完整 diff、生产/证明调用链及已有 nc-remote、Lean、Python 证据；独立执行轻量检查。未修改代码、PR 或 Issue，未运行本机 cargo/rustc。

## 代码审查报告

### 概念层质询结果

**通过（Q-4、Q-2、Q-6～Q-8）；X-38 的实现语义未发现反向矛盾，但其验收链未证明 required 行为。**

1. **Q-2 原阻断已消除**：`NestLevel.chosen` 在 `formal/Origin/IntervalNestCertificate.lean:527-528` 实际调用 `selectΘWithKeys ℓ.policy.keys`；`subB`/`nestCertB`/`chiBool` 均沿该节点取值。`nestCertB:573` 与 `NestCertPartialCorrect:652` 强制相邻级 policy 相同。`SelectorPolicyIO.readNestChain` 在 IO 边界读取一次记录并注入全链；`chiBoolWithRecord` 贯通真实证书入口。默认等值、chosen 变异、混合记录拒绝和缺文件/null/AwaitingPolicy/非法方向测试均存在且加入 Lake 根目标。
2. **Q-4 预注册边界成立**：`policy_records.rs:370-375` 将 default/hold26_anc/fusion_ta 分别绑定 SelfLayer/Ancestor/SelfOrAncestor；`:407-414` 对跨 profile 值返回 `InvalidPolicy`。`positional.rs:1146-1170` 在 `run_fusion` 前完成同一准入，合法值才进入判据并写版本戳。六种越界组合已有生产负控。
3. **tower_base 路径成立**：`try_level_config_from` 在 `policy_records.rs:463-469` 对无真实构件的 kline3/trade3 返回 typed `InvalidPolicy`，不是仅靠 panic；classify 在任何 L0 前复核基底。compose/resume 与 Python 递归底均消费同一记录，默认 segment 等值与版本戳有证据。
4. **X-38 定义本身一致**：`HasGap`/`Overlaps` 均显式接收 Tangency，`gap_iff_not_overlap` 对两值重证；Claim10 与 Claim5 的参数也已穿透现有定义。问题在验收没有证明 Claim5 的 open 行为及全部负控，而非已证明这些定义错误。

### 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 1 | fail |
| MEDIUM | 0 | info |
| LOW | 0 | note |

**结论：FAIL / BLOCK。当前 head 不具备合入或关闭 #1626 的资格。**

## Findings

### HIGH-1：X-38 的 Claim5 open 行为与负控没有进入有效验收，且测试文件不在默认构建根

**位置**：`formal/Origin/VersionedTangencyTests.lean:17-39`、`formal/lakefile.toml:104`。

1. **Claim5 的 open 没有行为断言。** `:20` 只是 `#check ... wellFormedV0 .open`，只显示函数类型。`:26` 的 `let _claim5 := ... p.tangency` 仅把当前记录值（`closed`）作为第一个参数绑定，未提供 `Segment` 输入，也未求值或断言 open 的相切结果。因而报告所称“Claim5 显式 open 保持历史参考行为”没有对应测试证据。
2. **该测试文件未加入默认构建。** `lakefile.toml` 的 Origin roots 包含 `Origin.VersionedSelectorTests`，不含 `Origin.VersionedTangencyTests`。仓内 `.github/`、`scripts/`、`formal/` 其它位置也未找到该文件的替代 CI 入口。当前只能证明它可被手工 `lake env lean` 执行，不能证明默认 `lake build`/CI 会持续执行这些验收。
3. **负控不完整。** 文件只有 `:32-39` 的 `AwaitingPolicy` 一例，缺少本票要求的缺记录、null 与非法值负控。 selector 的同类负控不能代替 tangency 的独立 IO 边界。

这使 X-38 的“默认行为等值 + 变异负控 + CI 持续验证”未闭合。需要：补一个真实三笔相切用例，分别断言 Claim5 `open` 与 `closed` 的相反结果（并保持 Claim10 的 closed）；把 `Origin.VersionedTangencyTests` 加入 Lake 根目标或提供等效明确 CI 入口；补齐缺文件/null/非法值负控。不得只保留 `#check` 或当前 closed 的函数绑定来宣称 open 已验证。

## 分项结论

| 范围 | 结论 | 依据 |
|---|---|---|
| Q-4 trend_scope | **PASS** | profile 预注册值、生产准入与六组越界负控均成立 |
| Q-2 selector_keys | **PASS** | 原 `NestLevel.chosen`/证书链已接管同一记录，默认/变异/混合链/IO 负控齐备 |
| Q-6～Q-8 tower_base | **PASS** | Rust 与 Python 全链消费记录，segment 等值、版本戳及 kline3/trade3 typed reject 成立 |
| X-38 tangency | **FAIL** | Claim5 open 无行为验收；测试不在默认根；缺记录/null/非法值负控缺失 |
| 全套 CI | **IN_PROGRESS** | run `40131130061` 尚未终结，未预写全绿 |

## 独立验证与既有证据

- 独立执行 `cd formal && lake env lean Origin/VersionedTangencyTests.lean`：文件可单独执行，输出 `TANGENCY_POLICY_IO_OK`、`TANGENCY_NEGATIVE_OK`；同时确认 `#check` 只打印 `wellFormedV0 ... : Segment → Prop`，没有 Claim5 open 的行为断言。
- 独立检查根列表：`Origin.VersionedSelectorTests=True`，`Origin.VersionedTangencyTests=False`；未找到其它 CI 引用。
- `git diff --check origin/main...HEAD` 通过；PR head 复核仍为 `fe1710dc3a`。
- 已阅读 nc-remote `20261025-035202-1af4e64ae0.log` 与 `20261025-041454-05ecc0338c.log`：Rust 相关测试、cargo check、fmt 与 Q-4 的 105 项通过证据成立；这些不能消除上述 Lean 验收缺口。
- 既有 Python 94 项、Lean build 与 fixture 三项字节一致证据可作辅助，但不替代 X-38 的 open 与负控验收。

## 下一步

1. 按 HIGH-1 补真实 Claim5 open/closed 行为测试、全部 IO 负控及默认构建入口，推新 head。
2. 对新 head 复核完整 X-38 直接依赖与默认 fixture 字节一致，确认没有以测试修补替代生产语义。
3. 等 GitHub CI 终结，再进行 main 合入批准与关票门；#1323 全图不随本票局部通过而关闭。
