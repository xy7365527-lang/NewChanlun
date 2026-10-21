# #1588 工程债清零台账（2026-10-20）

主控：owner_1588。基线：origin/main `1d70db893f`（开工时）。每条按「现象 → 复现命令与输出 → 根因 → 处置」记。处置三选一：修（附 PR 与 CI）、判环境问题（附证据）、转票（附理由）。各分片详细证据在同目录子目录与对应 PR；非作者独审记录在 `reviews/`（评审模型 claude-fable-5-1，实施模型 claude-opus-5-5，经本机 claude CLI + CLIProxyLocal）。

所有修复均为独立分支、开 PR 到 main、未合入；合入走用户闸，按 #1587 用 squash。

**DevSkim 对其它 PR 的影响**：在 #1593 合入前，main 及所有 PR 的 DevSkim 都是同一批存量红（约 55,148 条，全部在归档/测试数据目录），各 PR 均已核实本 PR 改动文件命中 0 条。建议合入顺序：#1593 先合（DevSkim 转绿），#1592（CI 拆分，main 未拆分时 rust-check 已到 137m40s）次之，其余无文本冲突；#1615 与 #1617 将来都改 `main-swarm.mts`，#1617 动手前先看 #1615 是否已合。

## 总览

| # | 债项 | 处置 | PR / 票 | 独审 |
|---|---|---|---|---|
| 1 | DevSkim 在 main 长期红 | 修 + 按规则排除归档/测试数据（需用户裁定与 #1474 的关系） | PR #1593；#1607 | PASS-WITH-NITS；尾部补审 PASS |
| 2A | #1488 宽对拍「基线即失败」16 条 | 转 #1488（对 main 是 #1488 新增的红，#1488 已接手合入前清零） | PR #1594（报告） | PASS-WITH-NITS；尾部补审 PASS |
| 2B | v3::financial_session 在 WSL 22 条失败 | 判环境问题（平台限定预期行为） | PR #1594（报告） | 同上 |
| 2C | m3_partition_synthetic_holds 稳定性 | 稳定（CI 连续 9 次绿）；「停产锁失效」转 #1493，已补锁 | PR #1594（报告）；#1582 `9f3b5281eb` | 同上 |
| 3 | CI rust-check 2h10m 逼近 150 分钟 | 修：拆并行 job + s_session 测试按名分片 | PR #1592 | PASS-WITH-NITS；尾部补审 PASS |
| 4 | 解析入口内存随前缀二次增长 | 修回放路径（E-b）与可隔离部分（E-a 部分）；主体转 #1597 | PR #1596；#1597 | PASS-WITH-NITS；尾部补审 PASS |
| 5 | 各票 P3/观察项中「当前就错」 | 4 处修（含评审新发现的同类泄露）、1 条转 #1488、3 条开票、2 条已有专票 | PR #1611；#1608/#1609/#1610 | PASS-WITH-NITS；尾部补审 PASS |
| 6 | nc-remote 共享 CARGO_TARGET_DIR 导致证据污染 | 已修（私有 NC_TARGET + flock）；本票窗口内读数全部复跑 | 工具改动不入仓，见 §6 | — |
| 7 | wayfinder_engine launchd 从用户工作区运行，#1578 修复从未生效 | 主控已改 plist 指向 sandcastle-host 并重启；仓内脚本硬编码路径修复中 | PR #1614；#1617（见 §7） | PASS-WITH-NITS；尾部补审 PASS |
| 8 | wayfinder spec 起草子代理默认 deepseek，违反「子代理只用 Claude 家族」 | 修复中 | PR #1614；#1617（见 §7） | PASS-WITH-NITS；尾部补审 PASS |
| 9 | Sandcastle 评审工蜂把整段 diff 塞进提示，大 PR 静默失败 | 修复中 | PR #1615；#1618（见 §8） | PASS-WITH-NITS；尾部补审 PASS |
| 10 | Sandcastle 实现工蜂在 macOS 本机跑 cargo | 修复中 | PR #1615；#1618（见 §8） | PASS-WITH-NITS；尾部补审 PASS |

## 1. DevSkim 在 main 长期红

