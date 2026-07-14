# bughunt phase2 对抗核验报告（20260710）

- 工位：#40 bughunt 蜂群 phase2 对抗核验（只读）
- 输入：`chanlun/review-results/bughunt-parser-20260710.md`（12 项）、`bughunt-cert-20260710.md`（16 项）、`bughunt-backtest-20260710.md`（11 项），共 39 项编号 finding
- 核验基线：报告宣称快照 **bf1854d71960**（分支 `gap3-rework-codex9-fix` 上）。工作树 HEAD = main = `b3aceb19b6`（bf 的祖先，被审文件大多不在工作树）；分支 tip = `3ebc4e493c`（bf 之后两提交：`3ebc4e493c` 普通单 open 成交、`4f94b3fdc2` goal 脚本归档 + strict_nest_check 反事实诊断 +118 行不改硬门）。所有代码引用行号除特别注明外均为 **bf 快照行号**（`git show bf1854d71960:<path>`）。
- 总口径：**39/39 CONFIRMED（bf 基线），0 REFUTED，0 STALE@bf**。tip 残留面另见 REFUTED/STALE 节。

---

## 核验总表（finding × 判定 × 一句话依据）

### parser 路（bughunt-parser-20260710.md，12/12 CONFIRMED）

| # | 判定 | 一句话依据 |
|---|---|---|
| BUG-01 (P0) | CONFIRMED | `rust/src/theta_v0/parse/rmove_compose.rs:144-170` 的 `retrace_no_break` 对紧随 i1 的向上段几何恒真（m2.lo()≈m1.lo()），B2 恒在 i2=i1+1 提前发出；witness 测试 :286-303 把错误冻结成断言（i2=1, second_point=-8）；生产真接线 `mod.rs:335→:1949→signal.rs:655`；第21课原文（"第一买点后的第二段次级别走势低点构成第二类买点"）核对属实。独立反例：subs=[m1 Down [-10,-2] 破中枢背驰, m2 Up [-8,3], m3 Down [-5,3]]，正确 B2 应为 m3 低点 -5，代码输出 {i1:0, i2:1, second_point:-8}。 |
| BUG-02 (P1) | CONFIRMED | `rmove_compose.rs:106-111` + `signal.rs:674-675` + `recursive_tower.rs:666-672`：外包络价格绑 end_index，B2 价格/索引可指向非最低点段。 |
| BUG-03 (P1) | CONFIRMED | `recursive_tower.rs:666-672`、`mod.rs:2038` 用结构相等首匹配冒充身份，:663-665 注释自认；重复结构段会错配。 |
| BUG-04 (P1) | CONFIRMED | `mod.rs:453` 守卫 `==n` vs :913/:959 写入恒为 n-1 → n≥2 时增量缓存是死代码，每 bar 全量重算；纯性能，不触判据。 |
| BUG-05 (P1) | CONFIRMED（定级偏高） | `rust/src/theta_v0/cert/nest.rs:124-128,131-133` 证书字段可伪造；但生产路径未必可达伪造态，属 hardening；修复=改 P2 证书判据定义，须裁定。 |
| BUG-06 (P1) | CONFIRMED | bf:1176（phase1 报告 1267-1269 为 tip 行号，已重定位）`cert_total<=9 && cert_missing_terminal==0`；≤9 上界门本身是 STRICT-NEST-CHECK.md 冻结口径（md:52 "不许放宽凑产量"）→ 修复=改冻结硬门。 |
| BUG-07 (P2) | CONFIRMED | `strict_nest_check.rs` bf:982 默认 checkpoint=1000、:498-531 只累积 bsp 侧、:1067-1077 终态定判 → <1000 bar 的瞬态分叉不可见；修复触 P1 硬门协议。 |
| BUG-08 (P2) | CONFIRMED | `segment.rs:148-161` 外包络合并 vs `feature_seq.rs:112-141` 方向性正解并存；主链走 feature_seq（segment.rs:439/512/674 调用），仅 second_kind.rs 平行路径踩坑，P2 恰当。 |
| BUG-09 (P2) | CONFIRMED | `divergence.rs:215-221` `closes[0]` 无空检；纯边界 panic。 |
| BUG-10 (P2) | CONFIRMED（as coded） | `divergence.rs:530,540,553` 除零/NaN 路径存在，但实务可达性≈0（需 MACD 面积恰为 0）。 |
| BUG-11 (P2) | CONFIRMED | `force_conformance.rs:122-127` 背驰比较仅 `area_a<=area_b+quantum`、:75 仅 debug_assert；不在任何硬门内。 |
| BUG-12 (P2) | CONFIRMED | `signal.rs:788-806`（:797 先取 a[i] 后校验）、`recursive_tower.rs:763-782`：越界为安全 panic 非 UB，生产不可达。 |

