# #1818 / PR #1826 41I F1 窄修订报告（layout-repair41l）

时间（date -u）：2026-11-01T00:24:19Z

## 结果

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1826 （OPEN、Draft、MERGEABLE）
- 新精确 head：`a2cf513c25c45f1e835d7732425bdf024dd79b7f`（普通追加提交，父为 `b636af32e84c9e64701d9f7c4d3c482dd83bf434`；未 force、未 rebase）
- 工位/分支：`/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`，`codex/1818-adr0059-publish`；开工前核 head=origin=PR=`b636af32e8`、工作区干净。
- 只改 `docs/adr/0059-production-session-layout-contract.md`（+22/−17）；PR 正文同步更新，正本 `pr-body.md` 在本目录。

## 修订内容

修法是在原判定目录之外加一个，而不是替换：`Q₁ = canon(parent(d))` 原样保留，新增 `Q₂ = parent(canon(d))`，两者任一命中拒绝条件即 `InvalidDomain`。d 非链接时 Q₁=Q₂，所以原有每条拒绝/放行用例（目录链接 7、不存在路径 8、A 子树 4–5、S 族 6、正控 12–15）判决不变；新增判定只会多拒。

- 裁定第 1 条 canon：明写最终条目是链接时也解析；悬空最终链接先取目标（相对目标按链接所在目录，多级逐级）再求 canon。
- 裁定第 3 条：定义 Q₁、Q₂，并指明 Q₂ 与 `process_lock` 同源（`rust/src/session_transport.rs:43-62`：已存在 d 先 `canonicalize(d)` 再拼 `.writer.lock`）。
- 裁定第 4 条：拒绝条件作用于 Q₁ 或 Q₂。第 8 条：并发写域一侧报命中的判定目录。
- Q2 推理链补现役写目标：X `x_execution_session.rs:176`、E/B `rust/src/economic_session/service.rs:1569` 调用 `process_lock`；launcher 原样传 manifest 路径 `production_session.rs:462-463`、`:471`、`:483`。悬空链接两处都写：锁在 Q₁，`Connection::open(d)` 经 `open(O_CREAT)` 在目标目录建库文件（推论，依 POSIX 语义，未运行验证）。
- Q8：补最终文件链接与悬空链接；出处订为 `admission_anchor.rs:1292-1294`（:1292 注释自称与 process_lock 同语义）。
- 考古冲突清单加第 7 条，记录原稿漏查与 41I 来源；状态行改为「订正部分待独立审查」（原写「已通过独立审查」，对修订后文本不再成立）。
- 验收新增 9 `layout_rejects_x_db_file_symlink_into_s_parent`（41I 反例：安全目录 `alias.db` → 已存在 `P/x.db`，核 `InvalidDomain`、`x.db`、`s.db`、P 目录项不变、无域文件）、10 `layout_rejects_e_db_dangling_symlink_into_admission_state`（悬空链接指向 A 内，核 `s.admission_job_limits`）、11 `layout_accepts_db_file_symlink_between_safe_dirs`（防止以「一律拒链接」代替解析）。原 9–18 顺延为 12–21；锚句加 `实际库文件所在目录`。外部旧引用「验收 14/15」现为 17/18。

T/P：仍是已有 InvalidDomain 规则的输入口径对齐现役写者，无新政策、无开关、方向只多拒，未超出 T 范围。

## 实际验证

- `git diff --check origin/main...HEAD` 通过；`git merge-tree --write-tree origin/main HEAD` 无冲突（origin/main `71df2ae749664714e6bff10f8f71d9e15cc4c815`）。
- 验收 17 后缀计数 `1`；验收 18 九条锚句 `rg -c -F` 计数 3/8/2/2/2/3/2/2/3。
- 引用行号逐处读源码核对：`session_transport.rs:43-67`、`x_execution_session.rs:171-178`、`service.rs:1561-1569`、`production_session.rs:455-490`、`admission_anchor.rs:1290-1296`。
- 未跑 Rust、未等 CI；未改实现、System 工位、main、票、隔离或服务。push 时本机 pre-push 提示「本地 main 落后 origin/main」，属本地 main 引用陈旧提示，不影响本分支推送结果。

## 待独审

候选 PR #1826 精确 head `a2cf513c25c45f1e835d7732425bdf024dd79b7f`，对 `b636af32e8` 的差异即本次修订。作者为本 Claude 家族子代理，独审宜交非作者 gpt-6-astra（异家族）。重点：Q₁∪Q₂ 是否完整覆盖现役写目标（含悬空链接）；新增验收是否二值可执行；原裁定其余条款与 R2/R4–R7 结论是否未受扰。合后动作（#1818 关票、#1781 范围、#1323 Notes）仍归主控。