- **现象**：main 上 DevSkim 连续失败（如 run 39379637868）。SARIF 91,438 条，基线（`.github/devskim-baseline.json`，8,201 条，#1194 冻结）外新增 83,220 条；另 gate 写 `$GITHUB_STEP_SUMMARY` 6.3 MB 超 1 MB 上限报错。
- **复现**：`gh run download 39379637868 -n devskim-sarif` 后按 `rule|path|line` 与基线做差集 → `baseline 8201 new 83220`。分布：约 83,140 条在 `.chanlun/review-results/**`（评审证据归档，几乎全是 DS173237 对 SHA-256/Git 对象名的疑似 token 误报）与 `s_session/tests/fixtures/**`；源码面 80 条（加 fixtures 内 1 个 `.py`，共 81）。
- **根因**：评审证据归档与测试数据目录持续入仓大量摘要值；基线键含行号，行移即失效（7,944 条已失效）；摘要无截断。
- **处置（PR #1593）**：81 条源码面逐条分诊，无安全缺陷——66 条行内抑制（同行尾注、带理由）、4 条改写已完成的 TODO 措辞、11 条字节受哈希锁的文件按精确键入基线并逐条写理由；`.chanlun/review-results/**` 与 `s_session/tests/fixtures/**/*.json{,l}` 用 ignore-glob 排除，排除前全量凭据扫描（1,812 文件、15,130 个 zip 成员、535 个解码附件）0 命中；gate 新增 `--summary-max-new`（默认 200），stdout 仍全量，判红与退出码不变。PR run 39383843793 DevSkim PASS（0 条基线外新增）。独审 N1（前导零按八进制解析）已在第二轮修复并补测试。
- **需用户裁定**：目录级排除与 #1474 已批口径「不能只凭字段名、目录或 40/64 位形状放行」有张力。本 PR 的依据是 #1588 票面「按规则排除归档文件，排除须附理由」并配全量凭据扫描；代价是今后写入这两个目录的新文件不再被扫描。替代路线是等 #1474 稳定告警身份方案。详见 PR #1593 描述「需要裁定」节。
- **派生转票**：`strategy.rs:314` 限价腿 entry 价回填由已有 #1385（OE09）承接，已在 #1385 留言；`replay_comparator.rs:151` 完整 CERT/DIV/TERM 对拍新开 #1607；基线 7,944 条失效键归 #1474。第二轮 head `1b2ab76227`：DevSkim run 39401546164 PASS，CI run 39401546269 绿（pytest 5,065 passed）。
- 证据：`devskim/DEVSKIM-TRIAGE.md`，评审 `reviews/1593-claude-fable-5.md`。

## 2. 基线失败测试

### 2A. #1488 宽对拍「基线即失败」16 条

- **现象**：#1488 宽对拍过滤 `v3::{audit_owner,request_audit,cross_request_source_cache,fixed_radix,driver}` 有 16 条失败，名单称「基线既有」（`1488-baseline-failures.md`）。
- **复现**：`nc-remote <ref> rust cargo test --locked --features s_session,s_capture_native_testonly --bin s_structure_session --no-fail-fast -- v3::audit_owner v3::request_audit v3::cross_request_source_cache v3::fixed_radix v3::driver`。main `1d70db893f`：103 passed / 0 failed / 5 ignored（私有 target 复跑 `20261020-160727-1d70db893f` 同）；#1488 头 `c2bdf9b662`：236 / 16 / 5，失败集与名单逐名相同。
- **根因**：所称「基线」`959d860065` 是 #1488 自身 WIP，不在 main。16 条中 11 条在 main 上不存在（由 #1488 线 `3f2b4fd924`、`9d03c773ae`、`ea41d4dfd1` 引入），5 条在 main 上绿（其中 4 条在 #1488 线首个可编译提交 `25333c0f2a` 已红）。
- **处置**：转 #1488。#1488 负责人已核实接受并排进切片 0058（逐条 bisect 首红、分类为生产回归/语义改变/夹具问题），合入 main 前清零。对 main 而言这是 #1488 的合入门条件。

### 2B. v3::financial_session 在 WSL 22 条失败

- **复现**：`nc-remote 1d70db893f rust cargo test --locked --features s_session --bin s_structure_session v3::financial_session` → 4 passed / 22 failed / 3 ignored（私有 target 复跑 `20261020-160510-1d70db893f` 同）。
- **根因**：22 条失败点全是 `MissingDependency：会话保留量读数当前只在macOS实现`（`financial_session.rs` 非 macOS cfg 分支，依赖 heap_probe 与 getpeereid）。macOS CI run 39358379664 同批 26 / 0 / 3 全绿；Linux CI 以 `--skip v3::financial_session` 显式记账（#1394 根裁定 A）。
- **处置**：判环境问题，不改码。不加 cfg 门控的理由：测试在 Linux 上会直接消失，不如「诚实地红并写明原因」；macOS 作业是该批唯一判据。可选后续：把整批 skip 收窄到具名列表，让 4 条平台无关用例在 Linux 也跑。