### cert 路（bughunt-cert-20260710.md，16/16 CONFIRMED）

| # | 判定 | 一句话依据 |
|---|---|---|
| F-01 | CONFIRMED | `nest.rs:151-156,164-173,201-219` 证书字段全 pub，`econ_positive.rs:969-974` 字面量直构 NestCertificate 绕过 `assemble_certificate` 的三道门（nest.rs:277/328/333）；修复属实现层自由度，但须保住裁决①的 Cand^δ 定义。 |
| F-02 | CONFIRMED | `recursive_tower.rs:798-801,821-833,851-853` + `decompose.rs:159-168,213-226`：趋势门与 Consolidation 要求互斥，盘整背驰诊断生产不可达——裁决②"保留诊断"被架空。附加发现：phase1 引用的 `chanlun/escalate/strict-nesting-rulings-20260708.md` 在 bf 快照不存在（`4f94b3fdc2` 才首次入库），其宣称快照下引用不可解析（取证瑕疵，结论不受影响）。 |
| F-03 | CONFIRMED | strict_nest_check bf:970-976,1334,1485：MAX_BARS 截断仅 eprintln，overall 判定无 full_replay 项，截断跑可绿灯；修复=给硬门加项（加严方向），仍触 STRICT-NEST-CHECK 冻结协议。 |
| F-04 | CONFIRMED | bf:341 `as i8` 先截断、:351 才查 ±1 → 输入 255 变 -1 通过校验；纯加严修复。 |
| F-05 | CONFIRMED | json_raw 提取 bf:310-321、array_body bf:1593-1607 手写解析对嵌套/转义脆弱，双向反例成立（漏配/误配）。 |
| F-06 | CONFIRMED | `runner.rs:199-207` + strict_nest_check bf:1145-1153 按 top 级别重复计数嵌套证书；当前计数为 0 故潜伏；sidecar 诊断不触判据。 |
| F-07 | CONFIRMED | bf:1350-1354,1405-1417 强制 `cand_candidates>0`，None 分支空转不告警；诊断层。 |
| F-08 | CONFIRMED | bf:981 `env::var("STRICT_NEST_P1_PERBAR").is_ok()`（设 =0 仍启用）、:998 PROFILE 同病、:970-973 MAX_BARS `parse().ok()` 静默回退默认值——fail-open env 解析；触硬门脚本但纯加严。 |
| F-09 | CONFIRMED | `scripts/goal_events.py:404-434,577-613` append_event 全程无锁（读→幂等查→:584 二次 `_load_events` 引用完整性→:433-434 append），并发 TOCTOU 双写；tip 已删三脚本（-2884 行）但 `.chanlun/archive/goals-ceremony-20260707/scripts/` 归档副本仍带病。 |
| F-10 | CONFIRMED | `goal_events.py:438-452` 坏行宽容 continue + `ceremony_scan.py:57-66,81-87,1744-1754` clean_terminate 五条件不含 skipped_event_lines → 损坏事件被静默吞掉且不阻断收尾。 |
| F-11 | CONFIRMED | `goal_events.py:348-351` 仅 `in (None,"")` 非类型校验，goal_id=[] 过关 → reducer :146-149 TypeError（unhashable）事后炸。 |
| F-12（高危，亲验） | CONFIRMED | 详见高危项详审。 |
| F-13 | CONFIRMED | `goal_events.py:160-176` 依赖边无唯一性/自环检查；reducer :211-216 最后写者覆盖。 |
| F-14 | CONFIRMED | `ceremony_scan.py:800-830` 先 append 后 `"w"` 整文件截断重写，无锁/无临时文件/无 fsync，:829 吞错；附加：:824 `content.replace("status: pending",…,1)` 文本替换可错位到非目标条目。 |
| F-15 | CONFIRMED（用词清单需修正） | `ceremony_scan.py:22-23,547-554` 终态枚举不全机制成立；但实测漏网大户是状态列"已结算"×91 行（不在枚举、无✅），phase1 举的 completed/closed 多出现在正文非状态列。 |
| F-16 | CONFIRMED | `ceremony_scan.py:1470` `nargs="*"` + :1491 truthiness → 裸 `--workstations` 得 []，falsy 走全量扫描而非空集。 |
| 裁决①②复核 | 三点全成立 | ① nest.rs:277/:328 强制 cand_delta 与裁决①一致；② pan_div_diag 装配器不消费（"不入链"成立，但"保留诊断"被 F-02 掏空）；③ nest.rs:147-149,154,197-198 过期文档注释与 :224-232 正式定义冲突。 |

