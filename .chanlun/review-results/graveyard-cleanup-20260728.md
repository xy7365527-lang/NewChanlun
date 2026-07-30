# 坟场清理执行报告（#502 裁定）——2026-07-28

- 执行口径：即清三条件 AND（票已关 ∧ worktree 干净 ∧ `git rev-list --count main..<branch>`=0）；无票号 agent-*/wf_* worktree 干净即清；深度分叉 4 个先挪 graveyard 再删。
- 铁律遵守：只用 `git worktree remove`（拒绝即跳过，不 --force）与 `git branch -d`（不 -D）；主仓与当前 HEAD（main）不碰；零提交。
- 快照复核：执行前已逐项重核——票状态（gh issue view，repo xy7365527-lang/NewChanlun）、worktree 脏文件（git status --short）、分支计数（rev-list）。复核时 main=e294c5b85a。
- 第一批已完成（不重复）：worktree agent-a8bbb19daba5402bb；分支 capture-ratio-verify-148、ticket-472。
- graveyard：`/tmp/newchanlun-graveyard-20260728/`（目录已存在，含他批产物）。

## 批二：无票号 agent-* worktree（干净即清，12 项）

复核结论：均无票号、脏文件=0。分支保留在仓（删 worktree 不丢任何 commit）。

| # | worktree 路径 | 挂载分支 | 结果 |
|---|---|---|---|
| 1 | .claude/worktrees/agent-a0389ec5753435684 | worktree-agent-a0389ec5753435684 | ✅ 已删 |
| 2 | .claude/worktrees/agent-a19bbaf9995460c33 | worktree-agent-a19bbaf9995460c33 | ✅ 已删 |
| 3 | .claude/worktrees/agent-a3c1604fc473623ee | worktree-agent-a3c1604fc473623ee | ✅ 已删 |
| 4 | .claude/worktrees/agent-a487440bf7bbca944 | worktree-agent-a487440bf7bbca944 | ✅ 已删 |
| 5 | .claude/worktrees/agent-a7e569944b1d9a46d | worktree-agent-a7e569944b1d9a46d | ✅ 已删 |
| 6 | .claude/worktrees/agent-ab4db18d22db8694c | worktree-agent-ab4db18d22db8694c | ✅ 已删 |
| 7 | .claude/worktrees/agent-ace0022ed478bed16 | worktree-agent-ace0022ed478bed16 | ✅ 已删 |
| 8 | .claude/worktrees/agent-ad21219d4b564138e | worktree-agent-ad21219d4b564138e | ✅ 已删 |
| 9 | .claude/worktrees/agent-ae517ccfbf0a8080a | worktree-agent-ae517ccfbf0a8080a | ⏭ 跳过：删除前终检突现 7353 脏文件（疑有活跃进程写入，判据②在制品） |
| 10 | .claude/worktrees/agent-aeac8e36d80879e2b | worktree-agent-aeac8e36d80879e2b | ✅ 已删 |
| 11 | .claude/worktrees/agent-aeb1c6afb338546fd | worktree-agent-aeb1c6afb338546fd | ✅ 已删 |
| 12 | .claude/worktrees/agent-af41a2af699aa4763 | worktree-agent-af41a2af699aa4763 | ✅ 已删 |

## 批三：分支（票已关 ∧ 未合入=0 ∧ 未挂载，3 项）

| # | 分支 | 票号/状态(gh 复核) | main..branch | 结果 |
|---|---|---|---|---|
| 1 | impl/397-stop-in | #397 CLOSED | 0 | ✅ 已删（was 15ad9d5ecc） |
| 2 | impl/400-has-residual | #400 CLOSED | 0 | ✅ 已删（was e3c837bc47） |
| 3 | issue-181-selldecision-cleanup | #181 CLOSED（挂载 worktree 第一批已删） | 0 | ✅ 已删（was 05d56f2d2b） |

## 批四：深度分叉 4 分支（钉 19b4015927，领先 main 1171）——先挪 graveyard 再删

