# PR #1826 差异独审：BLOCK

- 精确 head：`a2cf513c25c45f1e835d7732425bdf024dd79b7f`；差异基线：`b636af32e84c9e64701d9f7c4d3c482dd83bf434`。
- 只读仓库：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。本轮仅审 ADR 差异与直接相关的现役源码。
- 2026-11-01T00:28:00Z 实查：本地 HEAD 与 PR head 相同，工作区干净；PR 为 OPEN、Draft、MERGEABLE，base 为 main。
- 原 41I 的 DB 最终文件链接反例已修复；本轮发现同一写足迹判定仍漏掉锁文件自身的最终链接。不是以“只多拒”为通过依据。

## 唯一阻断：锁文件链接仍能把创建落进 P

**F1 / P2 / ADR 0059:83–87**：Q₁、Q₂ 只覆盖库目录项与库目标的目录。`process_lock` 在解析 DB 后拼出锁路径，但随后 `OpenOptions::open` 仍会跟随锁文件本身的链接；拼接后的锁路径没有被纳入合同解析。

静态反例（无需并发改链接，无需 SQLite 创建侧件）：

1. `s.db=/abs/s/s.db`，P=`/abs/s`；A=`/abs/s-state`，与 P 分离。
2. `x.db=/abs/x/x.db` 是安全目录中的普通既存库文件。预先存在悬空链接 `/abs/x/x.db.writer.lock -> /abs/s/x-external-lock`，其目标尚不存在；目录均存在且可写。
3. 裁定计算 Q₁=Q₂=`/abs/x`，两者均不命中 P、A 或 S 族条目，因此放行。
4. `rust/src/session_transport.rs:43–56` 得到库规范路径；`:57–63` 在锁路径上以 create=true、truncate=false、read/write 打开，没有 O_NOFOLLOW。按普通 open/create 跟随最终 symlink 的语义，它创建 `/abs/s/x-external-lock`，直接改变 P 的目录 stamp。
5. X 的 `run`（`rust/src/bin/x_execution_session.rs:148–178`）只解析参数、profile、epoch 后即调用该函数，没有拒绝锁 symlink 的前置；launcher `load_manifest`（`:125–201`）亦无此检查。launcher 在 `:455` 启动 S、`:490` 启动 X，中间没有等待 S 打开完成。
6. S 的 `members`（`admission_provider.rs:206–255`）纳入 P 本身，却不会递归枚举 P 的所有非族兄弟；外部 `/abs/x/` 中的锁链接不在 M 内，目标名字也非 S 族。故 S 对 M 内 symlink 的拒绝挡不住这个布局。
7. `Stamp::of`（`:19–39`）包含目录 mtime/ctime；`Pinned::verify(true)`（`:74–88`）与 `verify_before_open`（`:299–302`）做完整 stamp 比较。目标创建若落在该窗口，会产生本票要消除的 `SourceIdentityChanged` 竞态。

本项是现役调用链加文件系统 open 语义的静态反例，未作运行复现。即使 SQLite 打开库随后失败，锁目标创建已发生。

**最小修订**：在 Q₁、Q₂ 之外，按 `process_lock` 的 exists 分支精确推导拼接后的锁路径，再解析该锁路径的最终链接（含悬空目标），把目标所在目录纳入同一拒绝集合；同步 Q2/Q8、裁定 3–4/8 和验收。建议补 `layout_rejects_writer_lock_symlink_into_s_parent`：上述安全 DB＋悬空锁链接，必须启动前 InvalidDomain，点名 x.db 与 s.db，P 项集不变、目标未创建。保留安全 DB 文件链接的正控，不要求修改 S admission 或扩大 socket 范围。

## 本次修订中已成立的部分

- 既存 DB 文件链接：Q₂ 解析整个 d，原 41I `/abs/x/alias.db -> P/x.db` 反例命中 P，原缺陷已修。
- 悬空 DB 链接：Q₁ 保留目录项处的锁创建目录；Q₂ 解析目标目录。相对目标与多级最终链接有明文规则。两处覆盖不能替代拼接后锁路径自身的解析。
- 新验收 9：准备一个 P/x.db 既存目标、锁不存在的夹具，退出码、字段名、P 项集与无新增 S/structure-source.json 均可作二值断言。
- 新验收 10：悬空 DB 链接目标为 A/e.db；退出码、字段名、目标未创建、无新增域文件可作二值断言。预置 symlink 是夹具，不计作运行新建域文件。
- 新验收 11：既存安全目标可使用有效 X 夹具库，保持现有正常启动断言；能防止把修复替换为一律禁用 DB 文件链接。三项均属可实施验收定义，本轮没有执行。
- 原同目录、目录链接、A 子树、S 族与不存在父目录规则保持；原正控顺延为 12–15，没有移除。新锁反例仍能通过这些验收，故整体完整性未通过。

## R1–R8 差异复核

| 项 | 结论 | 差异审查依据 |
|---|---|---|
| R1 引用与事实 | BLOCK | DB canonicalize 引用正确；“两个判定目录”仍未覆盖现役锁的最终链接落点，见 F1。 |
| R2 候选与权威一致 | PASS | 复用 41I；不换库、不自动迁移、启动排序不选及显式部署工具候选均未改。 |
| R3 二值验收 | BLOCK | 新增 9–11 各自可执行，原 F1 有锁；整套仍缺本轮锁链接反例。 |
| R4 落点与范围 | PASS | 复用 41I；load_manifest、测试、runbook owner 与资金/隔离/恢复帽/P0/Destination 边界未改。 |
| R5 单一判据 | PASS | 复用 41I；无开关、无失败后宽松兜底，新增目录同用一个冲突集合。 |
| R6 图与 owner | PASS | 复用 41I 的有效核验；本差异未改 #1769→#1781、#1818→#1781 或 #1823 无新增边的合同。 |
| R7 T/P 分级 | PASS | 复用 41I；仍属技术布局与现役写目标对齐，未添加政策授权。 |
| R8 考古完整性 | BLOCK | 原漏查 DB 文件链接已登记并修正；Q2/Q8 对现役写足迹的收敛仍缺锁最终链接。 |

## 证据复用与交回

41L 对本精确 head 的 diff-check、merge-tree、后缀计数与九锚句检查已通过（其基线 main=`71df2ae749664714e6bff10f8f71d9e15cc4c815`），本轮复用，未重跑。它们不消除 F1 的语义缺口。未重查最新 main 或外部依赖状态；上述图结论仅表示本次差异未破坏原已查合同。

`Prove It Works` 本轮落实为对精确差异、实际锁创建代码及 S 的测量/核验代码逐段核对，并构造能区分 Q₁/Q₂ 与真实写目标的静态反例；没有把锚句命中或“只多拒”当作语义证明。throughput checkpoint: n/a, read-only investigation。

请主控把 F1 交原 owner 修订后再审该窄差异。编号先后不构成语义依赖；不因 ADR 0058/PR #1821 未合入阻断。未跑 Rust、CI 或实验，未等待 CI；未改仓库、票、服务，未合入或关票。只写本目录的 REPORT.md 和 review.json。
