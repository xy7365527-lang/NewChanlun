# #552 实装收尾 · Spec 轴评审（2026-07-28）

评审面 `git diff 8096600a35...HEAD`（502 行纯新增）。规格源 = #552 票体 4 条验收 + SPEC #547 US10/US11/US15/US16/US18 + #246 + audit §6。新上下文，只读，未改代码。

## 结论：**PASS**（3 条登记 + 1 条声明收窄，无阻断性规格缺口）

独立复跑 `cargo test --lib cand_sub` = **7 passed / 0 failed**（非引用报告数）。

## 已核对通过

| 规格原文 | 落点 |
|---|---|
| 「子级事件 interval ⊆ 父级事件 interval」「跨级分支」 | `cand_sub.rs:31-33`，`child.event_level < parent.event_level && interval_is_sub(...)` |
| 「闭区间含端点，相切算包含」（#246） | `cand_event.rs:146-148` `parent.0<=child.0 && child.1<=parent.1` ⟹ 端点相等成立；真值表 `cand_sub.rs:267-273` 三格锁 |
| 「source_index 坐标域」（US11） | `cand_event.rs:124` 字段注 + `mod.rs:293-294` 级别-N 经 source_index 映射同坐标系 |
| 「不复读 nest.rs is_sub」（US18／audit §6） | `cand_sub.rs` 无 `nest::` 调用；`nest.rs:67` 是 `(start_time,end_time)` 时间域，确属另一身份 |
| 「真值表：相切／相离／严格包含／反向不包含／同区间」 | `cand_sub.rs:264-317` 双轴 20 格，五类全覆盖 |
| 「真实事件流相邻级只读探针 + 覆盖计数」 | `cand_sub.rs:135-207` + `issue550_event_battery.rs` 探针节，同源直调谓词 |
| 「零生产行为变更」 | 零删除零改行；lib 外唯一引用 = 诊断 bin |

## 发现

**1｜(c) 声明范围过宽（中）** — `cand_sub.rs:11-13` 模块头写「本模块的区间不等式**唯一**委托上游单一来源…不在此重写第二套不等式」，报告 §一行 1/2、§二.2 同调。实际探针内有两处**新写**的区间判据：`cand_sub.rs:169` 相切等式（即 #246 口径的第二处编码）、`cand_sub.rs:175` 相离不等式。谓词 `candidate_is_sub` 本身确为零重写，探针不是。US18「只共享更上游单一来源」是本票核心纪律，声明应收窄为「**谓词的包含不等式**唯一委托」。行为层无害（只读、不进生产），属 090 声明层问题。

**2｜(a) 部分：票体验收 ④ 后半「#544 随票闭环（评论核销）」** — 报告 §一 #10／§八6 自陈未做。实测 #544 已 CLOSED，末条评论已写「核销：实装票已出笼 = #552…本票内容被其完整覆盖」——核销在开票时已完成，残留仅「实装落地」回执。判为**部分满足**，非阻断。

**3｜(b) 受控 scope 扩展（低）** — 票体只要求「相邻级」探针；`SameLevelBlock`（`cand_sub.rs:63-84`、`188-207`）扫**同级**对，字面在范围外。它服务 US16 防真空且是级别分支非装饰的主证据（报告 §八1 自陈），建议保留、登记即可。

**4｜(c) 恒等式登记不齐（低）** — `cand_sub.rs:179` 中 `!candidate_is_sub(parent, child)` 在该调用点恒真（`parent_level == child_level+1`），故 `reverse_blocked_by_level` 等价于裸 `interval_is_sub(parent.interval, child.interval)`。这与报告 §八7 已登记的 `same_level_blocked == interval_ok` 是同一结构，却未同等登记；`cand_sub.rs:57-59` 仍称其「防真空绿的直接证据」。结论不假（确证级别分支改变结果），但 090 要求同类恒等式同等登记。

## 非缺口备注

#544 提到的「全量/增量一致」属 N1（#550/#551）事件流的锁；`cand_sub` 是流上的纯函数，无独立增量态，不构成 #552 缺口。
