# PR #1594 非作者独立评审（#1588 tests 分片）

- PR：xy7365527-lang/NewChanlun#1594 `docs(#1588 tests): 三组基线失败测试的复现、根因与处置`
- 分支 `eng-debt-1588/tests` @ `16ef7da3ee`（基 `origin/main` `1d70db893f`），唯一改动 `.chanlun/review-results/eng-debt-20261020/tests/REPORT.md`（+187，纯文档）
- 评审者：claude-fable-5（非作者）· 2026-10-20 · 只评审，不改码、不推送、不评论 PR、不合入

## 结论：PASS-WITH-NITS

三组处置的事实基础全部经独立复核成立（见文末清单）。PR 不改任何代码，不存在掩盖问题、降覆盖或误改行为的风险；三组测试在 main 上的状态与报告所述一致。nits 集中在两点：① 报告对主控在名单文件末尾追加的 A 组执行指示保持沉默，偏离未写明；② 两处转票（A→#1488、C→#1493）目前只存在于报告里，没有可见交接，而其中两条是会影响对方合入门的新信息。均不阻塞本 PR 合入，但需主控落地。

## 两轴结论

### ① 正确性 / 风险

- **不掩盖问题**：A 组 16 条逐条给出 main 状态（11 条不存在、5 条绿）与 #1488 头 panic 点；B 组 22 条给出同一 MissingDependency 根因与源码 cfg 分支；C 组给出确定性证据，并额外揭露 m3 在「停产」与「不停产」下读数相同、已不是回归锁。这一条是本 PR 最有价值的发现，报告如实写明了「绿是偶然的」，没有借 CI 绿把问题关掉。
- **不降覆盖**：无 ignore、无 cfg 门控、无删测试、无改断言。
- **不误改行为**：无代码改动。
- **残余债务要说清**：B 组处置是「判环境问题并记账」，不是清零。Linux/WSL 本地全量 `cargo test` 仍会出 22 条红，只是红的原因已被证明是平台限定且 macOS CI 全绿。这在票面验收允许范围内（「判为环境问题并给证据」），但主控汇总 LEDGER 时应按「已记账未消除」而非「已清零」计。

### ② 是否满足 #1588 票面验收

| 验收项 | 本分片状态 |
|---|---|
| LEDGER 逐条 现象/复现命令与输出/根因/处置 | REPORT.md 三组均具备四要素，run-id 可对日志；是 LEDGER 的合格输入（LEDGER.md 本身由主控汇总，不在本 PR 范围） |
| 基线失败测试逐条清零或给出不可修理由并转票 | A：16 条逐条给理由，转 #1488；B：22 条给平台限定理由，判环境问题（未转票，票面允许）；C：判无稳定性问题，锁失效转 #1493 |
| m3 连续 N≥5 次 CI 稳定 | 满足：6 次 run 日志直证 `... ok`，另 3 次「default targets」步 success（该步 `--no-fail-fast`），WSL 10 进程同读数 |
| 每个修复非作者独审 | 本文 |

## 逐条发现

### F1 [中] 报告未回应主控追加的 A 组执行指示，偏离未写明
- 位置：`REPORT.md` §A-3（`.chanlun/review-results/eng-debt-20261020/tests/REPORT.md:53-58`）
- 理由：`/Volumes/AgentStorage/eng-debt-20261020/1488-baseline-failures.md` 末尾「主控（owner_1588）给 tests 分片的指示」要求：先在 main 复现（已做），然后**从 #1488 合并面起独立分支 `eng-debt-1588/tests-1488` 逐条做根因与修复**，避开 #1488 在改的文件，「或若根因判断为需裁定则转票」；并要求 **A 与 B/C 分开 PR**。文件 mtime 14:43:58，PR 提交 15:11:29，指示在提交前已存在。报告既没建 tests-1488 工位，也没把 A 拆出，更没有一句话说明为何不按指示走。
- 我的判断：偏离本身可辩护——指示点名的基分支 `codex/1488-budget-full-chain-fix-v2-20261018` 远端不存在；#1488 头在本分片运行期间仍在移动（`c2bdf9b662` 13:04 → `bba547492d` 14:55）；16 条红的「测试对还是实现对」确属 #1488 预算/所有权合同的裁定面，走指示里的「需裁定则转票」出口说得通；A 组没有代码，「分开 PR」的目的（依赖 #1488 先合）也就不成立。但这些都应由报告自己写出来交主控裁定，而不是沉默。
- 建议：A-3 补一小节「与主控指示的关系」，写明：未建 tests-1488 的原因、走的是「需裁定则转票」出口、A/B/C 合并 PR 的原因（A 无代码）。由主控确认是否接受。

