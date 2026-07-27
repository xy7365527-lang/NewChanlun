# #473 范围 0：高级别三类点逐级漏斗纯观测探针

日期：2026-07-27  
工位：`/tmp/wt-473f`  
基线：`1d21934c8a470f8e92f39448cfe191f82a0286cc`  
探针提交：`a8156d67edaa96466b4b7fec5e84de19a6f45701`（分支 `ticket-473f`，未 push、未 merge）

## 结论

**[可证，限定于本次 wf8、按 `judge_third` 几何评估次数]：anchor 门是 L≥1 三类几何通过后不能进入证书阶段的主拒绝门。**

- L≥1 共发生 107,700 次相邻 leave/retest 评估；其中 16,040 次三类几何成立。
- 几何成立后，14,122 次仅因 `leave_anchor=None` 被拒，占 **88.04%**；各有数据的高级别分别为 L1 **89.64%**、L2 **85.51%**、L3 **85.60%**，不是单一级别偶发。
- 其余 1,918 次 anchor 与几何方向匹配并全部取得生产证书；`anchor=Some(错向)` 拒绝为 0，观测几何与生产证书不一致为 0。
- 但 anchor 不是唯一损耗：L≥1 最终进入 `push_point` 的新确认三类点只有 48 个，后续为 Broken 12 / Stale 4 / ArenaEmpty(Silent) 32 / MisKill 0；即已产三类点中仍有 **66.67%** 因场空静默。

因此，前序报告的“anchor 不对称是最可能主因”已在**几何 → 证书**这一层闭合为可证；不能据本探针声称“放开 anchor 后必新增 14,122 个唯一 BspPoint”或“所有高级别挂起都会清算”，后两项仍分别受增量重复评估、确认去重及生命周期场状态/归属链约束。

## 探针接线与不干预边界

1. `classifier::thirdclass_probe::with_level` 仅给本级一/三类提取绑定只读级别上下文；不写回 `BspPoint`。
2. `signal::judge_third` 仍只消费原 `judge_third_cert` 返回值。探针在返回值确定后旁路记录：
   - 候选评估；
   - leave anchor 的 Up / Down / None；
   - 去掉 provenance anchor 资格门后的同一方向与价格几何；
   - 原证书是否通过。
3. `fill::step_center_oscillation` 对原 `push_point` 只调用一次，先保存原返回值，再旁路记账，随后沿原分支处理同一返回值。
4. 探针只在 `OPSEM_DUMP_DIR` 非空时启用；最终快照随既有 `OpsemDump` 落到 `thirdclass_funnel.jsonl`。默认路径不写探针文件。
5. OPSEM 独立生命周期消费者不重复计数；生命周期 outcome 只取本次 wf8 开臂实际生产 `push_point` 链。

## 验证实录

### 编译与测试

```text
cargo test --lib
test result: ok. 2097 passed; 0 failed; 141 ignored
```

新增探针单测另行靶向复核：

```text
cargo test --lib thirdclass_probe
test result: ok. 2 passed; 0 failed
```

### wf8 开臂

执行命令：

```bash
export CARGO_TARGET_DIR=/tmp/wt-473f-target
M8_WIN_FILTER=wf8 \
VOICE_EXEC=1 \
THETA_CENTER_OSCILLATION=1 \
OPSEM_DUMP_DIR=/tmp/wt473f_dump \
cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture
```

结果：

```text
test theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos ... ok
test result: ok. 1 passed; 0 failed
wf8: execR=+4040483 MaxDD=0.0917 stage=I(降成本) R=+4417092 LCB(R)=-1921382 → INCONCLUSIVE
```

`INCONCLUSIVE` 是该交易实验原有裁决输出，不是测试失败；本票只读取结构漏斗，不据此改交易结论。

落盘：

