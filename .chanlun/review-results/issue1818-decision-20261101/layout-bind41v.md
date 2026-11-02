# #1818 / PR #1826 41T F1 返修报告（layout-bind41v）

时间（date -u）：2026-11-01T01:02:42Z（push 后取）

## 结果

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1826 （OPEN、Draft、MERGEABLE）。
- 新精确 head：`8ce09b7b31aa64ec6a299e5a66e0f76c63a5b265`，普通追加提交，父为 `07fb67f116c002f6604214f10ddebb55e58a6500`。未 force、rebase、merge。开工前 HEAD 与分支 `codex/1818-adr0059-publish` 一致，工作区干净。
- 只改 `docs/adr/0059-production-session-layout-contract.md`（+26/−22）。复审差异：`git diff 07fb67f116 8ce09b7b31 -- docs/adr/0059-production-session-layout-contract.md`。
- PR 正文已同步。改前原件 `pr-body.before.md`，现文 `pr-body.md`，均在本目录。

## 修订内容

采纳 41T 的 Apple XNU 固定源码证据。撤回前轮「bind 不跟随最终链接、所有 Unix 相同」的断言。规则取最小统一形式，不按平台分支。

- **Q2 第四类**：bind 对父目录各级链接一律解析。最终条目是否跟随，各 Unix 平台不同。XNU commit `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea` 中 `unp_bind` 以 `FOLLOW` 调 `namei`，悬空目标在解析后的父目录建 socket（`uipc_usrreq.c:1189-1236`；`vfs_lookup.c:154-160`、`:1040-1044`、`:1601-1609`）。其余 Unix 平台标为本轮未逐一核实。socket 创建目录取两处，两处都判，对跟随与不跟随两种语义都只多拒不多放。标注为静态源码证据，未运行验证。
- **裁定第 3 条**：保留 `Q₄ = canon(parent(k))`，新增 `Q₅ = parent(canon(k))`。canon 沿用第 1 条的悬空链接语义。k 不是链接时两者相等。
- **裁定第 4、8 条**：拒绝集合纳入 `Q₅`，与 `Q₄` 同用一个 M 判据。错误给出命中的 `Q₄` 或 `Q₅`。
- **Q8 与 Q2 小结**：socket 句改为同取两处。「三类布局」第三类补上「或是指向那里的悬空链接」。
- **新验收 17** `layout_rejects_e_socket_final_dangling_symlink_into_s_parent`：`e.socket` 是安全目录内的悬空链接 `alias.sock`，指向不存在的 `P/e.sock`。只有 `Q₅` 命中 P。断言 stderr 含 `InvalidDomain`、`e.socket`、`s.db`，P 项集不变，任何域文件都不存在。
- **新验收 18** `layout_rejects_b_socket_final_dangling_symlink_into_admission_state`：`chongs[0].socket` 是悬空链接，指向不存在的 `A` 下 `b.sock`。断言 stderr 含 `chongs[0].socket`、`s.admission_job_limits`，目标未建，无域文件。
- **正控保留**：socket 在 P 非族子目录（现 20）与 `s.socket` 在 P（现 21）不变。
- **编号同步**：原 17–29 顺延为 19–31。状态行「验收 9–19」改为「9–21」，并改为发布审查四轮。考古第 7 条「补验收 17」改为 19。第 9 条「补验收 18、19」改为 20、21。新增考古第 10 条，记 41T 来源与撤回理由，放在第 9 条未核清单之后，不改「未核足迹见第 9 条」的指向。PR 正文的「验收 25、26」改为 27、28。

没有加平台回退、开关、socket 清理或启动排序。DB、lock、直接 socket 规则与第 9 条四项未核足迹未改动。

## 实际验证

- `git diff --check` 通过。`git merge-tree --write-tree origin/main HEAD` 无冲突，集成差异仍只有本 ADR。
- 验收编号 1–31 连续。ADR 内对验收编号的引用只有状态行与考古第 7、9、10 条，均已核对。
- 验收 27：后缀计数为 `1`。验收 28 十一条锚句 `rg -c -F` 计数依次为 3/17/2/2/2/3/2/3/4/3/3，均 ≥1。
- 未跑 Rust、CI、socket 探针或重型实验。XNU 行号直接沿用 41T 报告，本轮没有重读 Apple 仓库。

## 交独审

- 候选 head `8ce09b7b31aa64ec6a299e5a66e0f76c63a5b265`。作者为本子代理（Claude 家族），交 Astra 非作者独审。
- 重点核三项。第一，`Q₅` 在悬空链接下是否等于 XNU 的实际创建目录。第二，验收 17、18 能否检出只判 `Q₄` 的实现。第三，「其余 Unix 未逐一核实」的适用范围表述是否准确。
- 实现与运行证据归 #1781。合后关票与 squash 正文不含自动关票关键词，由主控负责。
