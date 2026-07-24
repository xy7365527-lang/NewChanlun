# trig=1/2/3 → BSP 类别/级别/确认深度 精确映射对账

- 工位：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717）
- 数据：`/tmp/m8_opsem_fixed/trades.jsonl`（518 笔，`trigger_bsp_class_at_exit` ∈ {1,2,3,null}）
- 性质：只读对账（.md 产出），零代码改动、零 git mutation、主仓只读
- 日期：2026-07-19

## 0. 结论摘要

1. **trig 字段的代码语义已逐行锚定**：`trigger_bsp_class_at_exit` = 反向关闭触发候选 g 的**最小成立 BSP 类号**（`min_class`，1<2<3），且 g 与被关腿**严格同级别**（解释器规则2 只关同级腿）。trig 不是「次级别信号」，也不是「区间套深度」读数——关闭谓词 **不消费任何区间套证书**，只在六 bit 上判反向（`reverse_signal`）。
2. **统计观察证实（Short 侧，数字分毫不差）**：Short 侧 trig=1 −4,229.27 + trig=2 −7,413.94 = **−11,643.21**；trig=3 Short = **+3,860.28** ——与「trig=3 +3,860 唯一幸存 / trig=1/2 −11,643 全亏」完全对齐。**该口径是 Short 侧口径**。
3. **统计观察的边界（证伪其全称形式）**：Long 侧 trig=1/2 合计 **+3,320.26**（n=66），「深确认全亏」对 Long 不成立——亏损是 **Short 侧不对称**现象，不是确认深度本身的普遍税。
4. **③号问照实回答**：trig=3 **不是**「次级别最早的反向结构点（区间套最深证书）」——代码链不支持这个读法（关闭规则同级别、不查 nest、dump 不含触发侧 nest 字段）。trig=3 = **同级别三类反向点**（离开中枢 ∧ 回试不入，中枢破坏确认）。trig=1 = 同级别一类反向点（破中枢 ∧ 背驰，**确认链最深**、内禀延迟最大）；trig=2 = 一类后回调结束点（以一类已成立为前提，时序上严格晚于一类）。

## 1. 代码链：trig 字段从哪来（逐行锚）

### 1.1 写入点（唯一来源）

`trigger_bsp_class_at_exit` 只有一个非 null 写入点：

- `rust/src/theta_v0/backtest/runner.rs:1911` — 反向关闭腿的 `exit_type = interp::reverse_exit_type(open.entry_v, trig.bsp_class)`
- `rust/src/theta_v0/backtest/runner.rs:1920-1923` — `dump.write_trade(&pushed, &open, Some(trig.bsp_class))`，`trig` = `step_trace.closed` 里与该腿配对的**关闭触发候选**（`close_triggers`，与本腿一一对应）
- 序列化：`runner.rs:2829-2831`（`opt_u8_str(trigger_bsp_class)`）；入参签名 `runner.rs:2763`

**null 的四条路径**（无触发候选可记，诚实缺席）：
- 静默离场（§13 AncOK 连带剪/Stale prune）：`runner.rs:1958-1961`（`None`）
- 强平 RiskExit（P1 force_flat）：`runner.rs:1989-1992`（`None`）
- P2 overlay 关闭（TW StageII 重叠腿）：`runner.rs:2021-2024`（`None`）
- censored Hold（窗口末未平仓）：`runner.rs:2247`（`None`）

数据中 null 组构成与此逐项对上：187 笔 `via_anc_ok_prune=true` + 1 笔 censored Hold（见 §3.4）。

### 1.2 触发候选怎么产生（解释器规则2）