- `/tmp/wt473f_dump/thirdclass_funnel.jsonl`：5 行，3,476 bytes，SHA-256 `2fff21885e4ade294999bcdf9c30e012270f0869e876640e6b811c9ee7dd60fe`
- `/tmp/wt473f_dump/center_lifecycle.jsonl`：2,223 行，460,177 bytes，SHA-256 `2765b3e681c1e5de20b312b650f62d9718a42f6fce6422bf95ce7c5aa3613470`

## 逐级漏斗

口径：

- “候选评估”是增量生产链实际调用 `judge_third` 的次数，不是按结构键去重后的唯一对数。
- “几何通过（买/卖）”只去掉 provenance anchor 资格门，其余方向、`ZG/ZD` 严格价格边界与生产判据一致。
- “anchor（上/下/空）”统计全部候选评估；“匹配/空拒/错向拒”只统计几何通过子集。
- “证书（买/卖）”是原生产 `judge_third_cert` 成功次数，仍含增量重复评估。
- “新确认三类/一类”是实际进入生产 `push_point` 的 BspPoint；生命周期四桶之和与相应新确认三类点相等。
- Reset 的 `Some/None` 是 `died_center` 分布；一类点未落 Reset 的部分列在“一类其余”。

| level | 候选评估 | 几何通过（买/卖） | anchor（上/下/空） | 几何后（匹配/空拒/错向拒） | 证书（买/卖） | 新确认三类（买3/卖3） | 三类 outcome（Broken/Stale/Silent/MisKill） | 新确认一类（买1/卖1） | Reset（Some/None） | 一类其余（Stale/Silent/MisKill/Other） |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 162,021 | 44,848（23,150/21,698） | 81,073/80,948/0 | 44,848/0/0 | 23,150/21,698 | 782（429/353） | 377/387/18/0 | 11（5/6） | 4/3 | 4/0/0/0 |
| 1 | 74,505 | 9,783（7,113/2,670） | 2,751/1,290/70,464 | 1,014/8,769/0 | 929/85 | 24（23/1） | 8/4/12/0 | 27（0/27） | 5/13 | 9/0/0/0 |
| 2 | 22,667 | 3,271（2,338/933） | 1,311/549/20,807 | 474/2,797/0 | 307/167 | 18（17/1） | 3/0/15/0 | 0（0/0） | 0/0 | 0/0/0/0 |
| 3 | 10,528 | 2,986（2,986/0） | 430/0/10,098 | 430/2,556/0 | 430/0 | 6（6/0） | 1/0/5/0 | 21（0/21） | 2/10 | 9/0/0/0 |
| 4 | 0 | 0（0/0） | 0/0/0 | 0/0/0 | 0/0 | 0（0/0） | 0/0/0/0 | 0（0/0） | 0/0 | 0/0/0/0 |
| **Σ L≥1** | **107,700** | **16,040（12,437/3,603）** | **4,492/1,839/101,369** | **1,918/14,122/0** | **1,666/252** | **48（46/2）** | **12/4/32/0** | **48（0/48）** | **7/23** | **18/0/0/0** |

逐级守恒检查全部成立：

- 候选评估 = anchor 上 + 下 + 空；
- 几何通过 = 买侧 + 卖侧 = 匹配 + 空拒 + 错向拒；
- 生产证书买 + 卖 = anchor 匹配，镜像不一致为 0；
- 新确认三类 = 买3 + 卖3 = Broken + Stale + Silent + MisKill；
- 新确认一类 = 买1 + 卖1 = Reset Some + Reset None + 一类其余。

与既有 `center_lifecycle.jsonl` 交叉核对也逐级相等：

- Broken：L0/L1/L2/L3/L4 = 377/8/3/1/0；
- third-Stale：387/4/0/0/0；
- Reset：7/18/0/12/0；
- first-Stale：4/9/0/9/0；
- 既有通道不记录 ArenaEmpty(Silent)，本探针补得 18/12/15/5/0；MisKill 全级为 0。

## 对 #473 范围 1–2 的具体建议

### 范围 1：修 anchor 不对称

建议采用 #473 已列的“与一类同级的结构方向授权”，不要给出新的教义级否决：

