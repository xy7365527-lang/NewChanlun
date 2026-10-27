# PR #1681 / issue #1602 独立非作者最终复审

- 候选：`f8dec1a54f54544b57957f16ffce2b57dbbbe45f`
- base：`origin/main = 13ed4d45c8ff85175819ca987690328ab2a247a4`
- 前次 BLOCK 候选：`54cd6b2e2b8567e79ba3fe8bb513986ce007347c`
- 前次 base：`3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 工位：`/Volumes/AgentStorage/rebase-pr1602-20261024`
- 身份：独立非作者复审，不是实施者；未修改代码、票据或分支，未派子代理，未在本机运行 cargo/rustc/cargo test。

## 裁决

| 维度 | 结论 |
|---|---|
| CRITICAL | 0 |
| HIGH | **1（旧库缺少 `policy_records` 迁移）** |
| MEDIUM | 0 |
| LOW | 0 |
| 代码 / 合同 | **BLOCK** |
| 当前 head 的 GitHub CI | **PASS（但不覆盖旧库）** |
| 总体 | **BLOCK** |

前次 MEDIUM（v2/v3 外部回执缺记录 id/version）已由当前实现闭合；本次新发现的 HIGH 是独立的既有数据库兼容性缺陷，不能用新库测试通过或前次返修有效来抵销。

## 候选有效性与审查范围

- 复审首尾均核对：PR `headRefOid` 仍为上述候选，`baseRefOid` 仍为上述 base；本地 HEAD 也为候选，工作区 clean。
- 审查完整 `base..HEAD`：35 个文件，2632 insertions / 347 deletions；重点覆盖 v2/v3 回执、持久化、恢复、生产/诊断分派、记录读取与只读严格模式。
- 已读根 `AGENTS.md`、issue #1602、前次 BLOCK 报告、实施报告、ADR 0041/0040 与当前源码、测试及 CI。
- `git diff --check base..HEAD` 通过。

## 前次 MEDIUM 已闭合

### 1. 正常、幂等与 receipt 查询都读取实际历史 generation

- `structure_deltas` 在发布事务内保存实际 `policy_records`（`s_structure_session.rs:3858-3875`；v3 `commit.rs:283-290`）。
- v2 的 `lookup` 从消息绑定的 `first_published_generation` 读取记录戳（`s_session_v2/mod.rs:2244-2250`），不是调用时的当前配置。
- v2 正常入口保留并消费 `advance_core` 结果；Committed 回执与本次实际生成的记录戳交叉核对（`s_session_v2/mod.rs:2387-2418`）。无新输入的 advance 也读取既有 generation 的持久戳（`s_structure_session.rs:3070-3089`）。
- v3 的 `Generated.policy_records` 经 `Prepared` 传到各 commit 发布入口（`prepared.rs:573-588, 628-636`），再由共享 `lookup` 形成外部回执。`commit::publish_prepared` 的默认配置包装仅在 `#[cfg(test)]`，不是生产记录来源。

### 2. 双版本、重开和恢复路径有实际覆盖

- v2 测试 `v2_external_receipts_keep_historical_policy_records_across_config_versions` 覆盖正常成功、配置改版后的幂等重投、receipt 查询、无新输入 advance、第二版本以及配置回退后的重开查询；两版记录可区分。
- v3 测试 `driver_external_receipts_keep_historical_policy_records_across_config_versions` 同样覆盖正常、幂等、查询和 drop/reopen。
- `resumed_driver_receipt_uses_records_from_actual_recovery_generation` 在 Begin 后中断，经监督器 recover，再由新 writer 的 `open_with_config` 触发 `resume_pending`，实际发布 pending 原件并核对恢复 generation 的 v2 记录戳；不是只测一个恢复名称。
- 诊断配置仍明确为 `admissible=false`；测试输出可辨认 `venue-profile-diagnostic-fixture/1`，未接入生产准入。

### 3. 只读严格模式同步有效

`s_query_integrity.py` 与 Python 持久模式夹具已同步新增列。当前 `s_query_integrity`、TB-02-B/C、Python 持久模式/证据图/键复用测试与 GitHub CI 均通过，未观察到新增只读绕过。

## HIGH — 新增非空列没有旧库迁移，既有会话与回执会失败

### 缺陷

