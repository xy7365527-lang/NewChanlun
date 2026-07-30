# worktree 与本地分支台账（census）——#507 wayfinder 供数

- 生成时间：2026-07-28（数据快照自 `git worktree list` / `git branch` / `gh issue view`，repo xy7365527-lang/NewChanlun）
- 规模：worktree **75** 个；本地分支 **141** 个
- 口径：脏文件数=`git status --short` 行数；领先 main=`git rev-list --count main..HEAD/branch`
- 注意：真实工作线 `kimi-nest-mainline-20260717`（领先 main 103 commits），挂在其上的 worktree「领先 main」计数含整条主线 delta，不等于该 worktree 独有未合入工作。
- 快照漂移注记：盘点期间仓库仍在活跃推进——main 在采集窗口内从 a3bd8ac008（02:55）前进到 2a42e7caff（03:28），ticket-483 从 a9e316a382（02:54）前进到 916a78f6c6（03:30）；worktree 表与分支表存在 ~35 分钟快照差。
- 处置三态口径：**可立即删**=票已关且无在制品且无未合入 commit；**须留**=有在制品或票仍开；**待定**=其余（含无票号、票已关但仍有未合入 commit）。

## 表一：worktree 台账（75 个）

| # | 路径 | 分支 | HEAD | 最后活动 | 票号 | 票状态(gh) | 脏文件 | 领先main | 处置建议 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `/Users/silencehan/Projects/NewChanlun` | main | a3bd8ac008 | 2026-07-28T02:55 | 无票号 | 无票号 | 128 | 0 | 主仓（不适用） |
| 2 | `/private/tmp/kimi-nest-mainline` | kimi-nest-mainline-20260717 | 6e15ceffee | 2026-07-28T03:16 | 无票号 | 无票号 | 16 | 103 | 须留 **白名单#1：#421 在制品** |
| 3 | `/private/tmp/wt-412-pi-bsp-timing` | issue-412-pi-bsp-timing-args | cba2191f97 | 2026-07-27T13:06 | #412 | #412 OPEN | 0 | 1 | 须留 |
| 4 | `/private/tmp/codex-work-292-trigger` | codex/292-trigger-source | 102dcd88bc | 2026-07-26T14:26 | #292 | #292 CLOSED | 1 | 0 | 须留 |
| 5 | `/private/tmp/nc-verify-361` | (detached) | d6f7e26430 | 2026-07-27T05:05 | #361 | #361 CLOSED | 2 | 60 | 须留 |
| 6 | `/private/tmp/wt-441` | (detached) | cb7dfc584b | 2026-07-27T11:56 | #441 | #441 CLOSED | 4 | 0 | 须留 |
| 7 | `/private/tmp/wt-421-post` | (detached) | 6e15ceffee | 2026-07-28T03:16 | #421 | #421 OPEN | 0 | 103 | 须留 |
| 8 | `/private/tmp/wt-443-provenance` | docs-443-stat-provenance | 8689393840 | 2026-07-27T12:48 | #443 | #443 CLOSED | 0 | 1 | 待定 |
| 9 | `/private/tmp/wt-451-guard-role` | issue-451-guard-role | e4e0a2272c | 2026-07-27T13:31 | #451 | #451 CLOSED | 0 | 1 | 待定 |
| 10 | `/private/tmp/wt-483` | ticket-483 | a9e316a382 | 2026-07-28T02:54 | #483 | #483 OPEN | 1 | 1 | 须留 |
| 11 | `/private/tmp/wt-473f` | ticket-473f | a8156d67ed | 2026-07-27T14:32 | #473 | #473 OPEN | 1 | 1 | 须留 |
| 12 | `/private/tmp/wt-466d0` | ticket-466-d0 | 37d83c0302 | 2026-07-28T02:54 | #466 | #466 OPEN | 1 | 1 | 须留 |
| 13 | `/private/tmp/wt-452-twstepctx` | issue-452-twstepctx-visibility | 025b3a44f0 | 2026-07-27T12:54 | #452 | #452 CLOSED | 0 | 1 | 待定 |
| 14 | `/private/tmp/wt-486` | ticket-486 | 3036c6a616 | 2026-07-28T02:47 | #486 | #486 CLOSED | 1 | 0 | 须留 |
| 15 | `/private/tmp/wt-483-verify486` | (detached) | 64535c710d | 2026-07-28T02:54 | #483 | #483 OPEN | 1 | 1 | 须留 |
| 16 | `/private/tmp/wt-434-level-origin` | issue-434-level-origin-removal | c00f1a0efa | 2026-07-27T12:24 | #434 | #434 CLOSED | 17 | 0 | 须留 |
| 17 | `/private/tmp/wt-69` | ticket-69 | b80ebb3d8a | 2026-07-28T02:25 | #69 | #69 CLOSED | 0 | 94 | 待定 |
| 18 | `/private/tmp/wt-bisect` | (detached) | 29d0faa4b8 | 2026-07-27T07:27 | 无票号 | 无票号 | 1 | 0 | 须留 |
| 19 | `/private/tmp/nc-review-419` | (detached) | 4d0c3101c2 | 2026-07-27T14:43 | #419 | #419 CLOSED | 0 | 89 | 待定 |
| 20 | `/private/tmp/nc-base-359` | (detached) | 500b3ac4bb | 2026-07-27T04:15 | #359 | #359 CLOSED | 0 | 57 | 待定 |
| 21 | `/private/tmp/wt-483-probe-base` | (detached) | cbd0e27bb1 | 2026-07-28T02:27 | #483 | #483 OPEN | 1 | 0 | 须留 |
| 22 | `/private/tmp/wt-421-pre` | (detached) | a12a1022d9 | 2026-07-28T02:38 | #421 | #421 OPEN | 0 | 102 | 须留 |
| 23 | `/Users/silencehan/Projects/NewChanlun-wt-unn` | feat/unn-necessity-corollaries | 44c49d4d54 | 2026-06-15T00:25 | 无票号 | 无票号 | 0 | 1 | 待定 |
| 24 | `/Users/silencehan/Downloads/NewChanlun-complete-classification-origin-20260626` | codex/complete-classification-origin-20260626 | 8b83454ba9 | 2026-06-28T09:07 | 无票号 | 无票号 | 0 | 216 | 待定 |
| 25 | `/Users/silencehan/Projects/fengliang-blindtest` | (detached) | 0e6ae1d300 | 2026-03-17T09:21 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 26 | `/Users/silencehan/Downloads/NewChanlun-strict-formal-20260626` | codex/strict-formal-20260626 | 1b63379b0f | 2026-06-26T01:30 | 无票号 | 无票号 | 0 | 1 | 待定 |
| 27 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a1857ac32ba591225` | worktree-agent-a1857ac32ba591225 | de8c6e7915 | 2026-06-22T01:43 | 无票号 | 无票号 | 8 | 0 | 须留 |
| 28 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a3eac13cd25744e5d` | exp-155-wf7-l1-gate | eb5cc9b4a4 | 2026-07-22T05:11 | #155 | #155 CLOSED | 2 | 0 | 须留 |
| 29 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a3c1604fc473623ee` | worktree-agent-a3c1604fc473623ee | 0f669195f8 | 2026-07-27T08:07 | 无票号 | 无票号 | 0 | 0 | 待定 |
| 30 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a246c3b83f9580658` | worktree-agent-a246c3b83f9580658 | a8d43d6fab | 2026-06-23T04:49 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 31 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a0389ec5753435684` | worktree-agent-a0389ec5753435684 | fe50ea040b | 2026-06-25T04:56 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 32 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a19bbaf9995460c33` | worktree-agent-a19bbaf9995460c33 | 5fe2704ee8 | 2026-06-25T05:01 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 33 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a7e569944b1d9a46d` | worktree-agent-a7e569944b1d9a46d | 7b7efc4a4d | 2026-07-27T08:25 | 无票号 | 无票号 | 0 | 0 | 待定 |
| 34 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a8bbb19daba5402bb` | issue-181-selldecision-cleanup | 05d56f2d2b | 2026-07-23T02:22 | #181 | #181 CLOSED | 0 | 0 | 可立即删 |
| 35 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a7cbcf5d4f39d2ab3` | worktree-agent-a7cbcf5d4f39d2ab3 | de8c6e7915 | 2026-06-22T01:43 | 无票号 | 无票号 | 8 | 0 | 须留 |
| 36 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a6cdff06703f0de30` | worktree-agent-a6cdff06703f0de30 | 30a6b55372 | 2026-06-23T03:02 | 无票号 | 无票号 | 4 | 1 | 须留 |
| 37 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aa3821c25e4bcffb9` | worktree-agent-aa3821c25e4bcffb9 | 5590f449ca | 2026-06-22T00:56 | 无票号 | 无票号 | 2 | 0 | 须留 |
| 38 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a487440bf7bbca944` | worktree-agent-a487440bf7bbca944 | 0a9be07d81 | 2026-06-25T05:03 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 39 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ad0fb1a36b79a236c` | (detached) | df40d2ef6f | 2026-07-22T09:53 | 无票号 | 无票号 | 34 | 0 | 须留 |
| 40 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ab4db18d22db8694c` | worktree-agent-ab4db18d22db8694c | 59634ed9b8 | 2026-06-25T05:01 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 41 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aa324e3ed9e9fd19f` | worktree-agent-aa324e3ed9e9fd19f | 2f918d7bf3 | 2026-06-25T05:25 | 无票号 | 无票号 | 3 | 22 | 须留 |
| 42 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ac95bfeffbc771eb7` | worktree-agent-ac95bfeffbc771eb7 | 2f918d7bf3 | 2026-06-25T05:25 | 无票号 | 无票号 | 2 | 22 | 须留 |
| 43 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a9f43a8c08a3c1ccc` | worktree-agent-a9f43a8c08a3c1ccc | 6d1b0602dd | 2026-06-25T04:55 | 无票号 | 无票号 | 1 | 7 | 须留 |
| 44 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aeac8e36d80879e2b` | worktree-agent-aeac8e36d80879e2b | fd58eb2dd6 | 2026-07-27T08:25 | 无票号 | 无票号 | 0 | 0 | 待定 |
| 45 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ad1d992cede5cd786` | worktree-agent-ad1d992cede5cd786 | 646c92bd0e | 2026-06-22T01:16 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 46 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-adc7ffe1de7988faf` | worktree-agent-adc7ffe1de7988faf | f38cd25f4c | 2026-06-22T00:23 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 47 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ace0022ed478bed16` | worktree-agent-ace0022ed478bed16 | 800370192e | 2026-06-25T05:19 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 48 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ad21219d4b564138e` | worktree-agent-ad21219d4b564138e | 8d6e5404d4 | 2026-06-25T04:52 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 49 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-af3878feb35eb9a1a` | t5-149-shortdiff | fcd18cb389 | 2026-07-22T09:21 | #149 | #149 CLOSED | 1 | 0 | 须留 |
| 50 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aebd1e8f008f2ff3e` | worktree-agent-aebd1e8f008f2ff3e | 6d1b0602dd | 2026-06-25T04:55 | 无票号 | 无票号 | 1 | 7 | 须留 |
| 51 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ae517ccfbf0a8080a` | worktree-agent-ae517ccfbf0a8080a | 3e5eaceb0a | 2026-06-25T05:06 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 52 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aeb1c6afb338546fd` | worktree-agent-aeb1c6afb338546fd | 6c45e68704 | 2026-06-25T05:24 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 53 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-ae127f2abfab73c0e` | worktree-agent-ae127f2abfab73c0e | 6d1b0602dd | 2026-06-25T04:55 | 无票号 | 无票号 | 1 | 7 | 须留 |
| 54 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/dazzling-kowalevski-52a297` | claude/dazzling-kowalevski-52a297 | d53be572ff | 2026-04-24T04:54 | 无票号 | 无票号 | 107 | 4 | 须留 |
| 55 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/elastic-noyce-f82f92` | claude/elastic-noyce-f82f92 | 9c4bd58eae | 2026-04-25T03:27 | 无票号 | 无票号 | 26 | 0 | 须留 |
| 56 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/infallible-rhodes-a27a56` | claude/infallible-rhodes-a27a56 | a7aa3db5ca | 2026-04-20T07:08 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 57 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/fix-TC-churn` | fix-TC-churn-r4 | 86f4c14269 | 2026-06-24T06:50 | 无票号 | 无票号 | 0 | 0 | 待定 |
| 58 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-af41a2af699aa4763` | worktree-agent-af41a2af699aa4763 | 760ad76fd8 | 2026-06-25T04:45 | 无票号 | 无票号 | 0 | 4 | 待定 |
| 59 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/facea-impl-113` | facea-impl-113-20260623 | 54279a503e | 2026-06-24T06:56 | #113 | #113 CLOSED | 2 | 0 | 须留 |
| 60 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/fix-S1-noleg` | swarm/fix-S1-noleg | e86ece26bb | 2026-06-24T04:22 | 无票号 | 无票号 | 1 | 1 | 须留 |
| 61 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/distracted-herschel-75e391` | claude/distracted-herschel-75e391 | a7aa3db5ca | 2026-04-20T07:08 | 无票号 | 无票号 | 12 | 0 | 须留 |
| 62 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/keen-cori-3a5b2f` | claude/keen-cori-3a5b2f | 830efce412 | 2026-04-05T19:51 | 无票号 | 无票号 | 15 | 0 | 须留 |
| 63 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/bold-tesla` | claude/bold-tesla | 7c78d717c1 | 2026-04-05T04:19 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 64 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-aec7bff16fc502aff` | worktree-agent-aec7bff16fc502aff | 19b4015927 | 2026-04-20T07:08 | 无票号 | 无票号 | 1 | 1171 | 须留 |
| 65 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/route-bsp-complete` | route-bsp-complete-20260623 | b88d8201e6 | 2026-06-23T05:50 | 无票号 | 无票号 | 1 | 4 | 须留 |
| 66 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/orbit9-A-h0skeleton` | orbit9-A-h0skeleton | 0190470722 | 2026-06-24T09:13 | 无票号 | 无票号 | 0 | 1 | 待定 |
| 67 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/wf_5d177a52-0fb-1` | a-route-cseg-drill-fix | 5c5cebdbb5 | 2026-06-22T17:01 | #1 | #1 未知 | 1 | 2 | 须留 |
| 68 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/wf_2f91805d-f6f-1` | worktree-wf_2f91805d-f6f-1 | 3cc8c49b7a | 2026-06-22T07:56 | #1 | #1 未知 | 2 | 1 | 须留 |
| 69 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/priceless-boyd-b3b230` | claude/priceless-boyd-b3b230 | 6b2535fa18 | 2026-04-21T09:02 | 无票号 | 无票号 | 11 | 1 | 须留 |
| 70 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/peaceful-cerf-ac6d83` | claude/peaceful-cerf-ac6d83 | ed3822e7ed | 2026-04-24T07:10 | 无票号 | 无票号 | 3 | 0 | 须留 |
| 71 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/quirky-matsumoto-c22fc2` | claude/quirky-matsumoto-c22fc2 | 051c8b253d | 2026-04-22T00:40 | 无票号 | 无票号 | 3 | 1 | 须留 |
| 72 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/relaxed-lovelace-03bad0` | claude/relaxed-lovelace-03bad0 | a7aa3db5ca | 2026-04-20T07:08 | 无票号 | 无票号 | 4 | 0 | 须留 |
| 73 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/recursive-anchor` | (detached) | 4fcfd2840d | 2026-06-22T05:19 | 无票号 | 无票号 | 0 | 6 | 待定 |
| 74 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/top-all-bsp-short` | anchor-top-gate-combo | f45bd35b23 | 2026-06-23T00:25 | 无票号 | 无票号 | 5 | 12 | 须留 |
| 75 | `/Users/silencehan/Projects/NewChanlun/.claude/worktrees/wf_844a04d3-ddb-1` | worktree-wf_844a04d3-ddb-1 | 237394bba9 | 2026-06-22T18:24 | #1 | #1 未知 | 1 | 2 | 须留 |