1. 保留 `judge_third_cert` 的 `Some(Direction)` 契约和方向匹配，不把证书函数改成无方向判定。
2. 在 L≥1 三类调用点，把 leave 方向来源从 provenance `anchors[i - 1]` 收窄改为结构方向 `anchors_self[i - 1]`；L0 两者本来恒等。这样与 p117 一类路径的授权层次一致，同时保留回试方向、`>ZG/<ZD` 严格边界和 OwnerRef。
3. 单测至少锁定：
   - 同一合法几何在 provenance fallback=None 时，经 L≥1 调用层仍能以结构方向产三类；
   - 结构方向与几何方向不符仍拒绝；
   - `retest==ZG/ZD` 仍拒绝；
   - L0 输出逐字段不变，三类 OwnerRef 仍是所破中枢。
4. 开臂先做靶向 counterfactual：按 `(level, CenterId, leave区间, retest区间, side)` 去重比较新增唯一证书与新确认 BspPoint，避免把本报告的 14,122 次重复评估误报成 14,122 个新增点。

预期观测：L≥1 的“几何空拒”应降至 0 或接近 0，几何通过应与证书通过对齐；若新确认 BspPoint 仍不升，下一瓶颈就在确认/去重而非 anchor。

### 范围 2：修挂起绑定中枢归属

不能把 `nearest_confirmed_center` 简单放宽成“扫描所有历史同价格带”。按 #473 用户补充裁定，中枢是含时间轴左右边的四边框，建议：

1. 为第三类归属建立“中枢框 → 紧邻离开/回试”的时间配对：
   - leave 必须发生在该中枢 `end_index`（右边）之后，并是该框对应的那一次次级别离开；
   - retest 必须是 leave 紧随其后的反向回试；
   - 价格只相对该框的 `ZG/ZD` 判定，不能在任意晚时刻重新穿越旧价格带后认领旧中枢。
2. 第三类候选不要只投给 leave 之前“最近中枢”；同时覆盖仍有挂起绑定的原中枢，但每个候选必须先通过上述右边界与紧邻性证书，再写该原中枢的完整 `OwnerRef::Center`。
3. 生命周期路由不要粗暴扩大 `tail_prev` 容读窗。对已滑入历史但仍有挂起绑定的精确 `CenterId`，应有显式、可审计的 historical-bound 命中分支，使其三类终局能清算原绑定；无挂起绑定或时间框不匹配者仍为 Stale，且不得误杀当前 alive 中枢。
4. 必加反例：
   - 两个中枢 `ZD/ZG` 相同但时间框不同，后来的穿越不能认领旧框；
   - 中间已有别的 leave/retest 时，远期相邻对不能回填旧框；
   - 目标滑出 `tail_prev` 但有精确挂起绑定时可达；无绑定时仍 Stale；
   - 同一三类事件只清算其 Owner 中枢，MisKill 维持 0。

范围 2 后再跑同一 wf8 联表：需要同时看新增三类 BspPoint、Broken/Stale/Silent、命中挂起的 CenterId 与跨侧终结；只看 Broken 总数不足以证明归属正确。Reset 终态切换属于 #473 范围 3，本票不提前改。

## 探针局限

1. 候选、几何、证书均是增量链实际评估次数，包含 frontier/dirty-tail 重评；未按结构身份去重，因此只能闭合“哪一道门拒绝了评估”，不能直接预测修复后的唯一点增量。
2. 去 anchor 的几何是当前 `judge_third_cert` 方向与价格条件的观测镜像，单测锁定了 anchor-only 差异；若生产判据后续新增条件，镜像必须同步更新。
3. 只跑了 BTC wf8 一个窗口；结论可用于本窗归因，不能直接外推所有标的和窗口。
4. 生命周期 outcome 只计开臂生产 `push_point`，刻意不计 OPSEM 独立消费者以免双算。
5. L4 本窗无候选，零值只表示无样本，不构成对 anchor 的证据。