### F2 [中] 两处转票只落在报告里，缺少对 #1488 / #1493 的可见交接
- 位置：`REPORT.md` §A-3「查重」与「给 #1488 的一句话」（:57-58）；§C-4 第 3 点（:162）
- 理由：报告说「归属已在 #1488 票面在案，不另开票」，但 #1488 票面第 1174 行只是「宽对拍里 20 条基线既有失败的归属」待办，并不包含本报告的新信息：**5 条在 main 上绿的测试（r11/r13/r14/driver/request_audit r11）会被 #1488 合入打红**。同理，「m3 已不是停产锁、#1582 合入前应补段层连续性锁」对 #1493 也是新信息，#1563/#1564 当时依赖的「m3 保持红作锁」前提已失效。这两条都是对方的合入门条件，若只留在分片报告里，极可能在 #1488 squash 合入 / #1582 合入时丢失。
- 建议：主控（非本评审者）把两条分别写进 #1488 与 #1493 的票面，作为合入门硬条件；§E 第 3 点问「是否升级为 #1493 合入门硬条件」，我的回答是**是**。

### F3 [低] B-4 否决 cfg 门控的理由①不成立，结论不变但论据应修正
- 位置：`REPORT.md:98`
- 理由：报告称门控需「给 22 个测试和约 10 个辅助函数逐个加门，否则 dead_code」。实际测试模块经 `financial_session.rs:1888-1890` 的 `#[cfg(test)] #[path = "financial_session_tests.rs"] mod tests;` 挂载，在测试文件内加一层 `#[cfg(target_os = "macos")] mod session_dependent { … }`（或把 4 条平台无关用例拆到外层）即可，一个属性，无 dead_code 问题。理由②（Linux 上「诚实地红并写明原因」优于「不出现」）与理由③（ci.yml `--skip` 与 ci-time 分片的冲突面）仍成立，我同意不加门。
- 建议：删掉或改写理由①，避免后续读者据此判断门控成本。

### F4 [低] C-4 对 #1564 的引述加了引号但不是原文
- 位置：`REPORT.md:149`
- 理由：报告写「#1564 明写『m3 保持红，作为线段停产的回归锁，修复归 #1493』」。PR #1564 正文原文是「`m3_partition_synthetic_holds`：生产行为问题，不改测试，归 #1487/#1493 … 换 seed 或加长输入让 m3 变绿会掩盖这些缺陷，所以测试不动；#1493 落地后应重跑」。意思一致，但引号内不是原话。
- 建议：改为原文或去引号转述。

### F5 [低] C-4 的段数探针只在本地提交，第三方不可复现
- 位置：`REPORT.md:151`
- 理由：`645d66de23`（main 探针）与 `05c3cadd3d`（#1582 探针）未推送（已核实 `git ls-remote` 无对应分支），`14 vs 32` 段数读数只能信 run 日志。报告已如实声明「未推送」，不算隐瞒。
- 建议：把 `probe1420_segments` 的 diff 作为附录贴进 REPORT.md，或推到仓库已有的 `probe/*` 命名空间下，便于 #1493 据此补锁。

### F6 [备注] 名单文件与报告使用的 #1488 分支不同
- 报告用 `origin/codex/1488-uo-ci-20261019`（存在，头 `bba547492d`），名单主控指示用 `codex/1488-budget-full-chain-fix-v2-20261018`（远端不存在）。报告的选择正确，但宜在 A-0 注一句，免得主控对不上。

## 我实际复核过的事实与命令

均在 `/tmp`（gh）或工位 `/Volumes/AgentStorage/eng-debt-20261020/tests`（git）执行；Rust 用 `~/.local/bin/nc-remote`。