- `rust/src/theta_v0/strategy/interp.rs:1194-1208`（`interpret_with_close_triggers` fold 规则2）：候选 g 按 ≺_Θ 序消费，**A_t 中存在同级别、未关闭、方向被 g 反向的活动腿** ⟹ 该腿入 𝒟_x，g 作为关闭触发被消费并入 `close_triggers`（与 `buckets.close` 同步 push，一一对应，断言 `interp.rs:1226-1230`）。
- 反向判定 = `reverse_signal(leg.dir, &c.bits)`（`interp.rs:1200`）→ `rust/src/theta_v0/strategy/exec.rs:255-261`：持多遇 `sell1∨sell2∨sell3`、持空遇 `buy1∨buy2∨buy3`。**纯 bit 析取——无 nest 门、无级别下钻、无背驰力度门**。
- **同级别约束**：`interp.rs:1197-1202` 只在 `level_idx[c.level]`（同 level 腿索引）内找关闭对象。⟹ **trig 的级别 = 被关腿的级别 = `certificate.level` = `voice_id.level`**（dump 不另记触发级别，因为恒等）。
- **fold 内类优先级**：≺_Θ 排序键 `theta_key`（`interp.rs:1066-1084`）= (level DESC, **bsp_class 1<2<3**, source_index, …)。同 bar 同级别若一类与三类反向候选并存，**一类先消费关闭**。⟹ trig=3 的行还携带一个隐含证书：**该 bar 该级别不存在一/二类反向候选**（否则关闭会被低类号抢走）。

### 1.3 trig 值 → BSP 类别（`min_class`）

- `rust/src/theta_v0/strategy/interp.rs:352-367`（`min_class`）：按候选自身方向取 (c1,c2,c3) 三 bit，**最小成立类号**：c1→1，否 c2→2，否 c3→3，全无→`u8::MAX`（规则1 归 𝒦_x 不执行，`interp.rs:1190-1192`，故 trig 不会出现 MAX）。
- 六 bit 语义（分类器锚 `rust/src/theta_v0/classifier/bsp.rs`，Lean 锚 `Origin.BspClassification`）：
  - **一类** `IsType1 = brokeCenter ∧ IsDivergence`（`bsp.rs:8`；reference:34 `bsp.rs:53`）——**破中枢 + 趋势背驰**（第24课第一类），需 A/C 段 MACD 面积背驰证书（`bsp.rs:140`：C≥A ⟹ 一类 bit 不置）。
  - **二类** `IsType2 = afterTypeOne ∧ ¬brokeCenter`（`bsp.rs:9`；reference:35 `bsp.rs:60`）——**一类后回调/反弹结束点**，以一类已成立为前提 ⟹ 同级别时序严格晚于一类。
  - **三类** `IsType3`（reference:36 `bsp.rs:64`）——**离开中枢 ∧ 回试不入**（中枢破坏确认），**不需要背驰面积对**（`bsp.rs:155-157`：三类无 A/C 对，`force=None`）。
  - 置位点 `endpoint_to_bsp`（`bsp.rs:77-86`）；bit 非互斥（`types.rs:190`），`min_class` 取最小 ⟹ trig 是「该端点成立的**最强**类别」。

### 1.4 trig 值 → exit_type（`reverse_exit_type` 单源判据）

- `rust/src/theta_v0/strategy/interp.rs:250-258`：
  - `entry_v == ShortDiff` ⟹ **CloseShortDiff**（P7，任何 trig 类；子声部关闭语义压过触发类）
  - 否则 `trig == 3` ⟹ **ReduceCore**（P6：三类反向点=核心仓减仓）
  - 否则（trig ∈ {1,2}）⟹ **CloseRoot**（P5；**二类归 CloseRoot** 读法，`interp.rs:245-247`：PDF §9 五枚举无二类单列）
- 数据核验：518 笔中 trig≠null 的 330 笔，exit_type 与上式 **0 例不符**（逐行重算比对）。
- 诚实标注（090 声明=能力）：本 dump 的 ReduceCore 是**统计层 typed 标签**——fill loop 对该腿仍是整腿平仓（`open_trades.remove`，`runner.rs:1900`），「减仓」语义（部分平仓拆分）属 G5 未接线项（`interp.rs:218-220` 接线状态声明）。

### 1.5 与 exit.rs 的边界（两套关闭谓词，勿混）

`rust/src/theta_v0/strategy/exit.rs` 是**另一条**关闭路径（HeldVoice 台账 + §9 closePred 四析取，`exit.rs:175-243`），供 `plan_and_fill_mtm`/nautilus 消费；本 dump（`TypedTradeLedger`）走 coverage π fill loop 的 `step_trace.closed`（`runner.rs:1899`），即 §1.2 的解释器规则2。exit.rs 侧的反向项（`exit.rs:195-210`）复用同一 `reverse_signal`，但 trig 字段只由 runner.rs:1923 写入——两条路径共享判据函数、不共享账本。

