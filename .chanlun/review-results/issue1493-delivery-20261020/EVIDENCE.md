# #1493 交付证据清单（合入前，2026-10-20）

名分：工作草稿（宪法 §5），活期绑定 #1493。PR #1582，分支 `codex/1493-segment-turn-20261020`。合入方式：**squash**（`docs/agents/delivery-discipline.md` 关票门第 4 子句，#1587）。

## 关票门逐项（合入前状态；带 ⏳ 的项在合入后才能成立）

| 子句 | 现状 |
|---|---|
| 1 resolution 引用落地 hash | ⏳ squash 合入后取 main 上的 squash 提交 hash |
| 2 已并入 main | ⏳ 待用户批准合入；当前未合入 |
| 3 声明当场为真 | 本清单各项均为已执行结果；关票评论在合入后写 |
| 4 逐 commit 可编译 | squash 成单一提交，其树 = 分支 HEAD 树（含 merge origin/main 后的结果），由 GitHub CI 在 HEAD 上编译与全测；分支上的中间提交不进 main 历史，不另作逐个声明 |
| 5 工位清理 | ⏳ 关票时删：worktree `/Volumes/AgentStorage/issue1493-owner-20261020/wt`、分支 `codex/1493-segment-turn-20261020`；旧分支 `codex/1493-segment-turn-20261014`、`codex/1493-segment-turn-resume-20261018` 按 #1493 2026-10-20 接手评论不作基线，一并核残骸 |
| 6 报告入仓 | 本目录与 `.chanlun/review-results/issue1493-tb02x-head-20261020/` |
| 7 评审面 == 关票面 | R1（36adc7c7c8..17c7e24e08）、R2（17c7e24e08..afd7eadf37）、尾部 TAIL（afd7eadf37..合并 origin/main 后的 HEAD，代码面仅 `l3_delta_r_alpha.rs` 新测试与 `tb02x_runtime.py --relax-query`）；TAIL 之后只有收据文字修正（纯文档） |

## 实现与正本

- 裁定 1–4 实现：`rust/src/theta_v0/parser/feature_seq.rs`、`segment.rs`、测试 `tb02c_tests.rs`／`tb02w_tests.rs`／`tb1493_replay_tests.rs`、`backtest/l3_delta_r_alpha.rs`；合同 `s_session/s_tb02_contract.py`、`browser/q3-typed.js`、`q3-consumer.js`、`index.html`、生成包 `tb01c-client.js`。
- 正本：`.chanlun/definitions/xianduan.md` v2.4（§5.1、§5.2、§7.1 第 1/3/4/6/7/8 条、§10、§15 X-33～X-37、§16 V-15/V-17/V-18）。
- 票面回填：义务 1、义务 6 与验收例外；原文备份 `issue-body-before-20261020.md`（本目录）。

## 验证读数

| 项 | 读数 | 收据 |
|---|---|---|
| fmt / lib（私有 target，HEAD `9f3b5281eb`） | FMT 0；2866 过 / 0 败 / 150 ignored | `logs-private-target/20261020-154404-9f3b5281eb.log` |
| 正式会话二进制 | 507 过 / 0 败 | 同上 |
| 全量 4,613,598 根回放 | 51,741 段；最长空窗 13,483；fail-closed 0；段端门失败 0；前缀改写 24 全为末笔可改写；全量 ≡ 增量 | 同上 |
| 尾窗 49,995 根 | 49/100/210/309/414/524；fail-closed 0；改写 0 | 同上 |
| 裁定 2 三方对照 | 随机 802 组段端点 ≠ P：0/0/204；真实数据段端点 = P 全部成立（17,815 段） | 同上；随机组读数见 #1493 评论 6283106225 |
| S 正式会话入口 | turn_up/turn_down：手算神谕 PASS，cut 27/35 页面正确，40 cut 离线全量校验与 Q 审计通过；脚本 FAIL（读超时，并入 #1497） | `../issue1493-tb02x-head-20261020/RECEIPT.md`、`logs-private-target/20261020-162946-9f3b5281eb.log` |
| GitHub CI | `9f3b5281eb` 全 job 通过（run 39386630435）；合并 main 后 HEAD 的 CI 见 PR | PR #1582 |
| 独审 | R1 PASS 附必改（已落实）、R2 PASS、TAIL PASS 附必改（收据数字已改） | `REVIEW-R1.md`、`REVIEW-R2.md`、`REVIEW-TAIL.md` |

`logs-shared-target/` 是 #1488 通报窗口（12:00–15:38 共用 target）内外的首跑日志，仅作对照；结论以私有 target 复跑为准。

## 不在本票

X-37 Rust 下游拒读（#1581）；S 入口读超时（#1497）；内存二次增长与 `Rc::make_mut` 整表复制（#1588）；签名目录补叶（SPEC #1340 待办）；#1419 L3 读数口径（#1419）。
