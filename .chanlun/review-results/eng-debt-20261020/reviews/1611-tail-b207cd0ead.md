# PR #1611 尾部补审（4de612c241..b207cd0ead）

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1611（head `b207cd0ead`，base `main`，OPEN）
- 上一次完整评审：`reviews/1611-claude-fable-5.md`（覆盖到 `4de612c241`，PASS-WITH-NITS）
- 本次范围：按交付纪律第 7 子句「评审面==关票面」，只审 `git diff 4de612c241 b207cd0ead` 的 4 个追加提交。
- 评审模型：claude-fable-5；评审日期：2026-10-21
- 评审方式：只读。未改代码、未推送、未评论 PR。本机不跑 cargo，Rust 结论基于作者留在本机 `/tmp/nc-remote/` 的日志（逐份打开核对）；TS 行为本机实跑。

## 结论：PASS

四个追加提交全部对应上一轮评审的发现 1–3（发现 4 作者明确「未改」并给出理由，我接受），处理方式与评审建议一致；没有越界文件、没有行为外溢；证据文字抽查属实（抽查了 A6 修前/修后两份日志，顺带把其余四份也核了，全部与 REPORT.md 逐字相符）。无阻断项，无新 nit。

## 尾部提交与处置对照

| 提交 | 对应评审发现 | 判断 |
|---|---|---|
| `7a01ee589e` fix(sandcastle) | 发现 1：`delivery-bundle.ts:126` 的 git 子进程仍继承完整 `process.env` | 正确。`gitEnvironment()` 从 `codex-child.ts` 迁入新共享模块 `.sandcastle/git-environment.ts`，函数体逐字不变（只扩了注释：改「预检」为「本地只读」，并加一句「需要访问远端的调用方不得复用」）；`codex-child.ts`、`delivery-bundle.ts` 都改为 import。`delivery-bundle.ts` 仅 1 行行为改动：`env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" }` → `env: gitEnvironment()`，白名单里已含 `GIT_OPTIONAL_LOCKS="0"`，原有语义保留。 |
| `55468c8079` test(classifier) | 发现 2：A6 只有静态依据，建议补合成夹具单测 | 正确且有效。新测 `type1_funnel_dx_up_trend_broke_arm_reachable` 放在 `signal.rs` tests 模块末尾；断言 `s_gate_open==3 / s_broke==2 / s_mapped==2 / s_diverge==0`，同时覆盖了评审要的两点：上涨臂 `(Some(Up), Up)` 可达（leave/top 两段），`(Some(Down), Up)` 落入 `_ => false`（retest 段不计）。作者用临时 revert 分支证明修前该测试失败（见证据 4）。 |
| `c779655232` review-results | 发现 3：919 passed 缺日志出处 + 补第二轮处置记录 | 属实。补了作者日志 `20261020-184250-1b27e492b0.log` 与评审复跑日志 `20261020-213958-4de612c241.log`，两份本机都在且内容相符。 |
| `b207cd0ead` style | — | 纯 `cargo fmt`：仅把一条 `assert_eq!` 换行，4+/1-，无语义改动。 |

## 逐条发现

1. **[无问题] 范围与越界。** `git diff --stat` 共 7 文件：REPORT.md、`codex-child.ts`、`codex-child.test.ts`、`delivery-bundle.ts`、`delivery-bundle.test.ts`、`git-environment.ts`（新）、`signal.rs`。全部在四个提交的说明范围内；`signal.rs` 只在 tests 模块末尾追加，生产区（含 `type1_funnel_dx` 本体）未动；`bi.md`、CODEX-CHILD.md 未再改。`gitEnvironment` 在仓内的全部引用（`grep -rn gitEnvironment .sandcastle/`）只剩新模块导出 + 三处 import，没有遗留对 `codex-child.ts` 旧导出的引用。新文件在 `tsconfig.sandcastle.json` 的 `.sandcastle/*.ts` include 内，tsc exit 0。

2. **[无问题] 新 TS 测试有效性。** 我把 `.sandcastle/` 拷到临时目录并用 `git show 4de612c241:.sandcastle/delivery-bundle.ts` 覆盖成修前版本，跑 `--test-name-pattern="1588"`：`not ok 1 … error: 'git 环境含 CLIPROXY_LOCAL_KEY'`，与 REPORT.md 描述一致。修后 head 全量 72/72 通过（69 + A1 两条 + 本条）。测试用 `t.after` 恢复 `PATH`/三个密钥名，不再依赖测试顺序，顺带回应了上一轮发现 4。密钥只用夹具字符串，包装器只记变量名不记值。

3. **[无问题] A6 单测的夹具推演。** 中枢/段沿用 `first_sell_mirror_with_trend_divergence`（C0 end=2，C1 end=8，上涨趋势）；A、B 两段 nearest center 为 C0（pos=0）被 `pos<1` 过滤，C1 之后三段过门 ⟹ `s_gate_open==3`；leave 420、top 430 均 > C1.zg=400 且自锚 Up ⟹ `s_broke==2`；retest 自锚 Down、门方向 Up ⟹ `_ => false`。closes 改为 A 缓 C 急 ⟹ 无背驰，使尾部 parity 的环 6 等式（`prod_sb == s_mapped`）成为钉子。作者解释为什么不能直接用 1 卖夹具（parity 环 7 多计，`20261020-221105` 日志 `left 1 / right 2`）以及为什么放模块末尾（`bsp.rs:733` 按行号登记白名单，`20261020-221755` 日志显示插中间会让 `no_stray_bsp_binding_rewrites_outside_registry` 变红）——两份日志都在，与说法一致。这两处取舍我同意；parity 环 7 对 D2 大闸的既有缺口作者已列入「第二轮新增遗留」建议转票，与 A6 无关，不要求本 PR 处理。