## 2. trig 值 → BSP 类别/级别/确认深度 映射表

| trig | BSP 类别（锚） | 结构谓词 | 级别 | 确认深度语义 | exit_type（非 ShortDiff 腿） |
|---|---|---|---|---|---|
| 1 | 一类（bsp.rs:8,53） | 破中枢 ∧ 趋势背驰（A/C 面积对必需，bsp.rs:140） | = 被关腿级别（规则2 同级，interp.rs:1197） | **最深确认**：需完整段配对 + 背驰证书；内禀延迟最大（与 T1 19–590 bar 口径相容，见 §4） | CloseRoot（interp.rs:256） |
| 2 | 二类（bsp.rs:9,60） | afterTypeOne ∧ ¬brokeCenter | 同上 | 深确认且**时序严格晚于**一类（以一类成立为前提） | CloseRoot（interp.rs:245-247 二类归并读法） |
| 3 | 三类（bsp.rs:64） | 离开中枢 ∧ 回试不入（中枢破坏） | 同上 | 确认链**最短**：不需背驰面积对、不需先有一类；fold 内隐含「同 bar 无一/二类反向候选」（theta_key 1<2<3，interp.rs:1073-1084） | ReduceCore（interp.rs:253-254） |
| null | 无触发候选 | 结构剪枝/强平/overlay/截尾 | — | 不经反向 BSP 通道 | prune 规则归置（runner.rs:1939-1943）/RiskExit/Hold |

**关闭谓词不查区间套**（关键结构事实）：规则2 的触发只读 `bits`（exec.rs:255-261）；`nest_confirm`（interp.rs:369-382）只用于候选组装/入场侧，且现行是**单级证书基例** Conf^δ（诚实有效域：无跨级塔，interp.rs:375-377）。dump 只记**入场侧** nest 字段（`certificate.nest_confirmed/nest_depth`，runner.rs:2789-2796），**触发侧无 nest 字段**——「trig=3 ⟺ 区间套最深证书」在本数据内**不可证也不可否**，但代码链可证：关闭决策**根本没消费** nest 证书。

## 3. 交叉统计（518 笔，pnl = `pnl_raw_unlevered` 费前方向盈亏）

### 3.1 trig 总账

| trig | n | 胜率 | pnl 合计 | 均值 | 中位数 |
|---|---|---|---|---|---|
| 1 | 6 | 0.50 | **−3,879.32** | −646.6 | −16.8 |
| 2 | 109 | 0.47 | **−4,443.63** | −40.8 | 0.0 |
| 3 | 215 | 0.54 | **+18,288.96** | +85.1 | +11.8 |
| null | 188 | 0.43 | −2,604.31 | — | — |

trig=1/2 合计：n=115，**−8,322.95**。

### 3.2 trig × exit_type × side（编排者口径核验）

| trig | exit_type | side | n | win | pnl |
|---|---|---|---|---|---|
| 1 | CloseRoot | Long | 1 | 1 | +349.95 |
| 1 | CloseRoot | Short | 2 | 0 | **−4,207.26** |
| 1 | CloseShortDiff | Short | 3 | 2 | −22.01 |
| 2 | CloseRoot | Long | 43 | 25 | +1,334.69 |
| 2 | CloseRoot | Short | 30 | 12 | **−6,370.08** |
| 2 | CloseShortDiff | Long | 22 | 11 | +1,635.62 |
| 2 | CloseShortDiff | Short | 14 | 3 | −1,043.86 |
| 3 | ReduceCore | Long | 99 | 55 | **+11,132.27** |
| 3 | ReduceCore | Short | 71 | 33 | **+4,042.25** |
| 3 | CloseShortDiff | Long | 28 | 18 | +3,296.41 |
| 3 | CloseShortDiff | Short | 17 | 10 | −181.97 |
| null | CloseRoot | Long | 31 | 13 | +8,500.88 |
| null | CloseRoot | Short | 36 | 15 | −1,143.98 |
| null | CloseShortDiff | Long | 60 | 30 | −2,423.13 |
| null | CloseShortDiff | Short | 60 | 23 | **−7,375.77** |
| null | Hold | Short | 1 | 0 | −162.31 |

