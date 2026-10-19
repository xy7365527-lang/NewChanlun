# #1405 恢复部分（PR #1571）非作者全量独立评审 brief

你是非作者独立评审（模型 claude-opus-5-5）。用简体中文。你不是唯一在仓库工作的代理：除你自己的工作目录 /Volumes/AgentStorage/issue1405-closegap-control-20261020/review-0001 外不写任何文件，不改代码，不 commit，不删分支，不回退任何改动。

只读检出：/Volumes/AgentStorage/issue1405-review-ro-20261020（detached，= main b63a3fd586 + 文档补记提交 299485822f；含被评提交）。

## 范围（关票门第 7 子句：评审面必须等于关票面）

全量评审 `git diff 5baeb030a6..1870e95895`：5 个提交 6973642619 f60430fe91 74c386f19e c4bbdd9f8a 1870e95895，只改 rust/src/bin/s_structure_session.rs，经 PR #1571 合入为 b34b0278ee。#1571 原「非作者 Opus 可合」评审无原件且在 1870e95895 之前，所以这次必须覆盖全部 5 个提交，不能只评尾部。关票门见 docs/agents/delivery-discipline.md:7-22。

## 要回答

1. 正确性：open_db 先只读判 rule_revision、旧规则库返回只读连接、hot rollback journal 拒绝（StorageUnavailable）、recover_core 版本门与不建库——逻辑是否成立；有无会改写旧库字节的残余路径（检查点、WAL 设置、-shm、并发写者、未初始化库、rule_revision 缺失、只读探测与随后写连接之间的 TOCTOU）；当前规则库路径行为是否不变。
2. 测试：五个 tb1405_* 测试是否真的锁住上述行为（会不会因夹具构造而空过）。收据：.chanlun/review-results/issue1405-tb02v-20261014/recovery-20261019/mutant-be4268a1af.txt（mutant 提交 be4268a1af = c4bbdd9f8a 上把 open_db 的 `if db.exists()` 改为 `if false && db.exists()`，tb1405 过滤集 2 过 2 败）、.chanlun/review-results/issue1405-tb02v-20261014/recovery-20261019/test-s_session-bin-1870e95895.txt（86/0/1）。指出缺口。
3. 与交付合同（.chanlun/review-results/issue1405-tb02v-20261014/DELIVERY.md、.chanlun/review-results/issue1405-tb02v-20261014/recovery-20261019/RECOVERY-DELIVERY.md）及 AGENTS.md 纪律（如「同一判断不得有宽严两档」）的一致性；错误文案与注释是否准确；补记对 runtime FAIL（收尾 stop-s 超时）的归因是否站得住，是否影响验收 3（读 s_session/tests/tb02a_runtime.py、tb02b_runtime.py、tb02v_runtime.py、s_session/s_service_control.py 与 .chanlun/review-results/issue1405-tb02v-20261014/recovery-20261019/browser-225737/ 收据）。
4. 浏览器复核用 c4bbdd9f8a 构建的二进制；1870e95895 相对它改了 open_historical_readonly 热日志分支的报错文案、一处注释和测试。判断这是否影响「终版二进制复核」的代表性。

## 约束

本机禁止 cargo 与任何重型验证。确需编译或跑测试时只能走 ~/.local/bin/nc-remote（先看 --help），仅在确有必要时用。不打印任何令牌或密钥。

## 产出（写在 /Volumes/AgentStorage/issue1405-closegap-control-20261020/review-0001/）

- NOTES.md：进度笔记（重启时据此续做）。
- REVIEW.md：开头写评审者身份（模型、非作者）、评审对象（5baeb030a6..1870e95895 全量，基线 main b63a3fd586）、方法与实际执行的命令；按 BLOCK / WARN / INFO 分级列发现，每条带 文件:行号 证据。最后一行严格为 `VERDICT: PASS` 或 `VERDICT: BLOCK`（任何必须修才能关票的问题即 BLOCK）。
- RESULT.json：最后写（它的出现表示完成），形如 {"review":"1405-recovery-review-0001","checkout_sha":"</Volumes/AgentStorage/issue1405-review-ro-20261020 的 HEAD>","range":"5baeb030a6..1870e95895","verdict":"PASS|BLOCK","findings":{"BLOCK":[],"WARN":[],"INFO":[]}}，每条 finding 含 id、title、locations。