处置：先 `git archive <branch> | tar -x -C /tmp/newchanlun-graveyard-20260728/<name>/`；aec7bff16fc502aff 另有挂载 worktree（脏 1 文件 `.claude/settings.local.json`，worktree 体积 1.9G 不整包，单独复制脏文件入 graveyard）。随后尝试 `git worktree remove` / `git branch -d`，拒绝即记录跳过（铁律不 force）。

| # | 分支 | 挂载 worktree | graveyard 产物 | 删除结果 |
|---|---|---|---|---|
| 1 | worktree-agent-a3eac13cd25744e5d | 无 | ✅ /tmp/newchanlun-graveyard-20260728/worktree-agent-a3eac13cd25744e5d/（1.9G） | ⏭ `git branch -d` 被拒：not fully merged（铁律不 -D，跳过） |
| 2 | worktree-agent-a8bbb19daba5402bb | 无（第一批已删） | ✅ /tmp/newchanlun-graveyard-20260728/worktree-agent-a8bbb19daba5402bb/（1.9G） | ⏭ `git branch -d` 被拒：not fully merged（跳过） |
| 3 | worktree-agent-aec7bff16fc502aff | .claude/worktrees/agent-aec7bff16fc502aff（脏1） | ✅ 分支 archive（1.9G）+ 脏文件 settings.local.json 另存 /tmp/newchanlun-graveyard-20260728/agent-aec7bff16fc502aff-dirty/ | ⏭ worktree remove 被拒（含未提交改动，不 --force）；branch -d 被拒（仍被 worktree 占用）（均跳过） |
| 4 | worktree-agent-af3878feb35eb9a1a | 无 | ✅ /tmp/newchanlun-graveyard-20260728/worktree-agent-af3878feb35eb9a1a/（1.9G） | ⏭ `git branch -d` 被拒：not fully merged（跳过） |

## 候选复核后跳过清单（原因逐项）

### worktree 跳过

| worktree | 跳过原因 |
|---|---|
| /private/tmp/codex-work-292-trigger | 脏 1（在制品，判据②）→ 分支 codex/292-trigger-source 连带跳过 |
| /private/tmp/wt-486 | 脏 1 → 分支 ticket-486 连带跳过 |
| /private/tmp/wt-434-level-origin | 脏 17 → 分支 issue-434-level-origin-removal 连带跳过 |
| .claude/worktrees/facea-impl-113 | 脏 2 → 分支 facea-impl-113-20260623 连带跳过 |
| .claude/worktrees/agent-a3eac13cd25744e5d（挂 exp-155-wf7-l1-gate） | 脏 2 → 分支 exp-155-wf7-l1-gate 连带跳过 |
| .claude/worktrees/agent-af3878feb35eb9a1a（挂 t5-149-shortdiff） | 脏 1 → 分支 t5-149-shortdiff 连带跳过 |
| /private/tmp/wt-443-provenance | 票 #443 已关、干净，但分支领先 main 1（条件三不满足） |
| /private/tmp/wt-451-guard-role | 同上，领先 1 |
| /private/tmp/wt-452-twstepctx | 同上，领先 1 |
| /private/tmp/wt-69 | 票 #69 已关、干净，但 ticket-69 领先 main 94 |
| /private/tmp/nc-review-419 | detached，HEAD 领先 main 89（虽被 kimi-nest-mainline-20260717/ticket-69 包含，判据三从严按字面不满足） |
| /private/tmp/nc-base-359 | detached，HEAD 领先 main 57（同上） |
| /private/tmp/nc-verify-361 | detached 且脏 2 |
| /private/tmp/wt-441 | detached 且脏 4 |
| /private/tmp/wt-412-pi-bsp-timing | #412 OPEN（gh 复核） |
| /private/tmp/wt-466d0 | #466 OPEN |
| /private/tmp/wt-473f | #473 OPEN |
| /private/tmp/wt-483、wt-483-probe-base、wt-483-verify486 | #483 OPEN |
| /private/tmp/wt-487 | #487 OPEN（台账外新增，脏 228，在制品） |
| /private/tmp/kimi-nest-mainline | 白名单#1（#421 真实工作线） |
| 台账标「须留」的其余脏 worktree（agent-a1857ac32ba591225 脏8、agent-a246c3b83f9580658 脏4、agent-a7cbcf5d4f39d2ab3 脏8、agent-a6cdff06703f0de30 脏4、agent-aa3821c25e4bcffb9 脏2、agent-ad0fb1a36b79a236c 脏34、agent-aa324e3ed9e9fd19f 脏3、agent-ac95bfeffbc771eb7 脏2、agent-a9f43a8c08a3c1ccc 脏1、agent-ad1d992cede5cd786 脏4、agent-adc7ffe1de7988faf 脏4、agent-ae127f2abfab73c0e 脏1、agent-aebd1e8f008f2ff3e 脏1、wf_2f91805d-f6f-1 脏2、wf_5d177a52-0fb-1 脏1、wf_844a04d3-ddb-1 脏1、claude/* 9 个全脏、fix-S1-noleg 脏1、route-bsp-complete 脏1、top-all-bsp-short 脏5、fengliang-blindtest 脏4、wt-bisect 脏1） | 有在制品，判据②一律跳过 |
| 无票号非 agent-*/wf_* 的干净 worktree：NewChanlun-wt-unn、Downloads/NewChanlun-complete-classification-origin-20260626、Downloads/NewChanlun-strict-formal-20260626、fix-TC-churn、orbit9-A-h0skeleton、recursive-anchor | 配套①仅覆盖 agent-*/wf_* 命名；无票号不满足即清三条件，从严跳过 |

