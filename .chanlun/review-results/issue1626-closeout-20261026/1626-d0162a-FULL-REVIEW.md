# #1626 / PR #1693 整票非作者独审

- 候选：`d0162a3609ff36960dfef441755472e7e2dc8093`（本次审查时 PR head 相同）
- 基线：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：Q-4 返修、Q-2 selector_keys、Q-6～Q-8 tower_base、X-38 tangency 与相关调用链。
- 方法：对照 #1626、ADR 0041、`qujiantao.md` §14 与 `xianduan.md` X-38，检查完整 diff、直接调用者、测试与独立轻量验证。未在本机运行 cargo/rustc；未修改代码、PR 或 Issue。

## 代码审查报告

### 概念层质询结果：发现矛盾（HIGH）

**H-1：Q-2 新增参数化选择器没有接管现有 `NestLevel.chosen`，现役选择仍走旧硬编码链，形成第二套默认来源。**

`qujiantao.md` §14.1 要求 `selector_keys` 成为一次运行、全级共用的一份版本化记录；ADR 0041 要求运行路径从记录取值，不能保留代码常数或备用配置源。当前实现没有满足这项合同：

1. `formal/Origin/IntervalNestCertificate.lean:90-112` 新增 `selOrderBWithKeys`，`:213-214` 新增 `selBetterWithKeys`，`:228-230` 新增 `selectΘWithKeys`；但旧 `selOrderB`（`:115-118`）、`selBetter`（`:217-218`）、`selectΘ`（`:233-235`）仍保留独立的硬编码默认逻辑，并不是新函数的显式默认包装。
2. 现有 `NestLevel.chosen`（同文件 `:421-422`）仍直接调用旧 `selectΘ`，继续沿 `selectΘ → selBetter → selOrderB → Strict.Nest.selOrder`（`formal/Strict/Nest.lean:185-188`）运行。`subB`、`nestCertB`、`chiBool` 和 `ChosenByΘ` 继续消费这条旧链，未接收任何 `SelectorPolicy`。
3. `readSelectorPolicy`（`formal/Origin/SelectorPolicyIO.lean:30`）只在 `VersionedSelectorTests.lean` 中用于新 `SelThetaRanked` 和记录元数据检查。它没有被用于上述 `NestLevel.chosen` 生产链。`NestingCertificate.N` 当前不消费新 `SelTheta`，不能用该新函数的存在证明原选择路径已读取记录。
4. 因而改 `selector-keys.json` 的合法键序时，新增的 `SelThetaRanked` 可以变化，而 `NestLevel.chosen` 仍按旧默认键序选择。当前同时存在“记录驱动的新选择器”和“代码写死的旧选择器”，违反一记录、全级同键表的要求，也使变异负控无法覆盖真正的证书选择路径。

这不是只缺一条测试：现有 `NestLevel`/`chiBool` 接口没有传入记录的途径，必须调整生产调用链。

**必须返修**：

- 将 `selOrder`/`selOrderB`/`selectΘ` 及 `NestLevel.chosen` 的调用改为显式消费同一 `SelectorPolicy`（或等价类型化键表），并把该值贯穿 `subB`、`nestCertB`、`chiBool` 和 `ChosenByΘ`；每级不得各自取默认。
- 在 IO 边界实际读取 `selector-keys.json` 后传入上述证书路径，而不是只传入 `VersionedSelectorTests` 的新函数。缺记录、null、`AwaitingPolicy`、非法方向与未批准 `endTime=ascending` 必须在边界拒绝。
- 旧硬编码函数须移除、改为只接收显式键表，或限制为明确的测试默认包装；不得让生产 `NestLevel.chosen` 继续使用独立旧默认。
- 增加一条经过 `NestLevel.chosen`/证书链的生产变异：改变合法键序时选择结果按记录变化，缺值或非法值被拒绝；默认 `selector-keys/1` 与旧行为逐位相同。不能只用 `selectΘWithKeys` 或 `SelThetaRanked` 的独立测试替代。

### 工程层审查

概念层未通过，按纪律不进入后续工程层验收。

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | — |
| HIGH | 1 | fail |
| MEDIUM | 未审 | — |
| LOW | 未审 | — |

**结论：FAIL / BLOCK。当前 head 不具备合入或关闭 #1626 的资格。**

## 其它范围与验证状态

- **Q-4：PASS（仅当前返修范围）**。三个 profile 分别限定 `SelfLayer`、`Ancestor`、`SelfOrAncestor`，六种跨 profile 组合在生产入口 `run_fusion` 前返回 typed `InvalidPolicy`；合法新版本可运行并携带 `trend-scope/2`。远端日志 `/tmp/nc-remote/20261025-041454-05ecc0338c.log` 为 105 passed / 0 failed，未把首轮夹具缺失误计为通过。
- **Q-6～Q-8、X-38：NOT_VERIFIED**。它们未因 H-1 被判失败，但也不以本报告宣称完成；待 Q-2 修复后重新整票复核。
- GitHub CI `40129181321` 仍在执行，未预写全绿。

## 已执行的独立轻量验证（不消除 H-1）

- `lake env lean Origin/VersionedSelectorTests.lean`：新选择器输出与 `SELECTOR_POLICY_IO_OK`、`SELECTOR_NEGATIVE_OK`；该入口未覆盖 `NestLevel.chosen`。
- `lake env lean Origin/VersionedTangencyTests.lean`：输出 `TANGENCY_POLICY_IO_OK`、`TANGENCY_NEGATIVE_OK`。
- `python3 scripts/check_fixture_drift.py`：center、certificate_chain、parity 均 0 字段差异且字节一致。
- 相关 Python 测试：94 passed / 0 failed。
- 上述验证与 Q-4 的 105 项通过均不能证明旧 `NestLevel.chosen` 已读取版本化记录。
