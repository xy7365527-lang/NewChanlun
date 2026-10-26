# 收据 workers/premerge-review-0088/REVIEW.md

- 原件：`workers/premerge-review-0088/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`f84b83a78c9796c21ed59eb9c0f0d439d8894e90bd0544d5903b84b77675b8e9`
- 字节数：14693；行数：163

## 关键行（行号: 原文）

```
1: # #1488 0088：0086 改动与第三次合并面的非作者独审
3: - 评审对象：`026e3e97b0..f0aeaf2817`，共 5 个提交：
12: - 计时提示：WSL 同时承载其他工位的负载（baselines run_prefix、harvest-1604），墙钟读数偏高。不过这不改变 S4 超时的结论，见 P1-1。
14: ## 结论：**CONDITIONAL**
18: - 主要问题：S4 锁在 `s_capture_native_testonly` 门内、没有 ignore，会被 main ci.yml 的 `rust-test-s-session-extras` 经 `test-feature-delta.sh` 以 debug 拉起跑。实测在 3600 秒内跑不完，而这个 job 上限只有 60 分钟。也就是说，合入后 main 的必跑 job 必红（P1-1）。
26: - 合入条件：处理 P1-1 后，复跑 extras 的 TFD 口径，必须在 60 分钟内全绿。P3 不阻塞合入。
28: ## 1 区间提交逐个核
30: | 提交 | 内容 | 结论 |
39: ## 2 再合 main 的合并面（7fbffdc4fb）
54: ## 3 S4 锁并入
59: | 提交 | sha256 | 字节 |
74: ## 4 修红：根因、修法与奖励作弊专查
88: ## 5 独立复跑（target-r88）
107: ## 6 CI yml P3-1 对齐 main #1588
112: ## 7 专节：改断言第十八～二十三处追认核查
121: | 十八 | `native_publication_begin_and_consumers_share_parent_scope`（content_publication.rs:2378，改动在 :2405–2419） | 读数位置未动（begin 返回后），该位置本身已在 begin 临时文本/解析值真实析构之后。原 `begin_held > before+M` 主张的是“保守持有”，与 P1c 相反；现在拆成 `peak() > before+M`（同一阈值，证明曾在父账出生）加 `begin_held == before+M`（析构后精确值） | 是，仅就 begin 临时载荷：`a
122: | 十九 | `b2_owner_codec_cross_scope_preserves_only_success_and_charges_current_parent`（s_content.rs:8130，改动约在 :8156） | 是。在场时 `strong_count == 1` 改为 `> 1`（载荷持许可到析构，P1c 所要求）；“只剩 1”这一读数移到 `drop(capture)` 之后 | 是：drop 后新增 `live == 0` 与 `strong == 1` | 否。在场断言的方向随 P1c 语义翻转，不是放宽原性质；`>1` 不是精确计数（P3-2） |
128: - 依据：P1c 裁定 `control/1488-budget-full-chain-counterexample-fix-v2.md:9`（临时文本、解析、格式化等实际载荷持原来源许可到真实析构，不得 retain 丢许可）。
132: ## Findings
134: ### P1
136: **P1-1：S4 锁会被 main 的 extras job 以 debug 拉起跑，必然超出 60 分钟上限。**
145: - 作者 REPORT 的 delta 读数剔除了 S4，只报告了 uo workflow 下 S4 FAIL(124)，没有报告这个暴露面。
146: - 修法交负责人裁定，可选：
151: ### P2
155: ### P3
```