### backtest 路（bughunt-backtest-20260710.md，11/11 CONFIRMED）

| # | 判定 | 一句话依据 |
|---|---|---|
| F-01 (P0) | CONFIRMED | `rust/src/theta_v0/backtest/runner.rs:240-257,330-334,2486-2492` 旧入口前视；措辞修正：非"按 source_index 回放"，而是 signal_index→fill_bar_index 延迟成交但信号本身用了确认后段（前视本质相同）；调用方 `l3_fullwindow.rs:38/120` 正在用它产出 L3 结论；tip 未修。 |
| F-02 (P0) | CONFIRMED（tip 部分已修） | bf 上普通单/止损单均 close 当根成交，违背冻结执行协议（下一可交易 bar open）；tip `3ebc4e493c` 已修普通单（`regular_fill_price` 四处：tip 1047/2540/2582/2849，untradable/open≤0 不成交）；**tip 仍残留**：①止损仍走普通延迟 Close 队列（`exit.rs:130`，tip runner 全文无 stop_fill_price）②apply_fees tick 舍入仍被 f64 连续比例绕过 ③与 capture_oos 三口径分歧仍在 ④MtM/prices 仍全 close 口径。 |
| F-03 (P0) | CONFIRMED | `runner.rs:1646/2688/2855` `daily_returns=bar_returns`（fn :3009 无日期聚合）+ `metrics.rs:700-710` ANN=252 → BTC 1min Sharpe 量纲错 sqrt(365.25×24×60/252)≈45.685 倍；tip 未修；若 prereg 判据含 Sharpe 阈值则触 prereg。 |
| F-04 (P1) | CONFIRMED | `runner.rs:1549,1566-1590`（mtm 路径 :2659-2681 同构）期末 MtM 不含退出费：MtM 9.970 vs 强平 9.937，漏 0.033；触"浮盈算上"铁律与推导链第11条；若改真实强平将波及 TW Realize 冻结量化 → 须裁定。 |
| F-05 (P1，高危，亲验) | CONFIRMED | 详见高危项详审（现金不足 fill 被拒仍计执行）。 |
| F-06 (P1) | CONFIRMED | `runner.rs:2716-2750` 仅持仓 sign 变化才产 TradeRecord vs `apply_order` :2950-2976 逐 fill push trade_pnls → 部分减仓 PnL 无对应 trade；反例 100买10→110减5→120平5（fee 3e-4）：真实 149.355 vs 按 TradeRecord 重算 99.670，漏 49.685；`coverage.rs:2648` 是设计说明非 bug 本体；修复动摇 L2 显著性结论。 |
| F-07 (P1) | CONFIRMED（附限定） | `treasury.rs:68` `.floor() as i64` vs `runner.rs:1111` `as i64`（trunc）：-0.7 → -1 vs 0；两笔 +0.6 逐笔 floor 累计 0 vs 汇总 1；限定：bf 上 `settle_all` 生产调用点为零（仅 :104 自测），分叉现属 API 级；runner cast≠floor 需负累计才兑现；**直接触 TW/Realize 冻结量化规则（清单⑥）**。 |
| F-08 (P0 级高危，亲验) | CONFIRMED | 详见高危项详审（训练 λ 越过 train_end）。 |
| F-09 (P1，高危，亲验) | CONFIRMED | 详见高危项详审（跨窗持仓延续）。 |
| F-10 (P1，高危，亲验) | CONFIRMED | 详见高危项详审（不可交易 bar 成交）。 |
| F-11 (P2) | CONFIRMED（P2 维持） | `wverify_run.rs:909-914`（`day.min(28)`）、:918-927、测试 :1521-1527 冻结 `q4_prev_day("2023-01-01")=="2022-12-28"`（正确应为 2022-12-31）；:917 注释自认有意钳位；方向为收缩训练窗（安全侧，无前视）；修复触 prereg 窗口冻结，须走 prereg 修订流程。 |

