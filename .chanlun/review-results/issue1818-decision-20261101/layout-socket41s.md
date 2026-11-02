# #1818 / PR #1826 41R F1 返修报告（layout-socket41s）

时间（date -u）：2026-11-01T00:51:27Z（push 后取）

## 结果

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1826 （OPEN、Draft、MERGEABLE）
- 新精确 head：`07fb67f116c002f6604214f10ddebb55e58a6500`，普通追加提交，父为 `163fb363a290ecf210a265058f1af71105ca73ce`；未 force、rebase、merge。开工前核 HEAD = `163fb363a2`、分支 `codex/1818-adr0059-publish`、工作区干净，无他人改动。
- 工位：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。
- 只改 `docs/adr/0059-production-session-layout-contract.md`（+42/−23，`git diff 163fb363a2 07fb67f116`）。PR 正文已同步，正本 `pr-body.md`，改前原件 `pr-body.before.md`，均在本目录。
- 性质：已 land_t 技术合同的发布返修，不是重新草稿 attempt，不转 P；不开新票、不改边。

## 修订内容（对应 41R F1 最小修订）

- **撤回缩窄**：删除 Q2 末句「本条只覆盖库派生路径」与考古第 8 条「三类不纳入、E/B socket 留待另裁」。Q2 引言从「只有下列三类」改为「已核的……有下列四类……已知未核的足迹见考古冲突第 9 条」。
- **Q2 第四类 E/B socket**：socket 由 E/B 从经济 manifest 读，E 取 `e.socket`，B 取 `chong_id` 等于 `economic.b[i].chong` 的 `chongs[j].socket`（`store.rs:281-296`）；`Config::load` 只验绝对且不重复（`:158`、`:162`、`:176`）；launcher 不等 S 拉起 E/B（`production_session.rs:455`、`:467`、`:480`）；serve 只等自身库/epoch/audit（`service.rs:1500-1503`），`:1524` → `session_transport.rs:617-635` bind。bind 只解析父目录、最终条目已存在即 `EADDRINUSE` 不跟随（bind(2)/unix(7) 平台语义，标注未运行验证），故 `Q₄ = canon(parent(k))`，不套用普通 open 的最终链接规则（Q8 同步写明）。
- **裁定第 3 条**新增 socket 段：launcher 在 `load_manifest` 内只读经济 manifest；读不出、缺字段或找不到对应 `chong_id` 时拉起前 `InvalidDomain`（E/B `Config::load` 本会确定性拒绝，`store.rs:158`、`:176`、`:286-287`）。第 4 条拒绝集合加 `Q₄`；第 8 条错误点名 `e.socket` 或 `chongs[<j>].socket` 并给出 `Q₄`；第 9 条注明读经济 manifest 只读、仍在返回 `Ok` 前。
- **保留 s.socket 原例外**：Q4 与第 6 条补一句「只因 S 自己在 driver open 之后才 bind（`writer_cli.rs:725`、`:802`），不涵盖 E/B」，锚句 `不比较 `s.socket`` 保留。
- **验收**：新增拒绝 13 `layout_rejects_e_socket_in_s_parent`（41R 静态反例：安全库＋`e.socket=P/e.sock`，stderr 含 `InvalidDomain`、`e.socket`、`s.db`，P 项集不变、无域文件）、14 `layout_rejects_b_socket_under_admission_state`（`chongs[0].socket`、`s.admission_job_limits`）、15 `layout_rejects_e_socket_under_s_family_entry`、16 `layout_rejects_b_socket_parent_symlink_into_s_parent`（父目录链接语义）；正控 18 `layout_accepts_sockets_in_non_family_subdir_of_s_parent`、19 `layout_allows_s_socket_in_s_parent`。原 13→17，原 14–23→20–29。锚句加 `socket 目录`。
- **考古第 9 条**：记 41R 来源与撤回理由；TestOnly 故障标记（`store.rs:395-406`、`:268-270`，launcher 拒 TestOnly 落点未核）、SQLite 临时文件、金融读子进程（`economic_session/finance_transport.rs:179`、`:286`）、grep 覆盖外模块，照实列为未核、未定性为反例，并写明「以上不是已穷举的清单」。末节「没有事实缺口」改为未核项照实留档、本轮不另开票。
- **状态行**：「原裁定经两轮审查，第二轮 PASS」；发布审查三轮漏项；订正范围 Q2、Q4、Q8、裁定 1/3/4/6/8/9、验收 9–19。

## 实际验证

- `git diff --check` 通过；`git merge-tree --write-tree origin/main HEAD` 无冲突（origin/main `71df2ae749664714e6bff10f8f71d9e15cc4c815`），集成差异仍只有本 ADR。
- 验收 25 后缀计数 `1`；验收 26 十一条锚句 `rg -c -F` 计数 3/15/2/2/2/3/2/2/4/3/3。
- 编号直接依赖：考古第 7 条「补验收 9、10、13」→「9、10、17」；第 9 条「补验收 13–16、18、19」；ADR 内无其他对验收编号的引用。PR 正文「验收 19、20」→「25、26」，「十条」→「十一条」。
- 新引用行号本轮逐行读过：`store.rs:158`、`:162`、`:176`、`:268-270`、`:281-296`、`:395-406`；`service.rs:1500-1503`、`:1524`；`session_transport.rs:617-635`；`production_session.rs:257`、`:455`、`:460`、`:467`、`:471`、`:480`；`writer_cli.rs:725`、`:802`；`finance_transport.rs:179`、`:286`（订正了旧稿路径缺 `economic_session/`）。
- 未跑 Rust、CI、重型窗口。push 时本机 pre-push 提示本地 main 落后 origin/main，属本地引用陈旧提示，推送成功。

## 待独审与剩余

- 候选 head `07fb67f116c002f6604214f10ddebb55e58a6500`，差异 `git diff 163fb363a2 07fb67f116 -- docs/adr/0059-production-session-layout-contract.md`。作者为本子代理（Claude 家族），交 Astra 非作者独审。重点：Q₄ 是否完整对应 E/B 实际 socket 取值路径；bind 只解析父目录的表述是否准确；验收 13–16 能否检出漏判 socket 的实现，18–19 正控是否与第 6、7 条一致；第 9 条未知项是否如实。
- bind 最终条目语义取自平台文档，未在 macOS/Linux 运行验证；归 #1781 实施验收沿受管构建。
- 未核足迹（第 9 条四项）不阻断本稿，若主控认为需定性，归主控决定是否另开调查。
- 合后动作（#1818 关票、#1781 范围、#1323 Notes）归主控；squash 正文须不含自动关票关键词。