### 2C. m3_partition_synthetic_holds 稳定性

- **复现**：main CI 连续 9 次「cargo test — default targets」步绿（39321369264 → 39380862852）；WSL 私有 target 复跑 6 次（`20261020-160635-1d70db893f`）均 ok，读数逐位相同 `|ledger|=1 kept=1 same_bar=0 nonpos=0 |records|=1`。
- **根因**：无随机性。转绿来自 #1420 的 `279b7c8708` 改变段划分（不是 #1493 的 `e79bdb2a52`，后者不在 main）；该种子在 main 上仍停产（14 段），#1582 头 32 段，m3 两边都绿——它唯一门槛是 ledger 非空，已锁不住停产。
- **处置**：稳定性无问题（N≥5 已满足）。「停产锁失效」转 #1493：#1582 已加 `m3_synthetic_segment_layer_does_not_stall`（`9f3b5281eb`，段数≥30、末段终点距末笔≤150 根），#1493 票面已写明前提失效。

证据：`tests/REPORT.md`（PR #1594），评审 `reviews/1594-claude-fable-5.md`。

## 3. CI 时长

- **现象**：main run 39358379553 的 rust-check 单 job 130m35s，timeout 150。
- **复现**：`gh run view 39358379553 --json jobs` 逐步耗时：s_session 正式结构会话 89.1m、s_capture_native_testonly 增量 15.4m、backtest_bin 9.3m、default targets 8.8m，其余各 ≤1.5m。
- **根因**：全部 Rust 步串行在一个 job；s_session bin 一步执行 88 分钟（编译仅 1m05s），tb02w/tb02c 两条长测试晚启动造成约 47 分钟拖尾；runner 机型差异约 2.2×。
- **处置（PR #1592）**：拆成 rust-check / rust-test-default / rust-test-s-session（4 片）/ rust-test-s-session-coverage / rust-test-s-session-extras / rust-test-backtest-bin 并行；分片脚本 `test-shard.py` 以 `--list` 动态生成全集，当场断言「两两不交、并集==全集」「执行集==分配集」，coverage job 汇总再核。#1141 遥测、`--jobs 1`、不缓存 target 均保留。三轮 PR CI：最长 job 31m（EPYC 9V45）/ 67m（EPYC 7763），测试 6,839 passed / 0 failed / 311 ignored 与拆前逐名一致（7,052 行 diff 为空）；磁盘余量增加。第二轮按独审 nits 与 P3 A2 改 artifact overwrite、分片 timeout 110、缺产物结构化报错、`if: always()`→`!cancelled()`、加 PR concurrency 组；rebase 到 `24c2fa068f` 后 head `3fee4c1aff` run 39425283861 全绿，最长 job 63m08s（9V74），各轮最慢 69m55s（7763），计数 6,877 / 0 / 337 与新 main 逐名一致（7,116 行）。对照：未拆分的新 main `24c2fa068f` rust-check 在 7763 上已达 137m40s（run 39417635987）。
- **遗留**：最长 job 下限是 tb02w 单条测试本身；进一步压缩需 test profile 开优化或拆 tb02w/tb02c，超出本票手段，建议转票。新增 check 名需在设分支保护时列入（main 当前无分支保护）。
- 证据：`ci-time/ci-time-1588.md`，评审 `reviews/1592-claude-fable-5.md`。

## 4. 解析入口内存随前缀二次增长