### worktree HEAD subject 明细

| # | HEAD | subject |
|---|---|---|
| 1 | a3bd8ac008 | merge: #486 anchor 门结构方向授权——L≥1 三类点按几何事实产出 |
| 2 | 6e15ceffee | feat(theta): 接通活假设账本生产侧车 (#421) |
| 3 | cba2191f97 | fix(rust): #412 pi_bsp_timing 参数解析静默失效——窗口切片改按显式字段判定 |
| 4 | 102dcd88bc | fix(center_oscillation_trade): #292 续修——重基后挂起悬空泄漏 |
| 5 | d6f7e26430 | fix(rust): #363 M4 逐级稀疏性判据同步——cap_narrowed_levels 逐级解释项（#355 MED-C） |
| 6 | cb7dfc584b | feat(witness): #442 三计数加 level 维探针——延续/清算/来源按级别分桶（纯观测） |
| 7 | 6e15ceffee | feat(theta): 接通活假设账本生产侧车 (#421) |
| 8 | 8689393840 | docs(agents): #443 统计结论口径标注与引用审查规范——轻/重两档+默认降级+五样本试判表 |
| 9 | e4e0a2272c | fix(guard): #451 nest_isolation_guard 从文件名白名单改按角色豁免 |
| 10 | a9e316a382 | fix(classifier): #483 补 C 不破核心盘背观测支 |
| 11 | a8156d67ed | chore(theta): add #473 third-class funnel probe |
| 12 | 37d83c0302 | feat(theta): add #466 D0 rebase observability |
| 13 | 025b3a44f0 | fix(coverage): #452 摘除 TwStepCtx 误标 cfg(test)——生产路径 fill.rs 消费点解阻 |
| 14 | 3036c6a616 | test(classifier): 修复 #486 q7 覆盖退化 |
| 15 | 64535c710d | fix(classifier): #483 补 C 不破核心盘背观测支 |
| 16 | c00f1a0efa | docs(rust): #438 CI 注释补 #452 issue 号——TwStepCtx 阻断已落点 |
| 17 | b80ebb3d8a | docs(rules): #69 裁定①——不可变条款补适用范围注记（replay 驱动性能缓存不适用，锚 R2/R3 + #494 翻转条件） |
| 18 | 29d0faa4b8 | feat(shortdiff): #381 空头 campaign 支持——键形状 (level,side) + 多空镜像口径 |
| 19 | 4d0c3101c2 | feat(rust): #419 OKLO 真实窗接入 treasury 三阶段层——M8_SYMBOL/M8_REPORT_PATH + FeeAudit 逐科目审计 |
| 20 | 500b3ac4bb | docs(chanlun): 影子评审报告入库——#350（350b）/#351（355）两轴评审 |
| 21 | cbd0e27bb1 | docs(adr): #486 基线重订（用户裁定）——execR=+4626831/R=+5154033/MaxDD=0.1085，立「反证+重订」常例 |
| 22 | a12a1022d9 | feat(rust): #409 pan 活窗探针 bin 入库（报告 32500e78aa 的可复现工具；cargo check 绿，簿记补交） |
| 23 | 44c49d4d54 | feat: unn 严格实装 4 条近似推论 T1/T5/T14/A5(根空头 MtM 翻转,prove 全 PASS) |
| 24 | 8b83454ba9 | Add Round137 exact relation classifier bridge |
| 25 | 0e6ae1d300 | fix: v263-swarm——479号broken_deps redirect(478→480) + topo_effects确认 |
| 26 | 1b63379b0f | feat(theta_v0): add strict formal engine package |
| 27 | de8c6e7915 | feat(recursive_t): 1s prove 守卫尺度不变性 L3 + L4 突破——多标的×多窗口 |
| 28 | eb5cc9b4a4 | #145 T1：typed 出场裁决前移到组合层决策点（StepTrace.closed 三元化） |
| 29 | 0f669195f8 | fix(interp): #396 影子评审两条——单射前置条件显式化 + 补规则体 golden 锁 |
| 30 | a8d43d6fab | docs(route-bsp): #36核心L3——emergent-long已honor(清现金非flip)+add=regime税+全8<BH |
| 31 | fe50ea040b | fix(ceremony): 补全 step 5b 按需工位消费端 + 三类工位区分 |
| 32 | 5fe2704ee8 | review(self-review): 9塔构造异步自指审查 — L0主线一致 CONDITIONAL PASS + HIGH拍扁残余(completion_source第七轴声明-定义不一致) |
| 33 | 7b7efc4a4d | fix(coverage): #398 消除拆分引入的 2 条 unused import 告警——回到 main 告警基线 |
| 34 | 05d56f2d2b | #152 终验A：ADR 0001 对齐审计报告（教义锚） |
| 35 | de8c6e7915 | feat(recursive_t): 1s prove 守卫尺度不变性 L3 + L4 突破——多标的×多窗口 |
| 36 | 30a6b55372 | feat(recursive_t): route_bsp CC orbit 升格（561号 N9 极性协变，任务29） |
| 37 | 5590f449ca | docs(recursive_t): 8标的×3模式 L3 验证——prove守卫 L2→L3 + 现状基线 |
| 38 | 0a9be07d81 | audit: codex 真异质审计 9 塔整体一致性(FAIL) + payoff #97 重测(部分成立) |
| 39 | df40d2ef6f | #146 T2 合并后修复：补齐 VoiceState/VoiceStepInput 新增字段（#149/#150 语义合流） |
| 40 | 59634ed9b8 | feat(task#5): payoff G轴 = 级别-方向对齐门 + 强平可达性（539 支配失血分量修复） |
| 41 | 2f918d7bf3 | docs(549): relations.jsonl 来源诚实标注 — .bak重建非正典,守549精神不丢1034边,揭示549无remote有效域漏洞 |
| 42 | 2f918d7bf3 | docs(549): relations.jsonl 来源诚实标注 — .bak重建非正典,守549精神不丢1034边,揭示549无remote有效域漏洞 |
| 43 | 6d1b0602dd | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 44 | fd58eb2dd6 | fix(rust): #394 评审订正——真实二进制 bit-exact witness + 见证测试去重言式 |
| 45 | 646c92bd0e | feat(recursive_t): 1s prove 守卫尺度不变性深度验证——滤波器比喻尺度维度实证 |
| 46 | f38cd25f4c | feat(recursive_t): 移植旧引擎 prove 守卫族（6守卫·rec+flat对称·bit-exact） |
| 47 | 800370192e | fix(compact): 捕获真中断点写入 session ## 中断点 章节（破除循环引导） |
| 48 | 8d6e5404d4 | docs(dispatch-dag): 对齐 event_skill_map 声明层到 562号扬弃（结构=常设teammate+守卫层） |
| 49 | fcd18cb389 | #149 T5：短差显式机制端到端 |
| 50 | 6d1b0602dd | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 51 | 3e5eaceb0a | docs(genealogy): 起草 597-601 五条谱系记录(生成态,塔形式化压缩谱系态) |
| 52 | 6c45e68704 | feat(btc-l3): BTC 1秒 L3 决定性判据 — 第四轴坐实 + 配额维瓶颈否证(瓶颈在信号层539) |
| 53 | 6d1b0602dd | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 54 | d53be572ff | feat: v71-swarm round2 同步——17条pending质询完成 + 490a /escalate 新增 + 四分法分类 + commit ff8046 补增 |
| 55 | 9c4bd58eae | chore: 研究材料 + 文件索引 + v69 sessions + 依赖更新 |
| 56 | a7aa3db5ca | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 57 | 86f4c14269 | diag(recursive_t): #164 R2 T3出场leak可约性诊断 — 修复L3否证(出场侧不可约) |
| 58 | 760ad76fd8 | feat(tower-sub5): 逐级内在配额 mobile_frac(L_pullback,L_confirm) + OFF bit-exact |
| 59 | 54279a503e | merge(facea): R1-R4 失败模式修复链集成 — R2 T3出场诊断(observation-only) cherry-pick + R1/R3/R4/shortleg |
| 60 | e86ece26bb | feat(recursive_t): 584定仓A/B并入R3(#6) — 核心腿涌现升级定仓来源对照 |
| 61 | a7aa3db5ca | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 62 | 830efce412 | fix: 走势引擎趋势判断修复(ZD/ZG替代DD/GG) + 美元信用溢价Δω + 影子金价覆盖率 |
| 63 | 7c78d717c1 | fix: 包含处理方向偏置修复(dir_state=None时用K线方向推断) + Codex审查记录 |
| 64 | 19b4015927 | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 65 | b88d8201e6 | feat(recursive_t): 全仓保证金 enable_cross_margin + L3(task #38) |
| 66 | 0190470722 | feat(orbit9-A): H⁰方向骨架 flip confirmed门 + τ手性对称化 (task#39) |
| 67 | 5c5cebdbb5 | fix(recursive_t): A路c段钻取重新实装——修正nest_chain_complete检测对象错误(异步.last()→构造性窗口过滤) |
| 68 | 3cc8c49b7a | feat(recursive_t): A 路本体实装——对称双吃最高级别精确切换(D_TOP区间套级联+anchor条件门控+平空对称) |
| 69 | 6b2535fa18 | fix: 结算483号元观察——从pending迁移到settled（worktree状态同步） |
| 70 | ed3822e7ed | feat: 485号谱系结算(修正)——jet bundle术语降级为多层约束系统 |
| 71 | 051c8b253d | chore: 清理 483号生成态残留（pending 副本，settled 已存在） |
| 72 | a7aa3db5ca | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 73 | 4fcfd2840d | feat(recursive_t): 递归 anchor 深度门（变体 2，REC_ANCHOR_DEPTH） |
| 74 | f45bd35b23 | docs(genealogy): combo 否证有效域边界=独立腿架构,不关闭命题4 a0嵌套构成L3-未决(recursive-bsp D1/D2) |
| 75 | 237394bba9 | test(recursive_t): 读法B 8标的×3模式 L3 全量验证 harness + 持仓时长by方向观测 |

## 表二：本地分支台账（141 个）

| # | 分支 | HEAD | 最后活动 | 领先main | 票号 | 票状态(gh) | 已挂载worktree | 处置建议 |
|---|---|---|---|---|---|---|---|---|
| 1 | _main_check_412 | 9de5c7c84a | 2026-07-27 12:59 | 0 | #412 | #412 OPEN | 否 | 须留 |
| 2 | a-route-cseg-drill-fix | 5c5cebdbb5 | 2026-06-22 17:01 | 2 | 无票号 | 无票号 | 是 | 待定 |
| 3 | alert-autofix--25 | 2e0e5debc5 | 2026-03-05 15:47 | 0 | #25 | #25 未知 | 否 | 待定 |
| 4 | alert-autofix-25 | 95cca147c5 | 2026-03-05 15:40 | 0 | #25 | #25 未知 | 否 | 待定 |
| 5 | alert-autofix-26 | 29e80eb6e8 | 2026-03-05 15:48 | 1 | #26 | #26 未知 | 否 | 待定 |
| 6 | anchor-top-gate-combo | f45bd35b23 | 2026-06-23 00:25 | 12 | 无票号 | 无票号 | 是 | 待定 |
| 7 | bughunt-fix-20260710 | aa9bac3ee2 | 2026-07-10 21:10 | 15 | 无票号 | 无票号 | 否 | 待定 |
| 8 | bughunt-fix-v2 | 1dcdb95a09 | 2026-07-10 21:49 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 9 | bull-bear-flip-37-20260623 | e4a2dca6e5 | 2026-06-23 05:42 | 1 | #37 | #37 OPEN | 否 | 须留 |
| 10 | c57-material-20260712 | 03a726ab0d | 2026-07-12 21:10 | 1 | #57 | #57 OPEN | 否 | 须留 |
| 11 | c58-assembler-20260712 | d676feafb2 | 2026-07-12 21:22 | 1 | #58 | #58 OPEN | 否 | 须留 |
| 12 | c59-entry-audit-20260712 | b3aceb19b6 | 2026-07-01 16:50 | 0 | #59 | #59 OPEN | 否 | 须留 |
| 13 | capture-ratio-verify-148 | 77f8adf69c | 2026-06-24 01:54 | 0 | #148 | #148 CLOSED | 否 | 可立即删 |
| 14 | case2-t2sell-shortleg-37 | 35a7bb83d7 | 2026-06-24 09:02 | 2 | #37 | #37 OPEN | 否 | 须留 |
| 15 | claude-sci/gap3-trigger | 01a2f60451 | 2026-07-01 12:40 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 16 | claude/add-ceremony-command-A5vJl | c6c20bb701 | 2026-02-19 15:17 | 5 | 无票号 | 无票号 | 否 | 待定 |
| 17 | claude/bold-tesla | 7c78d717c1 | 2026-04-05 04:19 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 18 | claude/cc-session-briefing-setup-TXHg8 | 8866b7bb97 | 2026-02-18 15:30 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 19 | claude/ceremony-command-8NRQ8 | 4e43171f5b | 2026-02-23 07:54 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 20 | claude/ceremony-feature-CInoK | f0670c0223 | 2026-02-17 13:21 | 48 | 无票号 | 无票号 | 否 | 待定 |
| 21 | claude/check-network-permissions-86zEe | 240c558c8e | 2026-02-20 15:02 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 22 | claude/dazzling-kowalevski-52a297 | d53be572ff | 2026-04-24 04:54 | 4 | 无票号 | 无票号 | 是 | 待定 |
| 23 | claude/distracted-herschel-75e391 | a7aa3db5ca | 2026-04-20 07:08 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 24 | claude/elastic-noyce-f82f92 | 9c4bd58eae | 2026-04-25 03:27 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 25 | claude/enable-agent-team-mode-2rNfS | c4bb3a2a9a | 2026-02-21 19:51 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 26 | claude/infallible-rhodes-a27a56 | a7aa3db5ca | 2026-04-20 07:08 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 27 | claude/keen-cori-3a5b2f | 830efce412 | 2026-04-05 19:51 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 28 | claude/merge-to-main-TXHg8 | 93d1b22956 | 2026-02-18 09:38 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 29 | claude/peaceful-cerf-ac6d83 | ed3822e7ed | 2026-04-24 07:10 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 30 | claude/persist-agent-team-MxfhB | 31ccc3966b | 2026-02-23 14:39 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 31 | claude/priceless-boyd-b3b230 | 6b2535fa18 | 2026-04-21 09:02 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 32 | claude/quirky-matsumoto-c22fc2 | 051c8b253d | 2026-04-22 00:40 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 33 | claude/relaxed-lovelace-03bad0 | a7aa3db5ca | 2026-04-20 07:08 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 34 | claude/review-plagiarism-check-UhDT5 | 01744c32a3 | 2026-04-12 03:43 | 3 | 无票号 | 无票号 | 否 | 待定 |
| 35 | codex-line-20260625 | 2f918d7bf3 | 2026-06-25 05:25 | 22 | 无票号 | 无票号 | 否 | 待定 |
| 36 | codex/292-trigger-source | 102dcd88bc | 2026-07-26 14:26 | 0 | #292 | #292 CLOSED | 是 | 可立即删 |
| 37 | codex/complete-classification-origin-20260626 | 8b83454ba9 | 2026-06-28 09:07 | 216 | 无票号 | 无票号 | 是 | 待定 |
| 38 | codex/f240 | 9909cfa2d9 | 2026-07-25 02:04 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 39 | codex/f242 | e77ffd1955 | 2026-07-25 02:45 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 40 | codex/p7-nest-anchor | 616ff6eb97 | 2026-07-07 18:32 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 41 | codex/strict-formal-20260626 | 1b63379b0f | 2026-06-26 01:30 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 42 | codex/t6-gross-exposure | df40d2ef6f | 2026-07-22 09:53 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 43 | codex/typed-close-t1 | 6e0362c7fb | 2026-07-19 05:33 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 44 | cursor/critical-bug-inspection-abec | 51438d150b | 2026-04-26 21:28 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 45 | dependabot/npm_and_yarn/frontend/npm_and_yarn-85e16ba948 | d68bb8385a | 2026-03-29 16:50 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 46 | dependabot/npm_and_yarn/frontend/npm_and_yarn-d447b59e3c | c884c3daf9 | 2026-03-07 13:48 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 47 | dependabot/uv/uv-5abb2dfbbd | 5aa995d076 | 2026-03-17 20:09 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 48 | docs-443-stat-provenance | 8689393840 | 2026-07-27 12:48 | 1 | #443 | #443 CLOSED | 是 | 待定 |
| 49 | exp-155-wf7-l1-gate | eb5cc9b4a4 | 2026-07-22 05:11 | 0 | #155 | #155 CLOSED | 是 | 可立即删 |
| 50 | experiment/t14-t5-root-flip-necessity | a7b2357f23 | 2026-06-15 04:18 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 51 | facea-impl-113-20260623 | 54279a503e | 2026-06-24 06:56 | 0 | #113 | #113 CLOSED | 是 | 可立即删 |
| 52 | feat/unn-necessity-corollaries | 44c49d4d54 | 2026-06-15 00:25 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 53 | fix-T3-leak-20260624 | fc52bc4376 | 2026-06-24 03:56 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 54 | fix-TC-churn-r4 | 86f4c14269 | 2026-06-24 06:50 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 55 | gap3-rework-codex9-fix | 3ebc4e493c | 2026-07-10 17:47 | 2 | 无票号 | 无票号 | 否 | 待定 |
| 56 | gap3/trigger-inv2 | 01a2f60451 | 2026-07-01 12:40 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 57 | impl/397-stop-in | 15ad9d5ecc | 2026-07-27 07:33 | 0 | #397 | #397 CLOSED | 否 | 可立即删 |
| 58 | impl/400-has-residual | e3c837bc47 | 2026-07-27 07:57 | 0 | #400 | #400 CLOSED | 否 | 可立即删 |
| 59 | issue-181-selldecision-cleanup | 05d56f2d2b | 2026-07-23 02:22 | 0 | #181 | #181 CLOSED | 是 | 可立即删 |
| 60 | issue-412-pi-bsp-timing-args | cba2191f97 | 2026-07-27 13:06 | 1 | #412 | #412 OPEN | 是 | 须留 |
| 61 | issue-434-level-origin-removal | c00f1a0efa | 2026-07-27 12:24 | 0 | #434 | #434 CLOSED | 是 | 可立即删 |
| 62 | issue-451-guard-role | e4e0a2272c | 2026-07-27 13:31 | 1 | #451 | #451 CLOSED | 是 | 待定 |
| 63 | issue-452-twstepctx-visibility | 025b3a44f0 | 2026-07-27 12:54 | 1 | #452 | #452 CLOSED | 是 | 待定 |
| 64 | kimi-nest-mainline-20260717 | 6e15ceffee | 2026-07-28 03:16 | 103 | 无票号 | 无票号 | 是 | 须留（白名单：#421 真实工作线） |
| 65 | main | 2a42e7caff | 2026-07-28 03:28 | 0 | 无票号 | 无票号 | 是 | 主分支（不适用） |
| 66 | merge-mainline-20260719 | fcd3214f18 | 2026-07-19 05:24 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 67 | orbit9-A-h0skeleton | 0190470722 | 2026-06-24 09:13 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 68 | orbit9-B-dispatch-20260624 | ed0b59c85b | 2026-06-25 02:12 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 69 | orbit9-D-integration-20260624 | 1700b1f3b2 | 2026-06-24 17:24 | 4 | 无票号 | 无票号 | 否 | 待定 |
| 70 | orbit9-hloc-20260624 | c3d1df543b | 2026-06-24 22:49 | 5 | 无票号 | 无票号 | 否 | 待定 |
| 71 | orbit9-next-a-20260624 | 7f0c8e3b36 | 2026-06-24 21:47 | 4 | 无票号 | 无票号 | 否 | 待定 |
| 72 | orbit9-nt-port-20260624 | 0e6191360a | 2026-06-24 23:01 | 4 | 无票号 | 无票号 | 否 | 待定 |
| 73 | pre-cleanup-main | ed3822e7ed | 2026-04-24 07:10 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 74 | prop4-bidir-consume-C-20260623 | fde2d3fdd4 | 2026-06-23 02:56 | 1 | 无票号 | 无票号 | 否 | 待定 |
| 75 | prop4-nest-readingB-20260623 | f754ea6a9f | 2026-06-24 09:13 | 14 | 无票号 | 无票号 | 否 | 待定 |
| 76 | proto-gatepass-20260722 | 640609071d | 2026-07-19 05:14 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 77 | rescue/ci-438-test-20260727-concurrent-20260727-1348 | 6efb7891f0 | 2026-07-27 13:48 | 3 | #438 | #438 CLOSED | 否 | 待定 |
| 78 | route-bsp-complete-20260623 | b88d8201e6 | 2026-06-23 05:50 | 4 | 无票号 | 无票号 | 是 | 待定 |
| 79 | shortleg-symmetric-entry-20260624 | 60ee1c33f1 | 2026-06-24 02:34 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 80 | strict-nesting-20260708 | 9d782cac61 | 2026-07-09 03:43 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 81 | swarm/fix-S1-noleg | e86ece26bb | 2026-06-24 04:22 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 82 | swarm/fix-T1-direction | ecbf7f5557 | 2026-06-24 04:12 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 83 | t5-149-shortdiff | fcd18cb389 | 2026-07-22 09:21 | 0 | #149 | #149 CLOSED | 是 | 可立即删 |
| 84 | t6-gross-exposure | 744da5ff81 | 2026-07-23 02:00 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 85 | task70-merge | d01d9b73ed | 2026-07-13 22:35 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 86 | ticket-466-d0 | 37d83c0302 | 2026-07-28 02:54 | 1 | #466 | #466 OPEN | 是 | 须留 |
| 87 | ticket-472 | df3e19c4f5 | 2026-07-27 14:47 | 0 | #472 | #472 CLOSED | 否 | 可立即删 |
| 88 | ticket-473f | a8156d67ed | 2026-07-27 14:32 | 1 | #473 | #473 OPEN | 是 | 须留 |
| 89 | ticket-483 | 916a78f6c6 | 2026-07-28 03:30 | 2 | #483 | #483 OPEN | 是 | 须留 |
| 90 | ticket-486 | 3036c6a616 | 2026-07-28 02:47 | 0 | #486 | #486 CLOSED | 是 | 可立即删 |
| 91 | ticket-69 | b80ebb3d8a | 2026-07-28 02:25 | 94 | #69 | #69 CLOSED | 是 | 待定 |
| 92 | top-all-bsp-short | fe60ea4865 | 2026-06-22 22:49 | 9 | 无票号 | 无票号 | 否 | 待定 |
| 93 | traycer | a202b3b479 | 2026-04-27 09:59 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 94 | unbind-confirm-src-38 | 7a571d1a38 | 2026-07-10 18:57 | 1 | #38 | #38 OPEN | 否 | 须留 |
| 95 | wip-orphans-20260625 | 63c2e56841 | 2026-06-25 04:03 | 15 | 无票号 | 无票号 | 否 | 待定 |
| 96 | worktree-agent-a0389ec5753435684 | fe50ea040b | 2026-06-25 04:56 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 97 | worktree-agent-a1857ac32ba591225 | de8c6e7915 | 2026-06-22 01:43 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 98 | worktree-agent-a19bbaf9995460c33 | 5fe2704ee8 | 2026-06-25 05:01 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 99 | worktree-agent-a1b324d1f1f9c6ff5 | f53b50f7ca | 2026-06-27 08:37 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 100 | worktree-agent-a1d20aa5fa82dc3e3 | 775e99f4c4 | 2026-07-01 11:58 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 101 | worktree-agent-a246c3b83f9580658 | a8d43d6fab | 2026-06-23 04:49 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 102 | worktree-agent-a3c1604fc473623ee | 0f669195f8 | 2026-07-27 08:07 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 103 | worktree-agent-a3eac13cd25744e5d | 19b4015927 | 2026-04-20 07:08 | 1171 | 无票号 | 无票号 | 否 | 待定 |
| 104 | worktree-agent-a487440bf7bbca944 | 0a9be07d81 | 2026-06-25 05:03 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 105 | worktree-agent-a55138cbf237e9057 | c5647c6384 | 2026-06-29 13:12 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 106 | worktree-agent-a5a6a34acc07e3fb7 | 7cb3c6460c | 2026-07-01 09:56 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 107 | worktree-agent-a65acf72e2dc14645 | a5bc6ff722 | 2026-06-27 12:47 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 108 | worktree-agent-a69a85bac6190be05 | 63c2e56841 | 2026-06-25 04:03 | 15 | 无票号 | 无票号 | 否 | 待定 |
| 109 | worktree-agent-a6c577e593f1cf675 | 47d4027de1 | 2026-06-23 03:29 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 110 | worktree-agent-a6cdff06703f0de30 | 30a6b55372 | 2026-06-23 03:02 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 111 | worktree-agent-a78ead6af0f6f80e2 | c5647c6384 | 2026-06-29 13:12 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 112 | worktree-agent-a7cbcf5d4f39d2ab3 | de8c6e7915 | 2026-06-22 01:43 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 113 | worktree-agent-a7e569944b1d9a46d | 7b7efc4a4d | 2026-07-27 08:25 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 114 | worktree-agent-a883448afa390e338 | 775e99f4c4 | 2026-07-01 11:58 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 115 | worktree-agent-a8bbb19daba5402bb | 19b4015927 | 2026-04-20 07:08 | 1171 | 无票号 | 无票号 | 否 | 待定 |
| 116 | worktree-agent-a9f43a8c08a3c1ccc | 6d1b0602dd | 2026-06-25 04:55 | 7 | 无票号 | 无票号 | 是 | 待定 |
| 117 | worktree-agent-aa1f62e1595e219aa | 775e99f4c4 | 2026-07-01 11:58 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 118 | worktree-agent-aa324e3ed9e9fd19f | 2f918d7bf3 | 2026-06-25 05:25 | 22 | 无票号 | 无票号 | 是 | 待定 |
| 119 | worktree-agent-aa3821c25e4bcffb9 | 5590f449ca | 2026-06-22 00:56 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 120 | worktree-agent-ab4db18d22db8694c | 59634ed9b8 | 2026-06-25 05:01 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 121 | worktree-agent-ac3a2924297104458 | c5647c6384 | 2026-06-29 13:12 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 122 | worktree-agent-ac95bfeffbc771eb7 | 2f918d7bf3 | 2026-06-25 05:25 | 22 | 无票号 | 无票号 | 是 | 待定 |
| 123 | worktree-agent-ace0022ed478bed16 | 800370192e | 2026-06-25 05:19 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 124 | worktree-agent-ad1d992cede5cd786 | 646c92bd0e | 2026-06-22 01:16 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 125 | worktree-agent-ad21219d4b564138e | 8d6e5404d4 | 2026-06-25 04:52 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 126 | worktree-agent-ad3eb3b39639b8306 | 260dfc4545 | 2026-06-22 05:45 | 7 | 无票号 | 无票号 | 否 | 待定 |
| 127 | worktree-agent-adaa2b4719750e157 | ddc22e310c | 2026-06-26 08:36 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 128 | worktree-agent-adc7ffe1de7988faf | f38cd25f4c | 2026-06-22 00:23 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 129 | worktree-agent-ae127f2abfab73c0e | 6d1b0602dd | 2026-06-25 04:55 | 7 | 无票号 | 无票号 | 是 | 待定 |
| 130 | worktree-agent-ae517ccfbf0a8080a | 3e5eaceb0a | 2026-06-25 05:06 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 131 | worktree-agent-ae691a4a585bbef12 | 775e99f4c4 | 2026-07-01 11:58 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 132 | worktree-agent-aeac8e36d80879e2b | fd58eb2dd6 | 2026-07-27 08:25 | 0 | 无票号 | 无票号 | 是 | 待定 |
| 133 | worktree-agent-aeb1c6afb338546fd | 6c45e68704 | 2026-06-25 05:24 | 6 | 无票号 | 无票号 | 是 | 待定 |
| 134 | worktree-agent-aebd1e8f008f2ff3e | 6d1b0602dd | 2026-06-25 04:55 | 7 | 无票号 | 无票号 | 是 | 待定 |
| 135 | worktree-agent-aec7bff16fc502aff | 19b4015927 | 2026-04-20 07:08 | 1171 | 无票号 | 无票号 | 是 | 待定 |
| 136 | worktree-agent-af3878feb35eb9a1a | 19b4015927 | 2026-04-20 07:08 | 1171 | 无票号 | 无票号 | 否 | 待定 |
| 137 | worktree-agent-af41a2af699aa4763 | 760ad76fd8 | 2026-06-25 04:45 | 4 | 无票号 | 无票号 | 是 | 待定 |
| 138 | worktree-level-dx | 8d45dc449b | 2026-07-01 16:09 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 139 | worktree-wf_2f91805d-f6f-1 | 3cc8c49b7a | 2026-06-22 07:56 | 1 | 无票号 | 无票号 | 是 | 待定 |
| 140 | worktree-wf_5d177a52-0fb-1 | 583b1cc72c | 2026-06-22 16:28 | 0 | 无票号 | 无票号 | 否 | 待定 |
| 141 | worktree-wf_844a04d3-ddb-1 | 237394bba9 | 2026-06-22 18:24 | 2 | 无票号 | 无票号 | 是 | 待定 |

### 分支 HEAD subject 明细

| # | 分支 | subject |
|---|---|---|
| 1 | _main_check_412 | fix(rust): #311 pure_bsp_timing E0277——&Rc<Vec<BspPoint>> 改 .iter() 借元素（#295 浮出，知会所属工作线） |
| 2 | a-route-cseg-drill-fix | fix(recursive_t): A路c段钻取重新实装——修正nest_chain_complete检测对象错误(异步.last()→构造性窗口过滤) |
| 3 | alert-autofix--25 | Potential fix for code scanning alert no. 25: Full server-side request forgery |
| 4 | alert-autofix-25 | Potential fix for code scanning alert no. 25: Full server-side request forgery |
| 5 | alert-autofix-26 | Potential fix for code scanning alert no. 26: Uncontrolled data used in path expression |
| 6 | anchor-top-gate-combo | docs(genealogy): combo 否证有效域边界=独立腿架构,不关闭命题4 a0嵌套构成L3-未决(recursive-bsp D1/D2) |
| 7 | bughunt-fix-20260710 | fix(ceremony_scan): cert F-15/F-16 低危收尾 |
| 8 | bughunt-fix-v2 | docs: 写入 bughunt-fix-v2 20260710 修复报告 |
| 9 | bull-bear-flip-37-20260623 | feat(bull-bear-flip): #37 牛熊切换三态(真顶flip short吃跌/强牛hold/真底关熊腿)+L3判决 |
| 10 | c57-material-20260712 | escalate(C裁决): #57 塔窗口单元 vs 完成走势类型语义偏差材料汇编（只呈材料不裁决）+ 谱系pending 670 |
| 11 | c58-assembler-20260712 | feat(theta): as-of 走势类型组装器纯视图 + 硬门测试 (#58) |
| 12 | c59-entry-audit-20260712 | feat(GAP3): 三阶段 EarningShares barrier-gated 可达 + κ canonical 政策旋钮 |
| 13 | capture-ratio-verify-148 | feat(recursive_t): capture-ratio v2 #154——矩阵完备性四检验前置 + 逐笔全称判定(禁求和) |
| 14 | case2-t2sell-shortleg-37 | docs(#37 reframe): 降为经验探针数据点(编排者裁定)——出数据喂#35完全分类,非SOLVE最终解 |
| 15 | claude-sci/gap3-trigger | feat(gap3): TW三阶段parity验收关口(步骤1/2完成,步骤3缺口B待触发判据裁定) |
| 16 | claude/add-ceremony-command-A5vJl | chore: update session state after PR review fixes |
| 17 | claude/bold-tesla | fix: 包含处理方向偏置修复(dir_state=None时用K线方向推断) + Codex审查记录 |
| 18 | claude/cc-session-briefing-setup-TXHg8 | feat: 026 现金边消歧层 + IYR 不动产数据补齐 |
| 19 | claude/ceremony-command-8NRQ8 | feat: v154-swarm——6工位完成 + 下游行动执行率69%→95%+ |
| 20 | claude/ceremony-feature-CInoK | docs: 015号谱系——方法不外在于内容 |
| 21 | claude/check-network-permissions-86zEe | chore: track ceremony-in-progress marker file |
| 22 | claude/dazzling-kowalevski-52a297 | feat: v71-swarm round2 同步——17条pending质询完成 + 490a /escalate 新增 + 四分法分类 + commit ff8046 补增 |
| 23 | claude/distracted-herschel-75e391 | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 24 | claude/elastic-noyce-f82f92 | chore: 研究材料 + 文件索引 + v69 sessions + 依赖更新 |
| 25 | claude/enable-agent-team-mode-2rNfS | chore: add ceremony-in-progress lock file |
| 26 | claude/infallible-rhodes-a27a56 | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 27 | claude/keen-cori-3a5b2f | fix: 走势引擎趋势判断修复(ZD/ZG替代DD/GG) + 美元信用溢价Δω + 影子金价覆盖率 |
| 28 | claude/merge-to-main-TXHg8 | chore: session 更新 — codex-bot 两个代码问题已修复 |
| 29 | claude/peaceful-cerf-ac6d83 | feat: 485号谱系结算(修正)——jet bundle术语降级为多层约束系统 |
| 30 | claude/persist-agent-team-MxfhB | fix: pattern-buffer 严格诊断修复——聚合、GC、一致性 |
| 31 | claude/priceless-boyd-b3b230 | fix: 结算483号元观察——从pending迁移到settled（worktree状态同步） |
| 32 | claude/quirky-matsumoto-c22fc2 | chore: 清理 483号生成态残留（pending 副本，settled 已存在） |
| 33 | claude/relaxed-lovelace-03bad0 | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 34 | claude/review-plagiarism-check-UhDT5 | chore: session 状态快照（hook 自动生成） |
| 35 | codex-line-20260625 | docs(549): relations.jsonl 来源诚实标注 — .bak重建非正典,守549精神不丢1034边,揭示549无remote有效域漏洞 |
| 36 | codex/292-trigger-source | fix(center_oscillation_trade): #292 续修——重基后挂起悬空泄漏 |
| 37 | codex/complete-classification-origin-20260626 | Add Round137 exact relation classifier bridge |
| 38 | codex/f240 | docs(chanlun): #235 Lean 孤岛五档处置表 + #238 接线形态核查（镜像自主仓） |
| 39 | codex/f242 | docs(chanlun): #240 formal↔rust 全对应核查（codex gpt-5.6-sol 执行，主 session 验收） |
| 40 | codex/p7-nest-anchor | docs(prereg): GAP-3 跨级别统计估计量 prereg 冻结——σ^H 条件化 z⁺ 双 estimand + 区辨判据 + 功效门 |
| 41 | codex/strict-formal-20260626 | feat(theta_v0): add strict formal engine package |
| 42 | codex/t6-gross-exposure | #146 T2 合并后修复：补齐 VoiceState/VoiceStepInput 新增字段（#149/#150 语义合流） |
| 43 | codex/typed-close-t1 | 归并对账报告封账：main 收编 cad69b2ef8、本体门 1736 PASS、残留 stash 登记（#117） |
| 44 | cursor/critical-bug-inspection-abec | fix: restore move trend GG/DD checks |
| 45 | dependabot/npm_and_yarn/frontend/npm_and_yarn-85e16ba948 | chore(deps): bump the npm_and_yarn group across 1 directory with 2 updates |
| 46 | dependabot/npm_and_yarn/frontend/npm_and_yarn-d447b59e3c | chore(deps): bump the npm_and_yarn group across 2 directories with 2 updates |
| 47 | dependabot/uv/uv-5abb2dfbbd | chore(deps): bump pyasn1 in the uv group across 1 directory |
| 48 | docs-443-stat-provenance | docs(agents): #443 统计结论口径标注与引用审查规范——轻/重两档+默认降级+五样本试判表 |
| 49 | exp-155-wf7-l1-gate | #145 T1：typed 出场裁决前移到组合层决策点（StepTrace.closed 三元化） |
| 50 | experiment/t14-t5-root-flip-necessity | feat(experiment): T14 根多空对称翻转 + T5 涌现出场(环21/环20必然性推论) |
| 51 | facea-impl-113-20260623 | merge(facea): R1-R4 失败模式修复链集成 — R2 T3出场诊断(observation-only) cherry-pick + R1/R3/R4/shortleg |
| 52 | feat/unn-necessity-corollaries | feat: unn 严格实装 4 条近似推论 T1/T5/T14/A5(根空头 MtM 翻转,prove 全 PASS) |
| 53 | fix-T3-leak-20260624 | diag(recursive_t): #164 R2 T3出场leak可约性诊断 — 修复L3否证(出场侧不可约) |
| 54 | fix-TC-churn-r4 | diag(recursive_t): #164 R2 T3出场leak可约性诊断 — 修复L3否证(出场侧不可约) |
| 55 | gap3-rework-codex9-fix | Use bar open for regular order fills |
| 56 | gap3/trigger-inv2 | feat(gap3): TW三阶段parity验收关口(步骤1/2完成,步骤3缺口B待触发判据裁定) |
| 57 | impl/397-stop-in | refactor(strategy): #397 抽出 stop_in 构造不变量——两处 22 行重复收敛 |
| 58 | impl/400-has-residual | fix(account): #400 影子评审三条订正——容差措辞+失真注释+边界单测 |
| 59 | issue-181-selldecision-cleanup | #152 终验A：ADR 0001 对齐审计报告（教义锚） |
| 60 | issue-412-pi-bsp-timing-args | fix(rust): #412 pi_bsp_timing 参数解析静默失效——窗口切片改按显式字段判定 |
| 61 | issue-434-level-origin-removal | docs(rust): #438 CI 注释补 #452 issue 号——TwStepCtx 阻断已落点 |
| 62 | issue-451-guard-role | fix(guard): #451 nest_isolation_guard 从文件名白名单改按角色豁免 |
| 63 | issue-452-twstepctx-visibility | fix(coverage): #452 摘除 TwStepCtx 误标 cfg(test)——生产路径 fill.rs 消费点解阻 |
| 64 | kimi-nest-mainline-20260717 | feat(theta): 接通活假设账本生产侧车 (#421) |
| 65 | main | feat(backtest): #467 链门语义订正——π 单门双覆盖钉案 + 出场归因统计落可达臂 |
| 66 | merge-mainline-20260719 | Merge branch 'kimi-nest-mainline-20260717' into merge-mainline-20260719 |
| 67 | orbit9-A-h0skeleton | feat(orbit9-A): H⁰方向骨架 flip confirmed门 + τ手性对称化 (task#39) |
| 68 | orbit9-B-dispatch-20260624 | fix(orbit9-B): O6 no-hedge被动基座取代bf17bb91c4开-hedge爆仓版 — verify-nohedge L3逐字复现 |
| 69 | orbit9-D-integration-20260624 | test(orbit9-D): rec_btc 加 ORBIT9 整合观测(h0_flip_blocked/A''抢先/B add, observation-only) |
| 70 | orbit9-hloc-20260624 | docs(orbit9-hloc): h-locality(ANCHOR)×full-ON L3否证编排者regime溶解假设 |
| 71 | orbit9-next-a-20260624 | feat(orbit9-next-a): 接通B/C 9轨道完整分类 — orbit9_sub_trend_done严格映射(located∧type1) + L3否证 |
| 72 | orbit9-nt-port-20260624 | feat(orbit9-nt): A/B/C镜像移植t_engine.rs(NT生产路径) + NT口径L3 |
| 73 | pre-cleanup-main | feat: 485号谱系结算(修正)——jet bundle术语降级为多层约束系统 |
| 74 | prop4-bidir-consume-C-20260623 | feat(recursive_t): 命题4读法乙双向(construct+consume)+开放轴C+诊断 L3 |
| 75 | prop4-nest-readingB-20260623 | feat(orbit9-C): #41 区间套H¹普适实例化定位算子 — covering tower逐级收缩到最低活跃级别 |
| 76 | proto-gatepass-20260722 | 收口提交（claude 438f5dc7 承接 kimi 030bee0b，用户指令代办）：#105/#106/#111 遗留 rust 实装 + harness 成套（.chanlun/WORKERS.md·locks·runbooks）+ 090 镜像修复 + 11 关验收矩阵与收尾产物归档。p7_inputs（23M 派生数据）不入库，登记于 chanlun/review-results/worktree-leftover-inventory-20260719.md |
| 77 | rescue/ci-438-test-20260727-concurrent-20260727-1348 | docs(review): 影子评审批次归档入仓 + docs/agents 补齐两份未跟踪规范 |
| 78 | route-bsp-complete-20260623 | feat(recursive_t): 全仓保证金 enable_cross_margin + L3(task #38) |
| 79 | shortleg-symmetric-entry-20260624 | feat(recursive_t): 做空腿开仓对称化(#108根因)——ShortEntry{T3默认/T1候选A/Any候选B} |
| 80 | strict-nesting-20260708 | docs(theta): P4 严格区间套回归与封边界——全量回归 PASS + 实装汇总报告 + 外推边界声明 |
| 81 | swarm/fix-S1-noleg | feat(recursive_t): 584定仓A/B并入R3(#6) — 核心腿涌现升级定仓来源对照 |
| 82 | swarm/fix-T1-direction | feat(recursive_t): T1方向错位修复(#164/R1)——开多入场方向/级别门控(LongEntry)+核心豁免 |
| 83 | t5-149-shortdiff | #149 T5：短差显式机制端到端 |
| 84 | t6-gross-exposure | #135 T6：毛暴露递归资金约束——子对冲额度 ⊆ 父级短差额度 |
| 85 | task70-merge | test: 修复 R5 opsem dump 全量回归 env 竞态——OPSEM_DUMP_DIR 改线程局部注入 |
| 86 | ticket-466-d0 | feat(theta): add #466 D0 rebase observability |
| 87 | ticket-472 | test(theta-v0): 锁定 Rebased 核销接线 (#472) |
| 88 | ticket-473f | chore(theta): add #473 third-class funnel probe |
| 89 | ticket-483 | fix(classifier): #483 拒绝 C 端点反向越界 |
| 90 | ticket-486 | test(classifier): 修复 #486 q7 覆盖退化 |
| 91 | ticket-69 | docs(rules): #69 裁定①——不可变条款补适用范围注记（replay 驱动性能缓存不适用，锚 R2/R3 + #494 翻转条件） |
| 92 | top-all-bsp-short | test(recursive_t): 顶层全部卖点做空闸门四路 L3 验证（556号） |
| 93 | traycer | chore(519): SKILL.md § 2.6 + § 6 (M2) + § 9 修订 + 519 号谱系 |
| 94 | unbind-confirm-src-38 | nest: 拆除 confirm_src≡interval.end 装配绑定，证书链保留独立确认时点（#38） |
| 95 | wip-orphans-20260625 | wip(防遗漏快照): 固化主仓工作区全部未提交改动 |
| 96 | worktree-agent-a0389ec5753435684 | fix(ceremony): 补全 step 5b 按需工位消费端 + 三类工位区分 |
| 97 | worktree-agent-a1857ac32ba591225 | feat(recursive_t): 1s prove 守卫尺度不变性 L3 + L4 突破——多标的×多窗口 |
| 98 | worktree-agent-a19bbaf9995460c33 | review(self-review): 9塔构造异步自指审查 — L0主线一致 CONDITIONAL PASS + HIGH拍扁残余(completion_source第七轴声明-定义不一致) |
| 99 | worktree-agent-a1b324d1f1f9c6ff5 | chore(swarm): 中断点 — 新goal对照gpt结果包零遗漏形式化(L1定义基底+L2每级分类层完成,L3-L5+迁主塔A→B待,codex 5层蓝图持久化) |
| 100 | worktree-agent-a1d20aa5fa82dc3e3 | docs(audit): 缠论完整实装深度gap审计——区间套回测零调用=最大简化(假否证根源) |
| 101 | worktree-agent-a246c3b83f9580658 | docs(route-bsp): #36核心L3——emergent-long已honor(清现金非flip)+add=regime税+全8<BH |
| 102 | worktree-agent-a3c1604fc473623ee | fix(interp): #396 影子评审两条——单射前置条件显式化 + 补规则体 golden 锁 |
| 103 | worktree-agent-a3eac13cd25744e5d | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 104 | worktree-agent-a487440bf7bbca944 | audit: codex 真异质审计 9 塔整体一致性(FAIL) + payoff #97 重测(部分成立) |
| 105 | worktree-agent-a55138cbf237e9057 | feat(nautilus): 注册 pub mod nautilus 激活 Bar→S_Θ→Order 数据流自检(15/15 L1) |
| 106 | worktree-agent-a5a6a34acc07e3fb7 | docs(genealogy): 666 refine降级—级别相位从alpha候选→beta漂移投影(perm_p=0.69否证) |
| 107 | worktree-agent-a65acf72e2dc14645 | chore(swarm): 中断点更新——precompact(新goal L2→Nautilus→生产→性能最大化) |
| 108 | worktree-agent-a69a85bac6190be05 | wip(防遗漏快照): 固化主仓工作区全部未提交改动 |
| 109 | worktree-agent-a6c577e593f1cf675 | feat(hook): #30蜂群机制修复——auto-ceremony强制+matcher Task→Agent+ceremony扬弃075 spawn 6结构工位 |
| 110 | worktree-agent-a6cdff06703f0de30 | feat(recursive_t): route_bsp CC orbit 升格（561号 N9 极性协变，任务29） |
| 111 | worktree-agent-a78ead6af0f6f80e2 | feat(nautilus): 注册 pub mod nautilus 激活 Bar→S_Θ→Order 数据流自检(15/15 L1) |
| 112 | worktree-agent-a7cbcf5d4f39d2ab3 | feat(recursive_t): 1s prove 守卫尺度不变性 L3 + L4 突破——多标的×多窗口 |
| 113 | worktree-agent-a7e569944b1d9a46d | fix(coverage): #398 消除拆分引入的 2 条 unused import 告警——回到 main 告警基线 |
| 114 | worktree-agent-a883448afa390e338 | docs(audit): 缠论完整实装深度gap审计——区间套回测零调用=最大简化(假否证根源) |
| 115 | worktree-agent-a8bbb19daba5402bb | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 116 | worktree-agent-a9f43a8c08a3c1ccc | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 117 | worktree-agent-aa1f62e1595e219aa | docs(audit): 缠论完整实装深度gap审计——区间套回测零调用=最大简化(假否证根源) |
| 118 | worktree-agent-aa324e3ed9e9fd19f | docs(549): relations.jsonl 来源诚实标注 — .bak重建非正典,守549精神不丢1034边,揭示549无remote有效域漏洞 |
| 119 | worktree-agent-aa3821c25e4bcffb9 | docs(recursive_t): 8标的×3模式 L3 验证——prove守卫 L2→L3 + 现状基线 |
| 120 | worktree-agent-ab4db18d22db8694c | feat(task#5): payoff G轴 = 级别-方向对齐门 + 强平可达性（539 支配失血分量修复） |
| 121 | worktree-agent-ac3a2924297104458 | feat(nautilus): 注册 pub mod nautilus 激活 Bar→S_Θ→Order 数据流自检(15/15 L1) |
| 122 | worktree-agent-ac95bfeffbc771eb7 | docs(549): relations.jsonl 来源诚实标注 — .bak重建非正典,守549精神不丢1034边,揭示549无remote有效域漏洞 |
| 123 | worktree-agent-ace0022ed478bed16 | fix(compact): 捕获真中断点写入 session ## 中断点 章节（破除循环引导） |
| 124 | worktree-agent-ad1d992cede5cd786 | feat(recursive_t): 1s prove 守卫尺度不变性深度验证——滤波器比喻尺度维度实证 |
| 125 | worktree-agent-ad21219d4b564138e | docs(dispatch-dag): 对齐 event_skill_map 声明层到 562号扬弃（结构=常设teammate+守卫层） |
| 126 | worktree-agent-ad3eb3b39639b8306 | test(recursive_t): 死锁全图 L3 三个半矩阵快照(548号)——cascade flip 否定性结果 |
| 127 | worktree-agent-adaa2b4719750e157 | chore(swarm): precompact — 中断点更新（L0/L1形式化+TW端native闭合，八commit；恢复后并行全部最严格4方向） |
| 128 | worktree-agent-adc7ffe1de7988faf | feat(recursive_t): 移植旧引擎 prove 守卫族（6守卫·rec+flat对称·bit-exact） |
| 129 | worktree-agent-ae127f2abfab73c0e | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 130 | worktree-agent-ae517ccfbf0a8080a | docs(genealogy): 起草 597-601 五条谱系记录(生成态,塔形式化压缩谱系态) |
| 131 | worktree-agent-ae691a4a585bbef12 | docs(audit): 缠论完整实装深度gap审计——区间套回测零调用=最大简化(假否证根源) |
| 132 | worktree-agent-aeac8e36d80879e2b | fix(rust): #394 评审订正——真实二进制 bit-exact witness + 见证测试去重言式 |
| 133 | worktree-agent-aeb1c6afb338546fd | feat(btc-l3): BTC 1秒 L3 决定性判据 — 第四轴坐实 + 配额维瓶颈否证(瓶颈在信号层539) |
| 134 | worktree-agent-aebd1e8f008f2ff3e | integrate(fix-structural): dispatch-dag 声明层对齐 562扬弃075（结构=常设teammate+守卫层） |
| 135 | worktree-agent-aec7bff16fc502aff | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 136 | worktree-agent-af3878feb35eb9a1a | feat: 谱系484号——multi_tf架构审计（完整实现+跨TF区间套链路缺口） |
| 137 | worktree-agent-af41a2af699aa4763 | feat(tower-sub5): 逐级内在配额 mobile_frac(L_pullback,L_confirm) + OFF bit-exact |
| 138 | worktree-level-dx | feat(W-VERIFY): Π_full μ̂ L2 检验完成（P1+P4+P7 后全历史 BTC） |
| 139 | worktree-wf_2f91805d-f6f-1 | feat(recursive_t): A 路本体实装——对称双吃最高级别精确切换(D_TOP区间套级联+anchor条件门控+平空对称) |
| 140 | worktree-wf_5d177a52-0fb-1 | docs(genealogy): 554号 A路D_TOP区间套链坍缩L3否证(吃跌三条路全否证) |
| 141 | worktree-wf_844a04d3-ddb-1 | test(recursive_t): 读法B 8标的×3模式 L3 全量验证 harness + 持仓时长by方向观测 |

## 汇总

- worktree 三态：主仓 1，可立即删 1，待定 24，须留 49
- 分支三态：主分支 1，可立即删 11，待定 117，须留 12
- 无票号 worktree：49 个（其中 agent-* 系 26 个）
- 异常：`worktree-agent-a3eac13cd25744e5d` / `a8bbb19daba5402bb` / `aec7bff16fc502aff` / `af3878feb35eb9a1a` 四个分支钉在 2026-04-20 的 19b4015927（谱系484审计），领先 main 1171——深度分叉，删前须确认无独有产出。
- 白名单：`/private/tmp/kimi-nest-mainline`（#421 在制品，脏 16 文件，领先 main 103——真实工作线）