### 分支跳过

| 分支 | 跳过原因 |
|---|---|
| codex/292-trigger-source、ticket-486、issue-434-level-origin-removal、exp-155-wf7-l1-gate、facea-impl-113-20260623、t5-149-shortdiff | 对应 worktree 脏（在制品），连带跳过 |
| docs-443-stat-provenance、issue-451-guard-role、issue-452-twstepctx-visibility、ticket-69 | 票已关但领先 main（1/1/1/94），条件三不满足 |
| 台账标「须留」12 分支（_main_check_412、bull-bear-flip-37-20260623、c57/c58/c59-*、case2-t2sell-shortleg-37、issue-412-pi-bsp-timing-args、kimi-nest-mainline-20260717、ticket-466-d0、ticket-473f、ticket-483、unbind-confirm-src-38） | 票 OPEN 或白名单 |
| ticket-487 | #487 OPEN（台账外新增） |
| 其余无票号「待定」分支（含未合入=0 者） | 裁定即清口径要求票已关；无票号分支不在本批裁定范围，从严跳过 |

## 执行结果汇总

- **批二 worktree**：12 项中 **11 删成**，1 跳过（agent-ae517ccfbf0a8080a 终检突现 7353 脏文件，判据②）。
- **批三分支**：3/3 删成（impl/397-stop-in、impl/400-has-residual、issue-181-selldecision-cleanup）。
- **批四深度分叉**：4 分支全部完成 graveyard 打包（各 1.9G，另存 1 个脏文件）；删除全部被 git 拒绝（3×not fully merged，1×脏 worktree 占用），按铁律不 force、全部跳过并记录。
- **跳过主因**：在制品/脏文件（约 40 项）、票 OPEN（10 项）、分支领先 main>0（7 项）、白名单（2 项）、裁定范围外（无票号非 agent/wf 命名，6 项）。
- **终态核验**：worktree 73→62（含第一批 -1），分支 137（census 141 + 新增 ticket-487 − 第一批 2 − 本批 3）；已删 3 分支 rev-parse 确认 gone；主仓 main/HEAD 未动，全程零提交、零 --force/-D。
- **遗留待裁定**：批四 4 分支已打包但未删（需 -D 授权或先合入）；wt-443/451/452 票已关但各领先 main 1 commit；nc-review-419/nc-base-359 detached HEAD 内容已被 kimi-nest-mainline-20260717 包含，如需清可另行裁定。
