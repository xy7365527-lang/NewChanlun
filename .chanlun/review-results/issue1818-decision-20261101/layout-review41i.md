# PR #1826 独立只读审查：BLOCK

- 审查对象：`b636af32e84c9e64701d9f7c4d3c482dd83bf434`。
- 集成基线：远端 main `71df2ae749664714e6bff10f8f71d9e15cc4c815`（2026-11-01 00:11:41 UTC 实查）。
- 只读工位：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。
- 审查身份：非作者独立 Astra 子代理；原二轮起草/独审均 Opus。本次未改仓库、票、分支或服务。

## 唯一阻断项：库文件符号链接漏过布局判定

**P2 / ADR 0059:82–86**：`Q = canon(parent(d))` 只解析库的父目录，漏掉库文件本身的符号链接。现役写者的锁路径却由整个库路径 canonicalize 后决定。按该合同实施，会把真实写目录在 M 内的布局放行，留下本票要消除的 S 启动竞态。

静态反例（目录及目标文件均已存在，锁文件尚不存在）：

- `s.db = /abs/s/s.db`，故 `P = /abs/s`；A 独立为 `/abs/s-state`。
- `x.db = /abs/x/alias.db`，该文件符号链接指向 `/abs/s/x.db`。
- ADR 推得 `Q = /abs/x`，既不等于 P，也不在 A 或 S 族条目内，因此放行。
- `session_transport.rs:43–56` 对存在的 d 执行 `db.canonicalize()`，再拼 `.writer.lock`；`:57–62` 创建 `/abs/s/x.db.writer.lock`。
- `x_execution_session.rs:171–178` 不禁止这种文件链接，`serve` 直接调用此锁函数，然后打开 DB。`production_session.rs:455、:490` 无等待先启动 S 后启动 X。
- 因而 X 能在 S 的严格打开核验窗口改动 P。`admission_provider.rs:19–39、:74–88、:299–302` 把目录 mtime/ctime 等纳入严格 stamp，足以触发 `SourceIdentityChanged`。

这是从现役源码推出的具体反例；没有运行 Rust 或制造真实会话。结论不依赖 SQLite 是否另建 journal：锁文件创建本身已足够。

**最小修订方向**：把已存在 d 的最终文件链接解析纳入实际写目录判定，不能只 canonicalize 父目录；与现有 `process_lock` 的实际目标对齐，同时保留已裁同目录、A 子树、S 族与不存在路径的覆盖。同步修订 Q2/Q8 的推理、裁定第 3–4 条和验收，至少加入“安全目录内 x.db 文件链接指向 P 内库”的拒绝用例，核 `InvalidDomain`、字段名及拉起前无新增域文件。不要通过取消链接解析或放宽 S admission 来修。

## 已通过的发布与集成检查

- PR 实查为 OPEN、Draft、MERGEABLE；仅新增该 ADR，154 行；工位无未提交改动。
- `git diff --check origin/main...HEAD` 通过；`git merge-tree --write-tree origin/main HEAD` 无冲突。
- 父提交 `9de18e606b2c0330162a83e2a030898b46c3690f` 至当前 main 只改六份 S audit/prepared/state 文件；本反例与 ADR 所引 admission、writer_cli、launcher、transport、X 文件未变。
- 对原 draft 的 diff 只有正本头/状态、空白/强调调整及移除“引用自证”“考古对照”工作凭据；裁定与验收语义保留。阻断来自原裁定本身漏查写目标，不是发布转换丢文。

## R1–R8 复核

| 项 | 结论 | 依据 |
|---|---|---|
| R1 引用与事实 | BLOCK | 已核关键权威与源码引文；但 Q2 把并发写足迹等同于 `canon(parent(d))`，与 transport 对整个 d 的解析事实冲突。 |
| R2 候选与权威一致 | PASS | ADR 0028:46、0046:20–21 支持不静默换库；启动排序只是不选，显式部署工具仍是未来候选；未扩大排除旧件。 |
| R3 二值验收 | BLOCK | 现有目录链接正反控可执行，但缺最终 DB 文件链接反例，全部现有验收通过仍可漏放。修订上述一项时补对应验收。 |
| R4 落点与范围 | PASS | 正本、load_manifest、实施测试与 runbook owner 清楚；资金、隔离、恢复帽、P0、Destination 不变。 |
| R5 单一判据 | PASS | 无放行开关或失败后宽松兜底；静态布局和 S 动态 stamp 是不同判断。 |
| R6 图与 owner | PASS | 当前 #1781 blocked_by 实查为 #1769、#1818；#1823 blocked_by 为空。ADR 保留前者、不增后者，实施及关票动作区分清楚。 |
| R7 T/P 分级 | PASS | 属已有 InvalidDomain 先例的接口、owner、测试合同；此次缺口仍是技术语义修订，不涉及 P 类政策。 |
| R8 考古完整性 | BLOCK | 原 Q1–Q12 全保留，但 Q2/Q8 漏查 `session_transport::process_lock` 的最终 DB 路径解析，需用此次代码反例订正。 |

## socket、恢复与目录身份

`writer_cli.rs:236–242` 明确先 parse 再 admission；`:357–359` 对任何既存 socket 路径以 `TransportUnavailable` 拒绝；driver open 在 :725，bind 调用在 :802。ADR 对“残留 s.socket 先于 admission 拒绝”的订正正确。运行期目录采用 `same_object`（dev/ino/mode/uid/nlink），不核完整 mtime/ctime stamp；不能把此结论误写成运行期完全不核目录，亦不能凭它宣称恢复完成。

本次不要求把 socket 清理塞入 #1781，不要求新设 #1818→#1823 依赖；沿用 #1823 独立报告的观察/恢复授权边界。L-2 保持只记录、另票 S 自核，不因本次审查改 recovery 或 admission 合同。

## 交回主控

1. 将上述库文件链接缺口交原裁定/发布 owner 修订；新 head 复核该差异与新增验收后再决定发布。
2. 0058 在 PR #1821；编号连续不是语义依赖。本次不以其未合入为 BLOCK。验收 14 的“合入时最大编号加一”由发布 owner 在实际落地时处理，不应据此新增产品依赖边。
3. 修订通过后仍需主控完成既有 CI/合入门和票面动作：#1818 resolution/关票、#1781 实施范围、#1323 Notes；#1769→#1781 不变。PR 合入不自动算实施或关图。

未运行 Rust、CI、重型实验；未等待 CI。`Prove It Works` 在此任务体现为逐段检查裁定、原稿差异与现役写者调用顺序，而不是用八锚句命中替代语义证明。最终读回校验时间：2026-11-01 00:16:53 UTC。