---

## 高危项详审（本体亲验，独立触发路径）

### 1. backtest F-05：现金不足 fill 被静默拒绝，但仍计入执行

- 证据链（均 bf `runner.rs`）：
  - `apply_fill` 定义 :2925；拒单 :2988-2990：`if need_cash && *cash < cost { return (realized, fee_paid); }` —— 返回值 `(0.0, fee)` 与成功开仓完全同形，调用方无法区分。
  - 调用点 :1052-1057 与 :2590-2607：无条件 `n_orders_executed += 1`，并照常更新 `voice_qty`/`held`（:2599-2605）。
- 独立触发路径：cash=100, px=100, fee_rate=0.0003, qty=1 → cost=100.03>100 → apply_fill 空转，但 n_orders_executed=1、voice_qty=1。
- 后果：账本认为持仓存在而现金账户从未出钱；`is_l2 = n_orders>0` 类判据可在**零真实成交**时为真。
- tip 状态：`3ebc4e493c` 的 regular_fill_price 未触及拒单路径，**tip 仍在**。
- 判定：CONFIRMED，phase1 定级（P1）成立，且因污染 L2 可达性口径建议提为首批修复。

### 2. backtest F-08：训练期 λ 标定读取越过 train_end 的数据

- 证据链（bf `capture_oos.rs`）：
  - :155 `fin = parse_layer(&ds.bars, &cfg)` —— 对**全窗**做一次终态解析，段身份本身含确认前视。
  - :159-168 `seg_gap` 特征读取 `ds.bars[s.start_index..=s.end_index]` 的完整路径。
  - :171-175 训练段过滤仅检查 `day(fin.segments[si].start_index) ∈ [train_start, train_end]` —— 一个 start_index 在训练期内的段，其 end_index 可以在 train_end 之后，λ 标定照读其后路径 bar。
- 判定：CONFIRMED。双重前视：段的存在性来自全窗终态解析 + 特征读取越界。OOS "out-of-sample" 声明在此实现下不成立。

### 3. backtest F-09：跨窗持仓延续

- 证据链（bf `capture_oos.rs`）：
  - :193-196 单一 `ParseLayerIncr` 与单一 `open` 持仓状态贯穿全部历史，无任何窗界重置。
  - :251-258 入场时冻结 `win`/λ，之后不再看 test_end —— 持仓可跨过窗边界继续存活。
  - :289-298 结果按 **entry 所在窗**聚合 —— 窗 k 的收益可含窗 k+1 时段的价格路径。
- 判定：CONFIRMED。各窗结果不独立，窗级统计（含显著性）被跨窗路径污染。

### 4. backtest F-10：不可交易 bar 上照常成交 + 止损无跳空处理

- 证据链（bf `capture_oos.rs`）：
  - :214,239,260 `untradable_fills += bar.untradable as usize` —— untradable 仅计数，**不阻止成交**。
  - :219 `exit: stop_px` —— 止损按理论价成交，无"跳空取劣侧"（冻结执行协议明文要求）。
  - :244,255 进出场均 `bar.close` 当根成交，与 runner tip 已修的 open 口径形成三口径分歧（runner-bf close / runner-tip open / capture_oos close）。
- 判定：CONFIRMED。三条子问题独立成立。

### 5. cert F-12：goal ID 同秒碰撞

- 证据链：bf `scripts/goal_events.py:649-656` `_slug_goal_id` = `g-<UTC秒级时间戳>-<sha256(desc)[:8]>`。
- 独立触发路径（纯函数复现，不写任何文件）：同一秒内以相同描述调用两次（不同微秒），两次均得 `g-20260710T190113Z-47b998b4` —— 碰撞确认。
- 细化定级：**串行**场景下第二次创建会被 :420-425 的同类型同 goal_id 唯一性检查 raise 拦截（fail-loud）；要产生**静默**双写需叠加 F-09 的无锁 TOCTOU 并发窗口。故维持 phase1 的"中危"定级，不升级。
- tip 状态：脚本已随 `4f94b3fdc2` 移入 `.chanlun/archive/goals-ceremony-20260707/scripts/`，活动路径已无此代码；归档副本仍带病。

