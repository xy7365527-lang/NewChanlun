# #1818 / PR #1826 增量独审 PASS

本轮精确 head 为 `8ce09b7b31aa64ec6a299e5a66e0f76c63a5b265`，差异基线为其直接父提交 `07fb67f116c002f6604214f10ddebb55e58a6500`。41T 的唯一 F1 已闭合，未发现新增阻断或与既定正控冲突的误拒。此结论只验收规则文书，#1781 仍负责实现与运行证据。

只读工位 `/Users/silencehan/.codex/worktrees/layout-publish41e/NewChanlun`。核验时本地 HEAD 与 GitHub PR head 均为上述提交，工作区干净。差异只有 `docs/adr/0059-production-session-layout-contract.md`，26 行增加、22 行删除。以下 ADR 行号均指该精确 head。作者为 Opus，本审查为非作者 GPT 进程，独立作出裁决。

## 最终悬空链接反例已被 Q₅ 拒绝

ADR 第 95 行规定最终悬空链接先取目标，再递归求 canon，相对目标按链接目录解析。第 109 行据此新增 `Q₅ = parent(canon(k))`，同时保留 Q₄。第 110–113 行将两者并入同一 M 拒绝集合。第 119–120 行要求错误点名命中的 socket 字段与 S 字段，并在 `load_manifest` 返回 Ok 前只读拒绝。

代入 41T 反例，P=`/abs/s`，A=`/abs/s-state`，库和锁均安全，`k=/abs/e-run/alias.sock` 指向不存在的 `/abs/s/e.sock`。Q₄=`/abs/e-run`，Q₅=`/abs/s`。Q₅ 等于 P，故在任何 init、bind 或域文件创建之前以 InvalidDomain 退出。将 B 的目标换成 `A/b.sock`，同样由 Q₅ 等于 A 拒绝。相对链接或多级最终链接也沿第 95 行解析到同一目标目录。

平台依据复用 41T 已固定的 Apple XNU commit `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea`。`bsd/kern/uipc_usrreq.c:1189–1236` 的 CREATE、FOLLOW、namei 与 vn_create 链，以及 `bsd/vfs/vfs_lookup.c:154–160,1040–1044,1601–1609` 的最终链接和缺失目标规则，支持以上创建目录。当前 ADR 第 65 行已撤回所有 Unix 均不跟随的断言，并把其他 Unix 标为未逐一核实。未把上游 commit 冒充本机内核，也未宣称运行验证。

## 误拒边界与正控保持一致

同时检查 Q₄、Q₅ 会保守拒绝部分在不跟随最终链接的平台上本来就会 bind 失败的布局。该代价在第 65、79 行已明示，并沿用本合同的跨平台超集比较。它没有把安全目录本身扩成 M，也没有引入失败后换判据的宽松通道。

- 普通安全 socket 的 Q₄、Q₅ 相等且均不在 M。验收 20 的 P 内既有非族、非 A 子目录仍放行。
- 最终链接的两个目录都安全时，两者均不命中 M，布局规则仍放行。若目标已存在，后续 bind 的占用失败与布局拒绝是两件事。
- 字面路径在安全子目录、最终链接却指进 P 或 A 时，Q₅ 命中的是实际潜在创建目录。第 118 行对 M 外目录的放行承诺不适用于这个 M 内目标，因此没有合同矛盾。
- `s.socket` 不进入 E/B socket 集合。第 117 行例外和验收 21 保持不变，S 自己在 driver open 后 bind 的既有依据继续有效。

## 验收能区分旧缺陷

验收 17、18 分别位于第 146、147 行。二者均把库、锁和 socket 词法父目录放在安全处，只有 Q₅ 命中。只实现 Q₄ 的旧检查无法在 load_manifest 阶段产生约定的 InvalidDomain 与字段组合，即使操作系统后来拒绝 bind，也不能冒充通过。验收还要求目标未创建、域文件未创建，17 额外检查 P 项集不变。

验收 1–31 连续。考古第 7 条指向 9、10、19，第 9 条指向 13–16、20、21，第 10 条指向 17、18，均对应实际条目。状态行的 9–21 覆盖修订范围。PR 正文最新提交段写明 17、18 和总数 31，Verification 与 Blast Radius 引用 27、28。旧提交段的历史编号由最新段说明顺延，不改变现行清单。

## 规则结论与复用范围

| 项 | 裁决 | 依据 |
|---|---|---|
| R1 引用与事实 | PASS | 第 65 行按 41T 固定 XNU 来源修正平台事实，Q₅ 与解析后的创建父目录一致。 |
| R2 候选与权威 | PASS | 复用 41T、41R 未变结论。不迁移 S，不改锚，不增加可调字段。 |
| R3 二值验收 | PASS | 新 17、18 独立隔离 Q₅，并要求启动前拒绝及无副作用。20、21 正控保留。 |
| R4 落点与政策 | PASS | load_manifest、测试和 runbook 落点不变，实现仍归 #1781。 |
| R5 单一判据 | PASS | Q₁ 至 Q₅ 共用第 4 条的一个 M 拒绝集合。没有平台回退或放行开关。 |
| R6 图与 owner | PASS | 复用未变的图结论。本轮未修改边，也未重查 live 全图。 |
| R7 T/P 分级 | PASS | 修正已纳入 bind 调用的技术事实。复用 41R 对 land_t 后返修的分级结论。 |
| R8 考古 | PASS | 41T 已证明的最后一跳缺口闭合。DB、lock、直接 socket 调用检查未变，复用原证据。 |

未变的 Q₁、Q₂、Q₃、L₁、L₂、bundled SQLite 与直接 socket 结论来自 `/Volumes/AgentStorage/layout-delta41t/REPORT.md` 及其引用的 `/Volumes/AgentStorage/layout-review41r/REPORT.md`。保留现役 macOS/Linux、默认 VFS 和编译配置适用范围。TestOnly marker、SQLite 临时文件、金融读子进程与未追模块仍是第 33–39 行的未核事项，本轮不把它们改写成新阻断或已核事实。

## 实际检查与交付

读取最小差异、完整 ADR、41T 与 41V 原件，并核对 PR 当前 head 与正文。运行 git diff --check，无输出且退出 0。按精确提交枚举验收编号得到 1–31，按正本后缀计数得到 1。Apple 源码、DB、lock 与直接 socket 证据按授权复用，没有重复考古。

应用 `Prove It Works`，直接读精确 head 的规则并独立静态代入反例与正控，不以作者自评替代裁决。吞吐检查点为只读增量审，无实现阶段。纯 ADR 无代码注释，no-comments 无适用对象。未运行 Rust、CI、socket 实验或并发复现。ADR 的实现、runbook 与 CI 验收留给 #1781，不能由本 PASS 认领已完成。

主控可将此 PASS 作为 PR #1826 上述精确 head 的文书独审证据。合入、ADR 编号顺序和合后图动作仍由主控按既有发布安排办理。本轮仅写 `/Volumes/AgentStorage/layout-final41w/REPORT.md` 与 `review.json`。

核验时间取实际 `date -u` 输出，为 `2026-11-01T01:05:32Z`。
