# P0 裁决：`cand_delta` 命名收窄 + 双时点口径（2026-07-12）

- 状态：**ADOPTED-DEFAULT**（任务 #49；可由 P0 后续显式改判推翻）
- 消费材料：`chanlun/review-results/p48-cand-delta-qualification-ruling-material-20260712.md`（任务 #48，只读研究）
- 数据权威：`p46-third-closure-zero-20260712.md`（31/0/11/20 分桶）
- 实现状态：#47（`B_p/c_p` 生命周期 + 双时点字段）当前在工作树，未合并；本裁决含 #47-GATED 条目。
- 分支 / HEAD：`p0-replay-dparent` / `99ce554b8a`。

## 裁决正文（采纳 p48 §8 草案 R1–R8，逐条生效状态如下）

### R1：对象与命名 — ADOPTED

`CandDeltaEvent.cand_delta=true` 裁为 **[新缠论] 算法背驰确认事件**（在
`divergence_confirm_src` 成立），不等价于第 37 课完整趋势 `c` 资格。

旧表述 "`Cand^δ = 趋势背驰段谓词/趋势背驰确认`" **SUPERSEDED-IN-PART**：
- 保留：算法真值、与 `buy1/sell1` 逐 bit 同源事实；
- 撤销：向"完整趋势背驰"的资格外推。

仅改报告与文档语义，不改 P1 bit、不改 strict chain 事件集、不改交易行为
（任何改真值方案按 p48 §7 FULL-RERUN 门另案处理）。

### R2：双时点 — ADOPTED（#47-GATED 落地）

`divergence_confirm_src` 与 `cp_certificate_confirm_src` 为两个不可互换时点。
消费者必须显式选择确认快照或终态对象；禁止未来信息回填。
硬门（p48 §4）：snapshot 不读终态对象；terminal 只经稳定边；历史事件不回填；
`cp_certificate_confirm_src == third.point_source_index`。

### R3：完整趋势资格 — ADOPTED（定义生效；POST-#47 实装）

```text
FullTrendCQualified(p) :=
    TrendContext(p)
AND ThirdClassInsideC(p, B_p)          // 第18行
AND NewExtremeInDirection(p)           // 第20行
AND InternalSublevelCenters(c_p) >= 2  // 第22行
AND CompletedTrendDecomposition(c_p)
```

#47 `Closed` 仅证明 `ThirdClassInsideC` 生命周期闭合，报告中应写作
`ThirdCertificateClosed`，不得写作 `FullTrendQualified`。

### R4：31 例登记 — ADOPTED

- 确认时：31 算法事件，0 third 快照闭合；
- 终点：11 `ThirdCertificateClosed`，20 `Pending`；
- 11 例仍待第 20/22 行与完成分解证明，不自动抬升；
- 20 例仅进入 `ConsolidationReviewPending`，**不得自动降类为盘整背驰**
  （分流裁决按 p48 §7 SEPARATE-RULING 另立 P0）。

### R5：因果与回测 — ADOPTED

早期策略只用 `divergence_confirm_src` 当时快照；晚到资格策略最早在相应证书
确认后行动；终态标签不回填历史交易；strict runner consumer 保持旁路 sidecar，
接入交易须新建因果回测基线。

### R6：继承边界 — ADOPTED

686（fallback 方向禁令）、#37（确认时点≢结构端点）、#43（episode≢完整 `c_p`）
完整保留。盘整背驰继续不入 strict chain，除非 P0 另案显式 supersede。

### R7：机器验收 — 生效为验收门

1. #47 合入后在目标分支复现 `31/0/11/20` 与 `L1..L4 = 1/6/3/1`（closed 分布）；
2. POST-#47：第 20 行新高/新低证书、第 22 行两个次级别中枢确定性 ID 链；
3. 报告分列 event-time / third-closed / full-qualified / classification-review；
4. 改 `cand_delta` 真值或交易消费须重跑无前视回测；
5. 20 例分类分流另立 P0。

### R8：权限 — ADOPTED-DEFAULT 性质声明

本裁决为 adopted-default：即时生效以解除下游阻塞；P0 可显式改判，改判时按
谱系惯例登记 supersede 链。

## GATED 清单（核销条件）

- [x] #47 完成并合入 `p0-replay-dparent`（commit `ba32291eff`，2026-07-12）；
- [x] 复现 R7-1 数字（P47 §3：31/0/11/20，L1..L4 closed = 1/6/3/1，全量 4,613,599 bar）；
- [x] 消费者迁移到显式 snapshot/terminal 选择（任务 #50；strict-nest 与 runner sidecar 均显式选 terminal，事件读取显式选 snapshot）。