---

## REFUTED / STALE 明细

**REFUTED：0 条。** 39 条 finding 在 bf 快照上全部有真实代码依据，无一被反证推翻。

**STALE@bf：0 条。** 所有 finding 在其宣称快照 bf1854d71960 上均可复现。

**tip（3ebc4e493c）残留面清单**（不改变 bf 判定，供 phase3 参考）：

| 项 | tip 状态 |
|---|---|
| backtest F-02 普通单 close 当根成交 | **已修**（`3ebc4e493c` regular_fill_price，open 成交 + untradable/open≤0 不成交 + close 降级 MtM） |
| backtest F-02 止损当根成交 / apply_fees tick 舍入绕过 / 三口径分歧 / MtM close 口径 | **仍在** |
| backtest F-01/03/04/05/06/07、capture_oos F-08/09/10 | **仍在**（tip 两提交均未触及） |
| parser BUG-01..12 | **全部仍在**（`4f94b3fdc2` 对 strict_nest_check 仅加反事实诊断，不改硬门与 parse 层） |
| cert F-01..08 | **仍在**（F-03/06/07/08 的行号在 tip 有漂移：`4f94b3fdc2` 在 763 后 +81、1223 后 +12） |
| cert F-09..16 + F-12（goal/ceremony 脚本类） | **活动路径已删**（`4f94b3fdc2` -2884 行，脚本移入 `.chanlun/archive/goals-ceremony-20260707/scripts/`）；归档副本仍带病。ceremony 协议已于 2026-07-07 整体停用归档，此组 finding 实际风险≈0，除非协议复活。 |

**phase1 报告取证瑕疵（两处，均不影响结论，phase3 引用时须换算）**：
1. cert 报告中所有 `strict_nest_check.rs` 行号实为 **tip 行号**而非其宣称的 bf 快照行号（本报告已全部重定位回 bf）；parser BUG-06 同病（报告 1267-1269 → bf 1176）。
2. cert F-02 引用的裁决文件 `chanlun/escalate/strict-nesting-rulings-20260708.md` 在 bf 快照下不存在，`4f94b3fdc2` 才首次入库——phase1 实际是在 tip 状态下取证。

---

## phase3 修复排序与冻结判据触碰清单

### A. 触碰冻结判据 —— 单列，phase3 不得自动修，须先裁定/报备

| 项 | 触碰的冻结面 | 说明 |
|---|---|---|
| parser BUG-06 | STRICT-NEST-CHECK.md 硬门（cert_total≤9 上界，md:52） | 任何改动=改冻结硬门本身 |
| parser BUG-07 | P1 硬门"逐 bar"协议语义 | checkpoint 采样 vs 逐 bar 声明的口径裁定 |
| parser BUG-05 | P2 证书判据定义 | 伪造防护=收紧证书构造语义，须裁定 |
| cert F-03 | STRICT-NEST-CHECK 硬门（加严方向） | 给 overall 增加 full_replay 项仍是改硬门，须报备 |
| backtest F-04 | "浮盈算上"铁律 + 推导链第11条；波及 TW Realize 冻结量化 | 期末 MtM vs 真实强平口径须裁定 |
| backtest F-07 | TW/Realize 冻结量化规则（清单⑥） | floor vs trunc 的负数语义统一须裁定 |
| backtest F-11 | prereg 窗口冻结（q4 切分 + 冻结测试值 wverify_run.rs:1521-1527） | 须走 prereg 修订流程，不得直改 |
| backtest F-03 | prereg（若判据含 Sharpe 阈值） | 年化修正本身机械，但会改所有历史 Sharpe 数值 |
| parser BUG-01/02/03 | bit-exact 对拍基线 + witness 测试（rmove_compose.rs:286-303）+ 潜在 E1 N_B_EXPECTED=1 | 修复不改判据定义但改判据**数值**：须同步重写 witness、重建基线，并先核 E1 是否消费二类事件计数 |
| cert F-09..16 | 已归档 goals ceremony 协议 | 协议已停用，建议**不修**（死代码）；若复活协议，先修 F-09/F-10/F-14 |