**编排者数字逐项对齐**：Short 侧 trig=1（−4,229.27 = −4,207.26 − 22.01）+ trig=2（−7,413.94 = −6,370.08 − 1,043.86）= **−11,643.21**；trig=3 Short = −181.97 + 4,042.25 = **+3,860.28**。口径 = **Short 侧、trig≠null 子总体**。

### 3.3 证实 / 证伪

- **证实**：Short 侧「trig=3 唯一幸存、trig=1/2 全亏」逐分对齐（§3.2）。全总体层「trig=3 唯一为正大类」也成立（§3.1）。
- **证伪（全称形式）**：「trig=1/2（深确认）全亏」对 **Long 不成立**——Long trig=1/2 合计 **+3,320.26**（n=66：+349.95 + 1,334.69 + 1,635.62）。亏损集中在 **Short × CloseRoot** 单元格（trig=1/2 两行合计 −10,577.34，占 trig=1/2 总亏 −8,322.95 的 127%——其余单元格净正对冲后仍亏穿）。**这是方向不对称问题，不是确认深度的普遍税**。
- 边界单元格：trig=2 × level 3 = +1,727.66（n=21，level≥1 的深确认反例）；trig=3 × Short × CloseShortDiff = −181.97（幸存类内的微亏单元格）。

### 3.4 辅助分布

- **trig × level**：trig=3 的 95.8%（206/215）在 level 0（+17,755.55）；level 1/2 微量（+157.23/+376.18）。trig=1/2 在 level 0 为 −6,830.35（n=60），level≥1 合计 −1,492.60（n=55）。
- **trig × 入场 bsp_class_min**：trig=3 出场者入场类 173 笔三类（+14,650.86）+ 42 笔二类（+3,638.10）——**三类进、三类出**是幸存主体；二类进三类出亦正。trig=1/2 出场者入场全为二/三类（无一类入场被一/二类反向关闭的样本）。
- **null 组**：187/188 `via_anc_ok_prune=true`（§13 连带剪枝，runner.rs:1932-1964），1 笔 censored Hold；exit_type 由 prune 规则归置（Ambient→CloseRoot / 否则 CloseShortDiff，runner.rs:1939-1943），**无 RiskExit 样本**。
- **持仓时长（exit_bar−entry_bar）**：trig=1 中位 62（n=6，不足信）；trig=2 中位 218；trig=3 中位 215；null 中位 199（均值 919.8，含 94,067 bar 截尾长尾）。
- **出场相对入场信号点（exit_bar−certificate.source_index）**：trig=1 中位 128 bar、trig=2 中位 293、trig=3 中位 291（p90：2,889 / 1,397 / 610）。
- **nest_confirmed（入场侧）**：518 笔全 true——本窗口入场全带证书基例，nest 在入场侧无区分度（再次印证 §2 末：区分度不在 nest）。

## 4. ③号问：对照区间套口径

**问：trig=3 是不是「次级别最早的反向结构点（区间套最深证书）」？**

**照实否定（代码证据）**：
1. 关闭规则只关**同级别**腿（interp.rs:1197-1202 的 `level_idx[c.level]`）——触发候选没有任何「次级别」成分；trig 的级别恒等于被关腿级别。
2. 触发判定 `reverse_signal` 是纯 bit 析取（exec.rs:255-261），**不读 nest 证书**；关闭路径上不存在「区间套深度」这个量。
3. 三类谓词本身是「离开中枢 ∧ 回试不入」（bsp.rs:64），**中枢破坏确认**——在缠论时序里它是中枢离开后的回试点，确认链比一类（需 A/C 背驰面积对，bsp.rs:8/140）**短**，不是「最深」。
4. dump 不含触发侧 nest 字段（只有入场侧，runner.rs:2789-2796）——「最深证书」一说在本数据无对应观测量。

**trig=3 的真实结构身份**：同级别三类反向点 + fold 隐含证书（该 bar 该级别无一/二类反向候选，§1.2 theta_key 优先级）。它「早」的含义至多能说到：**三类不需要等背驰面积对成立**，所以在一段新鲜反转里，同级别结构上**第一个可被确认的反向点**可以是三类（离开中枢回试不入先于完整背驰证书落地）——但「最早」在本 dump 无直接观测（无反转极值点时间戳），此读法是结构谓词复杂度的推论，不是数据实测。090 如实标注。