- **现象**（#1493 附带发现 E）：`parse_layer_with_inclusion_facts` 观察路径尾窗 2k→20k 根 RSS 0.4→9.3 GB；`ParseLayer` 跨 bar 持有时 `IncrSegments` 的 `Rc::make_mut` 每 bar 整表复制。
- **复现**：探针 bin `issue1588_parse_mem_probe`（`backtest_bin` feature），`NC_TARGET=eng1588 nc-remote <ref> rust bash -c '<build release; 各模式 × n=2k/5k/10k/20k/49,995>'`。
- **根因**：E-b（生产回放）：`classify_at` 在 append 期间仍持有上一 bar 的 `last_l0`，共享 `Rc` 表在 `make_mut` 时逐 bar 整表复制，累计 O(n²)；同样写法见 `s_structure_session` 逐 bar 回放与 4 个诊断 bin。E-a（完整事实入口）：来源坐标展开存储 Θ(n²)。
- **处置（PR #1596）**：append 前丢弃上一轮 layer（调用方一侧，未碰 #1582 在改的 segment.rs/feature_seq.rs），`stroke.rs::sorted` 去重后 `shrink_to_fit`；新增回归测试 `classify_at_keeps_shared_parse_tables_in_place`（反证：撤回一行即失败）。私有 target 复跑（修前 `20261020-172803-da78044035`、修后 `20261020-173051-e52cbb0171`）：classifier 累计分配 20k 根 20,511.6 → 937.6 MB、49,995 根 127,944.2 → 4,554.0 MB；完整事实入口留存 20k 根 9,530.9 → 6,518.1 MB；21 组测量的输出摘要修前修后逐位相同（另补跑 replay 5k 逐 bar 与 classifier 49,995 修前侧，亦一致）；存活峰值线性。独审 nits 在 `1bc3534014` 处理；head `33b8036784` CI run 39405420119 rust-check/test/fixture-drift/s-observer 绿。
- **转票**：E-a 主体转 #1597（需改公开事实类型，消费方在 #1488/#1493 在途区），replay-drop 累计分配略超线性作为观察项记入 #1597。
- 证据：`memory/REPORT.md`，评审 `reviews/1596-claude-fable-5.md`。

## 5. 各票 P3/观察项中「当前就错」的部分

收集结果全文：`P3-SWEEP.md`（只读收集，origin/main `bf871af9ce`，静态证据 + main CI run 39380862852 告警计数）。

| 条目 | 现象 | 处置 |
|---|---|---|
| A1 | `.sandcastle/codex-child.ts` 的 `git()` 把完整 `process.env`（含 provider 密钥）传给预检 git 调用，与 PR #1480 声明不符；独审又发现 `delivery-bundle.ts:126` 同类 | 修：共用白名单 `git-environment.ts`，两处测试（修前版本失败），PR #1611 |
| A2 | ci.yml `if: always()` 在取消后仍跑、无 concurrency 组 | 修：10 处改 `!cancelled()`，PR 级 concurrency（非 PR 用 run_id 独占组），并入 PR #1592 |
| A3 | `bi.md:338` 指向不存在的 `AFFECTED-CODE.json` | 修（纯指针订正，不改教义），PR #1611 |
| A4 | `content_producer.rs:3748` 文档注释描述旧生成路径 | 转 #1488（该文件在 #1488 大改中；已排进其切片 0061） |
| A5 | 编译告警回潮（lib 81 / lib tests 84 / s_structure_session tests 159），含弃用调用、未声明 cfg 致代码从不编译、测试丢弃 Result 可能假绿 | 开票 #1608 |
| A6 | `signal.rs:1588` 一类漏斗探针第二臂误写，上涨臂永不求值 | 修 + 合成夹具单测（修前版本失败，`20261020-221939-4f510470bc`），PR #1611；真实数据普查无法区分（趋势门恒 0），探针 parity 未建模 D2 门另记 |
| A7/A8 | v3/v1/v2 init 在失败时留残库 | 已有专票 #1568 / #1570 |
| Q1 | Linux 上 stop-s/stop-q 10 s 后判 FAIL、随后进程已不在 | 开诊断票 #1609 |
| Q2 | ADR 0040 三条 Codex 评论未答，与 ADR 0029 冲突 | 开裁定票 #1610 |
| — | #1520 三条测试已在最近 6 次 main run 绿，内容被 #1563 覆盖 | 建议按重复票关闭（对外动作留用户） |

## 6. nc-remote 共享 target 导致证据污染

- **现象**：2026-10-20 12:00–15:38 windows-5090 WSL 上 nc-remote 所有运行共用 `CARGO_TARGET_DIR=~/nc-remote/target`，并发运行会互相覆盖测试二进制，读数可能张冠李戴（#1488 发现并通报）。
- **根因**：`~/.local/bin/nc-remote` 写死共享 target，且无互斥。
- **处置**：已修（#1488 侧改）：`CARGO_TARGET_DIR=~/nc-remote/${NC_TARGET:-target}` + 对 target 加 `flock` 串行；改前备份 `~/.local/bin/nc-remote.bak-20261020`。工具在仓外，不入仓。本票在窗口内产生的远端读数（2A main 侧、2B、2C、4 的修前/修后曲线）已全部用 `NC_TARGET=eng1588` 串行复跑，读数逐项一致（日志见各节）。#1488 头一侧的 236/16 属 #1488 自证范围。