### B. 修复优先级排序（不触冻结定义，可直修；括号=同步义务）

1. **backtest F-05** — 现金不足拒单须返回可区分的 FillOutcome，拒单不计 n_orders_executed/voice_qty。污染 L2 可达性口径，代价最高。
2. **backtest F-02 残留** — 止损接入 open/跳空劣侧成交 + apply_fees 统一 tick 口径（向已冻结的执行协议**收敛**，有 tip open-fill 先例）（同步：重建 bit-exact 基线）。
3. **capture_oos 三连 F-08/F-09/F-10** — 训练段特征截断至 train_end、窗界平仓重置、untradable 阻止成交 + 止损劣侧。OOS 结论在修复前不可引用。
4. **backtest F-03** — bar_returns→日聚合或按 bar 频率年化（若 prereg 含 Sharpe 阈值转入 A 列）。
5. **backtest F-01** — 最小修：旧入口标记 deprecated 并禁止产出 L2/L3 声明；l3_fullwindow.rs:38/120 迁移到无前视入口（同步：既有 L3 结论作废重跑）。
6. **backtest F-06** — TradeRecord 逐 fill 生成或明确声明其为"往返"口径（同步：L2 显著性重算）。
7. **parser BUG-01（+BUG-02 耦合）** — retrace_no_break 修正为"i1 后第二段次级别回抽低点"（第21课）；因触 A 列 witness/基线面，实施前走裁定，但优先级应视同 P0。
8. **parser BUG-03** — 结构相等→身份索引。
9. **cert F-01/F-02** — 证书构造收口（builder/私有字段）+ 盘整背驰诊断可达性修复（守裁决②"不入链"边界）。
10. **parser BUG-04** — 缓存守卫 off-by-one，纯性能，低风险高收益（解锁逐 bar 硬门的可行性）。
11. **cert F-04/F-05/F-08** — 脚本输入加严（i8 截断、手写 JSON 解析、fail-open env）；**cert F-06/F-07** — 诊断层修正。
12. **parser BUG-09/BUG-11/BUG-12、cert F-11/F-13** — 边界加固。
13. **parser BUG-08/BUG-10、backtest F-07（进 A 列裁定后）、cert F-15/F-16** — 低危收尾。

---

## 核验方法与局限

**方法**：
- 快照锚定：三份 phase1 报告宣称快照 bf1854d71960；工作树为 main（bf 祖先），被审文件多不在工作树 → 全部代码核验通过 `git show bf1854d71960:<path> | nl -ba` 只读完成；tip 残留面用 `git show 3ebc4e493ca7:<path>` 双查。三方关系：main(b3aceb19b6) ⊂ bf(bf1854d71960) ⊂ tip(3ebc4e493c)，以 `git merge-base --is-ancestor` 验证。
- 判定标准：CONFIRMED 须给出独立最小触发路径或独立反例（非复述 phase1）；行号漂移一律重定位并注明。
- 高危四组（capture_oos 三连、现金不足 fill、goal ID 碰撞）由本体亲验；goal ID 碰撞用纯函数复现（同秒同描述 → `g-20260710T190113Z-47b998b4`），未写入任何事件文件。
- 派发：三个 general-purpose 子代理并行分审 parser/cert/backtest 三路，只读约束写入 prompt；登记见 `.chanlun/agent-roster-2026-07-10.md`。

**局限**：
- 未执行 strict_nest_check 全量重放与 cargo test（被审代码不在工作树 + 只读约束），核验以静态代码证据 + 纯函数复现为主；BUG-01 反例是按代码逻辑手推的结构反例，未在编译产物上运行。
- BUG-05（证书伪造）与 BUG-10（除零）的"生产可达性"判断基于调用图静态分析，未做穷尽路径枚举。
- backtest F-07 的定级依赖"settle_all 生产调用点为零"这一 bf 时点事实，后续接线后需重估。
- cert F-15 的量化（"已结算"×91 行）来自对当前工作树 ledger 的一次统计，随 ledger 演化会变。
- phase1 两处取证瑕疵（tip 行号、裁决文件 bf 下不存在）说明 phase1 实际在 tip 态取证；本报告已全部换算回 bf，phase3 引用行号时以本报告为准。