4. **[无问题 / 抽查] 证据文字属实。** 抽查对象：REPORT.md「发现 2」的修前/修后两段引文。
   - 修前 `/tmp/nc-remote/20261020-221939-4f510470bc.log`：ref=`eng-debt-1588/p3fix-a6revert`，有 `--> src/theta_v0/classifier/signal.rs:1588:13` unreachable 告警，`panicked at signal.rs:1654:5`，`漏斗环6（mapped 候选）须=生产 struct_break 候选数 left: 2 right: 0`，`test result: FAILED. 0 passed; 1 failed`。与报告引文逐字一致。
   - 修后 `/tmp/nc-remote/20261020-222120-55468c8079.log`：sha=`55468c8079…`，`(lib test) generated 52 warnings`，无 unreachable 行，`test result: ok. 920 passed; 0 failed; 10 ignored`。与报告一致。
   - 另核 `20261020-184250-1b27e492b0.log` 第 438 行 `919 passed`（发现 3 的补证）属实。
   - 临时 revert 分支 `4f510470bc` 报告称已删且未推送，本地/远端均不可见，无法也无需复核；日志头 ref/sha 已能自证该次运行对象。

5. **[无问题] A4 去向。** REPORT.md 称「主控已转 #1488，由其切片 0061 处理」。`gh issue view 1488 --comments` 可见 0061 已完成并明确写「#1588 转交的 A4 已订正：content_producer.rs 里过时的文档注释，只改注释」。上一轮评审第 ② 轴的待确认项已闭合。

6. **[信息] CI。** head `b207cd0ead`：rust-check / test / s-observer / fixture-drift 绿，DevSkim 红。取该 DevSkim run（39456541175）failed log 统计 sandcastle 命中：`codex-child.test.ts` 9 + `codex-child.ts` 4 = 13，与上一轮比对的 main 既有 13 条相同；新文件 `git-environment.ts`、`delivery-bundle.ts`、`delivery-bundle.test.ts` 零命中。本尾部没有引入新的 DevSkim 告警，DevSkim 仍由 #1593 分片处理。

## 实际复核过的命令

| # | 目的 | 命令（在工位 `/Volumes/AgentStorage/eng-debt-20261020/p3fix`，gh 在 /tmp） | 结果 |
|---|---|---|---|
| 1 | 尾部范围 | `git log --oneline 4de612c241..b207cd0ead`；`git diff --stat 4de612c241 b207cd0ead`；`git diff 4de612c241 b207cd0ead` 全文读 | 4 提交、7 文件、157+/14- |
| 2 | fmt 提交纯格式 | `git diff c779655232 b207cd0ead` | 仅一条 `assert_eq!` 换行 |
| 3 | 引用面 | `grep -rn gitEnvironment .sandcastle/`；`grep -n include tsconfig.sandcastle.json`；`grep -n "1588\|白名单" .sandcastle/CODEX-CHILD.md` | 旧导出无残留引用；新文件在 include 内；文档描述仍准确 |
| 4 | TS 测试 + tsc | 临时软链 `/tmp/ge1547deps/node_modules` → `./node_modules`，`node --import tsx --test .sandcastle/codex-child.test.ts .sandcastle/delivery-bundle.test.ts`；`./node_modules/.bin/tsc -p tsconfig.sandcastle.json`；完毕删软链，`git status --porcelain` 清洁 | `tests 72 / pass 72 / fail 0`；tsc exit 0 |
| 5 | 新测试修前失败 | 拷 `.sandcastle/` 到 `/tmp/p3fix-tail-prefix/`，`git show 4de612c241:.sandcastle/delivery-bundle.ts` 覆盖，`node --import tsx --test --test-name-pattern="1588" .sandcastle/delivery-bundle.test.ts`；用后删目录 | `not ok 1 … error: 'git 环境含 CLIPROXY_LOCAL_KEY'` |
| 6 | 证据日志 | `ls /tmp/nc-remote/ \| grep -E "184250\|213958\|221105\|221755\|221939\|222120"`；`cat` 四份短日志；`grep -anE "test result" 20261020-184250-1b27e492b0.log` | 六份全在，内容与 REPORT.md 一致（见发现 3、4） |
| 7 | Rust 夹具推演 | 读 `signal.rs:1515-1659`（`type1_funnel_dx` 本体与尾部 parity）、`:3027-3076`（镜像夹具）、`:5649-5688`（新测） | 见发现 3 |
| 8 | PR / CI | `gh pr view 1611 --json headRefOid,baseRefName,state`；`gh pr checks 1611`；`gh run list --commit b207cd0ead… --workflow DevSkim`；`gh run view 39456541175 --log-failed \| grep -aoiE "sandcastle/[A-Za-z._-]+" \| sort \| uniq -c` | head 对、OPEN；DevSkim 命中 13 条同 main 既有，新文件 0 命中 |
| 9 | A4 去向 | `gh issue view 1488 --json title,state`；`--comments \| grep -n "0061\|3748\|A4"` | 0061 完成记录明确含「#1588 转交的 A4 已订正」 |