**问：trig=1/2 是不是「同级别深确认（T1 内禀延迟 19–590 bar）」？**

**证实其结构半**：trig=1 = 同级别一类（破中枢 ∧ 背驰），是三类中确认链最深者；trig=2 以一类成立为前提、时序更晚（bsp.rs:60 `afterTypeOne`）。「同级别深确认」成立。**「19–590 bar」量级**：本 dump 无逐信号确认延迟字段，不能直接核验该区间；trig=1 出场相对信号点中位 128 bar、trig=2 中位 293 bar（§3.4），与「一/二类确认要付出十位到百位 bar 级等待」量级相容，不冲突。精确区间核验需 classifier 侧加确认延迟观测（授权外，登记）。

## 5. 结构决定的退出反向点类别建议（不拍阈值，由映射数据说话）

按出场必须走区间套背驰、止盈=反向买卖点、进出场同一条线的最高口径，映射数据给出的结构事实：

1. **现行反向关闭覆盖一至三类全集**（规则2 对 sell1∨sell2∨sell3 / buy1∨buy2∨buy3 无差别触发，exec.rs:257-258）——「买点买卖点卖」在谓词层已全类覆盖，无类别缺失。
2. **亏损不在「等三类」而在 Short × 深确认单元格**（§3.3）：trig=1/2 的 −8,322.95 中，Short × CloseRoot 一格占 −10,577.34。若问题是「等错层级的滞后」，数据指的方向是 **Short 侧一/二类反向确认**这一格，而非一类确认本身（Long 同类单元格为正）。
3. **trig=3（三类反向）是本窗口唯一自生存出场类**（+18,288.96，且 Long/Short 双侧 ReduceCore 均正），其结构身份 = 中枢破坏确认 + 无背驰面积对依赖——确认链最短，「等错层级」的可修复分量最小。
4. **二类归 CloseRoot 的读法**（interp.rs:245-247）把「一类后回调点」与「一类反转点」并入同一 typed 桶；trig=2 是全样本最大 n 的深确认类（n=109，−4,443.63）。若后续按类别拆分出场语义，二类是**单列价值最高**的一格（样本量够、盈亏结构独立于一类）。
5. **区间套不压滞后于关闭侧的现状**：关闭谓词不消费 nest 证书（§4），「区间套把滞后压到因果下限」在**出场方向**目前没有被机制兑现——入场侧 nest 全 true 无区分度（§3.4），出场侧连证书都没接。这是结构事实登记，不是参数建议。
6. **本对账不支持的读法**（090 照实否定）：「trig=3 = 次级别最早反向结构点 = 区间套最深证书」。任何以此身份为前提的后续方案，前提不成立。

## 6. 可修复分量 vs 内禀分量落账（按编排者验收口径）

- **内禀分量（causal floor，不修）**：三类谓词都需结构落地后才能确认（一类待背驰面积对、二类待一类后回调完成、三类待回试完成）——确认前的浮盈回吐是「不预测」的代价。
- **可修复分量（数据指向）**：Short × 一/二类反向确认格的 −10,577.34（§3.3）；Short × null 剪枝格的 −7,375.77（§3.2 末两行——无触发候选的连带剪枝，非反向买卖点出场，与「出场必须走区间套背驰」口径的距离最大）。
- **本次对账未动**：代码零改（只读），`cargo test` 无触发必要（无源码变更）；基线 1752 passed 未触碰。

## 附：核验命令

```bash
python3 - <<'EOF'
import json, collections
trades=[json.loads(l) for l in open('/tmp/m8_opsem_fixed/trades.jsonl')]
# trig x exit_type x side 交叉（本文 §3.2 全部数字可由三段聚合复现）
EOF
```

数据文件：`/tmp/m8_opsem_fixed/trades.jsonl`（518 行）。exit_type↔trig 一致性重算：330 笔 trig≠null 逐行比对 `reverse_exit_type` 规则，0 例不符（§1.4）。