1. **PR 元数据**：`gh pr list --head eng-debt-1588/tests` → #1594；`gh pr view 1594 --json files,commits,body` → 单文件单提交，纯文档。
2. **票面**：`gh issue view 1588` → 验收五项如上表。
3. **名单文件与时序**：`stat` 得 mtime 2026-10-20 14:43:58；`git log -1 16ef7da3ee` 得提交时间 15:11:29 → 主控指示先于 PR 提交存在。
4. **祖先关系**（`git merge-base --is-ancestor`）：`959d860065`∉main ✔；`e79bdb2a52`∉main ✔；`279b7c8708`∉`b34b0278ee`、∈`b63a3fd586` ✔（C-1 转绿点订正成立）。
5. **16 条在 main/#1488 的存在性**：`git grep -l "fn <名>\b" HEAD|origin/codex/1488-uo-ci-20261019 -- rust/src` 逐条 → main 0/1488 1 共 11 条，main 1/1488 1 共 5 条，与 A-1 完全一致。
6. **A 组 run 日志**（`/tmp/nc-remote/`）：`20261020-144647-1d70db893f.log` → `ok. 103 passed; 0 failed; 5 ignored`；`20261020-145629-c2bdf9b662.log` → `FAILED. 236 passed; 16 failed; 5 ignored`，失败名单 `sort|diff` 名单文件 16 条 → 空；`20261020-145230-25333c0f2a.log` → `1 passed; 4 failed`。
7. **B 组**：`20261020-144202-1d70db893f.log` → `4 passed; 22 failed; 3 ignored`，`grep -c 会话保留量读数当前只在macOS实现` = 22 = panicked 数。**重跑** `nc-remote eng-debt-1588/tests rust cargo test --locked --features s_session --bin s_structure_session v3::financial_session`（新 run `20261020-152441-16ef7da3ee`）→ 同为 4/22/3，MissingDependency 22 处。源码 `financial_session.rs:496-506` cfg 分支属实；`ci.yml:271` `--skip v3::financial_session`、`ci-macos.yml:73-74` 专跑属实。
8. **B macOS CI**：`gh run view 39358379664 --log` → head `6badaa6049` success，`ok. 26 passed; 0 failed; 3 ignored; 518 filtered out`。`git diff --stat 6badaa6049 1d70db893f -- rust/src/bin/s_session_v3/` 为空 ✔。
9. **C CI**：`gh run view 39358379553 --log` → m3 `... ok` ×2，`507 passed; 29 filtered out`、`167 passed` 属实；`39321352559` → m3 `FAILED` ×2（转绿前一次）；`39379637876`/`39380854169`/`39380862852` `--json jobs` → 「cargo test — default targets」步 success（整 run 仍 in_progress，日志未开放，与报告一致）。
10. **C WSL**：`20261020-144314-1d70db893f.log` → `m3 ... ok` 10 次，`|ledger|=1 kept=1 same_bar=0 nonpos=0 |records|=1` 10 次完全相同，EXIT=0。**重跑**单次 → `ok`，EXIT=0。
11. **C 探针日志**：`20261020-145621-645d66de23.log` n=3000 `segments=14 last_end=1708 ledger=1`；`20261020-150044-05c3cadd3d.log` `segments=32 last_end=2958 ledger=1`；`20261020-144723-8f86b657e6.log` #1582 头 m3 ok、ledger=1。两探针 ref 为本地分支，远端无（`git ls-remote --heads origin | grep probe`）。
12. **m3 源码**：`l3_delta_r_alpha.rs:2535-2567`，唯一非空门 `assert!(!ledger.is_empty())`，其余为分区守恒断言；文档注释说明它测的是 M3 分区正确性，本就不是停产锁 → C-4 判断成立。
13. **关联票**：#1563 CLOSED、#1564 MERGED（原文见 F4）、#1493 OPEN、PR #1582 OPEN（头 `ff2d1bac7b`，已越过报告用的 `8f86b657e6`）、#1520 OPEN 0 评论、#1488 票面第 1174 行待办属实。远端 1488 分支列表中无 `codex/1488-budget-full-chain-fix-v2-20261018`。
