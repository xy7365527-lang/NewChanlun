# 收据 workers/s2-review-0055/REVIEW.md

- 原件：`workers/s2-review-0055/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`6b8940a2bb87aa2d49c20f59ab4cfe50198812fc8b58fbf51de63ba16a55d4d9`
- 字节数：7909；行数：65

## 关键行（行号: 原文）

```
1: # #1488 S2 MIN12 oracle 非作者独审（0055）
4: - 范围：`.chanlun/review-results/1488-acceptance/min12/`、`tools/min12_oracle_run.py`、`tools/run_wsl.sh`、`tools/semdiff.py`、`tools/r3_offline_churn.py`、`tools/readings.py`；运行原件 WSL `G:/AgentStorage/1488-accept/s2/`
5: - 适用裁定：D2（oracle 单一来源 = main 全量二进制 new-bi/2；差异逐项归类、独立审）；A③ 取 (a)（MIN12 52 根全程 10s 帽内、读数与 new-bi/2 oracle 逐位一致；center 条件移交 #1419）
8: ## Verdict：APPROVED_WITH_FOLLOWUPS
10: oracle 本体（52 代 cut 投影、digest、双跑一致性）**可作为 D2 单一来源采用**。无 P1。
11: P2 均为 REPORT 文字/归类完整性问题，不影响 oracle 字节；建议作者修 REPORT 后入合，无需重跑。
13: ## 逐项核对
15: | # | 检查项 | 结论 | 证据 |
19: | 3 | 驱动无偷懒 | ✅（附 P3） | `exchange` 单发无重发，`validate_reply(producer=S, epoch=1)` 严格校验头/因果，非 Committed 即 break；`query_all` 逐页 snapshot_token 翻页并校验跨页 digest 稳定；pages 合计 2346 == q.log POST 数；驱动与 `s_session/` 相对 HEAD、6badaa6049 无改动 |
20: | 4 | 双跑新进程新库逐字节 | ✅ | 亲自在 G: 重算 SHA256SUMS == G: 清单 == 仓内清单（run-a f52362e3…、run-b b0082668…）；52 个 oracle 文件、STRUCTURAL eb75995a…、session.sqlite 4f185399… 两跑相同；两跑 PID/库/socket/端口皆不同；差异只在 INGEST/INDEX/RUN/config/control/shm/wal/owner（时间戳/路径/PID 类） |
22: | 6 | 与旧 known-15/31 差异归类 | ✅ 结论成立（附 P2/P3） | 独立脚本 `evidence/indep_diff.py`（不剥版本字段、obj/batch/wit 引用解析为 kind/fact_key/revision/lifecycle、空容器也比）→ `evidence/d15.json`、`d31.json`。见下节 |
24: | 8 | 第 46 条台阶 | ⚠️ 旧侧证据成立；main 侧"不复现"过强（P2） | 见下节 |
27: ### 6. known-15/31 独立对照细节
36: ### 8. op-46 台阶细节
39: - 旧 events 在 op-0046 窗口无 bounded_query；q-sidecar 最后活动 mono 262413.9 s，窗口 262469–262478 s，Q 空闲 → 非读侧争用，结论成立。
44: ## Findings
46: ### P2
48: - **F1 REPORT §3 逐项归类不完整。** 未列：CC-055.relation、CC-055.change_event（policy/schema/semantic 全变，不只 causes）、CC-056.version、CC-011.seed_candidate、CC-011.segment_seed、CC-012.feature_sequence、CC-055.segment_change、CC-055.segment_relation 的版本标签变化；fixed_cut 与 catalog 差异未逐项（rule_revision 去 `+center-seed/1`；ca
49: - **F2 REPORT §4 未记录 A③ 裁定，且 (a)/(b) 标签相撞。** 裁定 05:18Z "取 (a)"实质 = REPORT §4 自己的 (b)（center 半移交 #1419），而 REPORT §4 的 (a) 是"先合 #1419"。REPORT 先于裁定入仓。应在 §4 写明裁定原文与映射，免得下游按 REPORT 编号误读成"先合 #1419"（会与 #1488 阻塞 #1419 成环）。
52: ### P3
54: - **F4** REPORT §6 称 r3 在 main 上只出 g1..51"因脚本按增量对计数"——实因 `r3_offline_churn.py` 用 `mode=ro&immutable=1` 打开，忽略未 checkpoint 的 WAL；带 WAL 复跑得 52 代，g46/47 结论不受影响。
57: - **F7** `tools/run_wsl.sh` 的 `set -e` 使 `rc=$?` 捕获失效：python 失败时脚本直接退出，不会拷回 G:、不写 SHA256SUMS。
62: ## 建议
```