## 7. wayfinder 引擎运行位置与子代理模型

- **现象 ①**：`wayfinder_engine` 的 launchd 任务从用户工作区 `/Users/silencehan/Projects/NewChanlun`（无关分支、683 个脏文件）运行，自 10-18 未重载，#1578 的修复从未生效。
- **处置 ①**：主控已把 plist 改指 sandcastle-host 并重启（备份 `/Volumes/AgentStorage/wayfinder-engine.plist.bak-20261020`），新引擎首轮扫 145 张并放行 #1406。PR #1614：`run-wayfinder-engine.sh` 与 `watcher.sh` 改按脚本位置推导宿主仓（`SANDCASTLE_HOST_REPO` 可覆盖，与 `run-claim-loop.sh` 同款），plist 模板与安装脚本改指 sandcastle-host。两个用户工作区 plist 核查（只读）：sandcastle-watcher 应改指 sandcastle-host（会跑用户分支上的旧脚本，且每 10 分钟重写一个受控 roster 文件），合入后把 plist 脚本路径换成 host 即可，logs 经软链共用不丢数据；issue933-sampler 不应改（脚本只在用户工作区且被 gitignore），但 #933 已于 08-18 关闭仍每 2 小时采样（`samples.jsonl` 5.8 MB），停或迁需用户决定。
- **现象 ②**：`scripts/wayfinder_engine.mts:51-52` spec 起草子代理默认 `deepseek/deepseek-v4-pro`，违反用户「子代理只用 Claude 家族（opus 5.5 / fable 5.1）」。
- **处置 ②**：PR #1614。prime-agent 0.10.0 的 cliproxy-local 只登记带前缀型号、无 fable，故改用 `claude-route.mts` 同款 Claude Code CLI + `cliProxyEnv()`；默认 `claude-opus-5-5`，白名单仅 opus-5-5 / fable-5-1，其它值启动即 FAIL-loud；子进程环境删 `ANTHROPIC_API_KEY` 防绕开代理。测试 37→43 本机全过（这批 TS 测试尚未进 CI，见 #1617 §2）；真路由冒烟 `out="OK"`。head `f8b06a0090` CI run 39479499053 绿。其它非 Claude 默认位置（main-swarm k3、镜像 prime-agent 默认、deepseek 标签通道等）转 #1617。

## 8. Sandcastle 执行面两处缺陷

- **现象 ③**：评审工蜂把整段 `git diff main...branch` 塞进提示，#1406（36 提交、约 39 万 token）直接「Prompt is too long」失败并静默结束。
- **处置 ③**：PR #1615。评审提示改宿主侧渲染：只放 diff --stat、提交列表、文件清单与 issue，以刚取回的 origin/main SHA 为基准按需读 diff；估算 60k token 上限，超限 exit 4 + `workers.jsonl` `reviewer/failed reason=prompt_too_large` + 原票评论；渲染期与运行期失败分别记 `prepare_error` / `run_error`。#1406 实测：旧形态 1,574,443 字符 ≈ 393,611 token，新形态约 8,184 token。
- **现象 ④**：实现工蜂在 macOS 本机跑了 cargo check/test，违反本机禁 cargo 约定。
- **处置 ④**：PR #1615。implement/review/merge 提示与 CODING_STANDARDS 明令本机禁 cargo，验证走 `NC_REPO=… NC_TARGET=sandcastle-issue-<N> nc-remote <分支名>` 或 CI；`cargo-guard.mts` 在固定目录放 cargo/rustc/rustup/cargo-* 垫片（exit 87、打印指引）置于 PATH 最前，并经 `CLAUDE_ENV_FILE` 在每次 Bash 前重置。演练：真实 `claude --print` 的 Bash 工具里 `cargo --version` → 87；同 shell `nc-remote … cargo --version` → 远端 cargo 1.99.1、EXIT=0。新增 CI job `sandcastle-ts`（187 例）。head `8d15dffa9c` CI run 39480668191 绿。已知绕过（绝对路径、docker 蜂群无垫片）与实装失败静默、health-monitor 不告警等转 #1618。

## 9. 范围外与在案说明

- 已入 main 的 #1394 历史中间提交不可编译（12 个）：票面列为不在范围，防再发由 #1587（squash 合入）解决。
- GitHub Dependabot 报告默认分支 158 个依赖漏洞（2 critical / 63 high / 70 moderate / 23 low，推送时远端提示）：不在 #1588 票面五项内，建议另开依赖升级票。
