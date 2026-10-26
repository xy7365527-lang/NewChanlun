# PR #1688 `78dc044950` 非作者复审

- 审查时刻：2026-10-24T18:02:48Z；再次核对 PR head 同为 `78dc0449500d5ad82ef42b5304527beade24fc8e`。
- 固定点：`origin/main=3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`。
- 范围：`origin/main...HEAD` 全差异（8 文件，+599/-50），重点覆盖 `af6629ae1e..78dc044950` 的 BindVoice 负控与 C4b 恢复返修；关联 #1621/#1622。
- 方法：概念层质询、Standards/Spec 双轴独立审查、生产调用链与测试源码、同 SHA nc-remote 原始日志、GitHub CI。未修改仓库，未在本机运行 cargo/rustc。

## 结论

**代码/合同：PASS。上一轮 #1622 的 HIGH-1 已闭合，未发现新的代码阻断。**

**交付门仍未完成：CI run `40118404588` 尚未终结，不能据此合入或关闭 #1622。#1621 的 X 执行域仍为 NOT_VERIFIED，本候选只能作为 S/E/B 与主线回归修复的部分交付，#1621 必须保持 OPEN。**

## 代码审查报告

### 概念层质询结果：通过（保留 X 缺口）

1. **时钟定义一致**：`store.rs:1451-1460` 比较 `record["b_observation"]["first_known_ns"]` 与 E 原件 `record["commit_ns"]`，不是 B.commit_ns。`service.rs:1112` 在 B 持久化前调用同一判据，恢复审计在 `store.rs:1471` 复用，没有形成宽严两档。
2. **BindVoice 反例有效**：`tb03c_kill_tests.rs:1302-1319` 在步骤 21 的 before_commit 故障后跳过原消息重送；`:1382-1419` 要求 `final_diff` 报域 1 差异，并另核公开查询和直接 `QueryVoice`。参照集合显式断言包含 `kill-voice-1`，避免负控因目标根本不存在而空转。wrong-allocation 仍保留为另一种失败，不代替丢消息。
3. **恢复顺序符合合同**：`tb03c_c4b_tests.rs:552` 已先恢复 E；`:571-575` 只恢复三个 B，避免 E 二次恢复后领先一列。生产门未改，E 在全测试后段仍有 `c4b_reopen_all` 覆盖。该证据只证明实际写明的恢复协议，不扩写为任意恢复次序均已验证。
4. **X 边界不变**：S→B、E→B 的来源与负控存在，B 书账转移仍不是 X。当前没有真实 X 执行域正例与“提前获知”变异负控，不能把测试名中的 S/E/B/X 当作覆盖证据。

### 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | — |
| LOW | 0 | — |

结论：**PASS（代码与合同）；CI 另列。**

## Standards

独立 Standards 轴未发现信心超过 80% 的违规：

- 时钟判据由正式写入与恢复审计共用，没有第二套判据或失败兜底。
- `public()` 将 QueryVoice 纳入既有公共比较，`skip_resend` 复用同一故障轨；两个负控不互相替代。
- C4b 只调整测试恢复顺序，未改生产语义。相似 JSON 夹具属判断性重复，未达到应报告的缺陷。
- `git diff --check` 仅报告两份实施文档的 5 处 Markdown 硬换行尾双空格，不计代码缺陷；Rust `cargo fmt` 已由远端证据覆盖。

## Spec

独立 Spec 轴未发现新的阻断：

- **#1622 变异负控**：`tb03c_kill_tests.rs:1382-1399` 的全表与公开查询比较都实际返回 Err；`:1401-1419` 直接查询返回 `MissingDependency：Voice 尚未绑定`。22 步 before/after commit SIGKILL 矩阵未缩减。
- **C4b 回归**：修复针对旧 CI 的真实失败次序，不放宽 `B.first_known_ns >= E.commit_ns`。
- **#1621 X**：仍为 NOT_VERIFIED，不因本候选关闭。

## 前次 BLOCK 对照与 AC

| 项目 | 本次判定 | 证据 |
|---|---|---|
| HIGH-1：缺 `skip_resend(21)` | **PASS / 已闭合** | 首处差异为域 1 `/book/0/json/voice/identity 只在一侧`；公开查询比较亦失败，直接 QueryVoice 返回绑定缺失 |
| BindVoice before/after commit 真 SIGKILL | PASS（靶向）；完整 CI 待核 | `20261025-013852-888fb99f97.log`：2/2，日志含 pid、signal=9、重送及终态哈希 |
| 两次新进程终态逐字节相同 | PASS（靶向）；完整矩阵待 CI | 同一日志两条 BindVoice 轨各 rep-0/rep-1 的 `final_tables_sha256` 分别一致；续跑回复亦比较 |
| 变异负控 | **PASS** | skip-resend Capture/Apply/BindVoice + wrong-allocation 共 4/4 |
| C4b extras 回归 | **PASS（靶向）；完整 CI 待核** | `20261025-015017-5a0ac0b14b.log`：原失败及两个相关恢复用例各 1/1，funding 14/14 |
| #1621 known_at 来源与 S/E/B 负控 | PASS（当前范围） | 生产字段与正负控如上；X 不在此结论内 |
| #1621 X 正例与提前获知负控 | **NOT_VERIFIED** | 无真实 X 执行域证据；保持 OPEN 与后续承接 |
| 非作者复审 | 本报告完成，代码 PASS | 评审面为当前 head `78dc044950`，不是旧 `af6629ae1e` |

## 原始证据与 CI

已读取：

- `/tmp/nc-remote/20261025-013852-888fb99f97.log`：EXIT=0；新负控 1/1、BindVoice SIGKILL 2/2、全部负控 4/4、rustfmt 通过。
- `/tmp/nc-remote/20261025-015017-5a0ac0b14b.log`：EXIT=0；C4b 三个恢复用例通过、funding 14/14、rustfmt 通过。
- 旧 CI `40083596265`：extras 唯一失败是 C4b 的 `InvalidDomain：B 获知时刻早于 E 提交`，不是主线旧三项 funding 红；返修针对该真实签名。

截至审查收尾，GitHub CI `40118404588` 仍为 **IN_PROGRESS**：rust-check、test、s-observer、fixture-drift、sandcastle-ts 已 SUCCESS；default、四个 s_session shard、extras、backtest-bin 与 DevSkim 尚未全部终结。故本报告不作完整 CI 通过或 main 合入许可。

## 下一步

1. 等当前 head 的完整 CI 终结并核原日志；若出现新失败，按实际签名返修并复审新 head。
2. CI 与关票门满足后，#1622 才可进入用户 main 合入闸；#1621 另补真实 X 执行域证据，不随本 PR 关闭。