当前实现把 `policy_records TEXT NOT NULL` 加入 `structure_deltas`（`s_structure_session.rs:289-302`），但只使用 `CREATE TABLE IF NOT EXISTS`，没有为已经存在的同协议数据库补列或给出兼容读取路径。

旧库仍能通过现有的协议/初始化检查，却会随后失败：

1. 查询既有回执时，`lookup` 直接执行 `SELECT ... policy_records ...`（`s_session_v2/mod.rs:2244-2245`）；
2. 无新输入的幂等 advance 直接读取历史 `policy_records`（`s_structure_session.rs:3073-3079`）；
3. 新发布向旧表插入新增的必填列（`commit.rs:283-290`；v2 在 `s_structure_session.rs:3858-3875`）；
4. Python 只读严格模式的表结构也改为要求该列（`s_session/s_query_integrity.py:68`）。

`ensure_initialized` 只检查 `session_id`（`s_structure_session.rs:1890-1899`），并不验证该列存在；`init` 的“无热迁移”只限制覆盖初始化，不能保护正常 Driver 打开既有库。仓内检索 `ALTER TABLE structure_deltas` / `ADD COLUMN policy_records` 无结果，未发现迁移或显式兼容门。

### 最小复现

按旧 schema 建表后执行当前查询：

```text
sqlite3 :memory: \
  'CREATE TABLE structure_deltas(...旧列...); SELECT policy_records FROM structure_deltas;'
→ Error: in prepare, no such column: policy_records
```

因此这不是“缺少一个理想测试”的问题，而是当前 SQL 在真实旧库上必然报错。新库 CI 全部使用新 schema，无法覆盖这一兼容性面。

### 影响与关闭条件

该问题破坏既有会话的 receipt 查询、恢复及后续发布，也会使旧回执无法按新合同读取。属于生产持久化兼容回归，定为 HIGH。

关闭条件：

1. 为既有 `structure_deltas` 增加明确的迁移/兼容方案，保留旧消息身份、原始证据和历史 generation；不得用当前配置反填并冒充旧结果实际使用的版本。若旧记录不可确定，须使用明确的 `legacy/unknown` 语义，而不是伪造版本。
2. 增加旧 schema 夹具：至少含一个已提交 generation，分别验证旧 receipt 查询、无新输入 advance、恢复以及迁移后的新发布；新发布必须写当前实际记录版本。
3. 同步 Rust 与 Python 严格模式，并证明旧库迁移后只读审计通过。

## 其余合同复核

- P0 的 tick/成本未签仍在正式配置构造及 Begin 前 fail-closed；没有以诊断默认值参与生产。
- 场所、Θ_risk、数据 profile 的缺记录、缺字段、非法值分型保持；损坏生产 profile 头仍报 `InvalidPolicy`，不吞成缺记录。
- `theta_accept` 继续把配置失败与数据空窗分开，混合“一个成功 + 一个配置失败”仍整批失败。
- 既有等值记录、坏 tick 阈值、成本与声部参数、版本化输出及研究覆写标注未发现新的退化。
- 本次未发现前次 MEDIUM 以外的额外代码问题；上述 HIGH 是新增列自身引入的独立缺陷。

## CI 与平台证据（独立于代码裁决）

精确候选 `f8dec1a54f54544b57957f16ffce2b57dbbbe45f`：

- 主 CI [`40152849057`](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40152849057)：SUCCESS。rust-check、default、backtest-bin、四个 s_session shard、coverage、extras、Python/observer、sandcastle-ts、fixture-drift 均 SUCCESS。
- DevSkim [`40152849029`](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40152849029)：SUCCESS。
- `macos-session` 为 SKIPPED。旧候选 `07e3443c51` 的 macOS 26/0/3 证据不能外推为当前 head 的平台覆盖；当前 head 没有新的 macOS 实跑。

CI 全绿证明当前新 schema 的测试面通过，不证明旧库兼容，也不取消上述 HIGH。

## 剩余门

1. 先完成旧库迁移与旧 schema 回归，再重新独立复审；在此之前不建议合入。
2. 如需 macOS 平台验收，须在当前最终候选上另跑，不能引用旧 head。
3. PR 仍为 Draft；main 合入批准、关票门、工位清理及最终评审报告入仓仍由后续授权流程处理。本审不合 main、不关票。
