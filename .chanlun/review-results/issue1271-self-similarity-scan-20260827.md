# #1271 全仓自同构性盘点：四类破坏形态逐处扫（2026-08-27）

> 票：GitHub Issue #1271（[#1226](https://github.com/xy7365527-lang/NewChanlun/issues/1226) 拆出）。
> 性质：**盘点登记，零代码改动**（修另开票）。
> 范围：`rust/src/` 全仓生产面（不含 `bin/` 探针与 `tests/` 测试；合计 394 文件 / ~322k 行）。
> 方法：静态扫（grep + 定点读码）；逐处标「形态类 + file:line + 一句证据 + 已修/待烤」。
> 基线：`main @ 6a2a0737c2`（本分支基于此，工作树干净；扫描后 main 前移至 `1b33968ebc`，含 #1266）。
> 自同构性判别式·广义版（#1226 红线，2026-08-27）：同一套递归判据每级复用、级别只作参数；破坏形态四类。

## 结论摘要

- 四类共登记命中 **41 处 site**：已修 **11**（类2=4、类3=4、类4=3）、待烤 **27**（类1=2、类2=2、类4=4 已开票 + 类4=19 批外残余）、已登记例外 **2**（类2 中枢构造两套 + 分派点）、已登记待修 **1**（类4 buysellpoint T-6）。
- 另计「确认不动/移出辖」：类4 已裁合法载体 **14 项**（#1232 第四轮裁定，判据与代码不动）+ 风控门 **3 处**（#1232 第五轮裁定移出本图辖）。
- **新发现（批外残余）**：类 4「027:25 否定线」价格代理（价格越过 candidate 极值 ⟹ 窗口作废/定位失效/声部解栈）在 `trading/` 赋格族 + `spiral/` 引擎被复制到 **11 个文件 19 处 site**；#1232 批只 ticketed `nested_fugue.rs` 三处（#1263 探针前置），其余为批外残余，本票登记为待烤。
- 类 1（跨级给值）仅 1 处在案（#965 传播，待 #1270 实装）；类 2 已知项均已修/在修，#804 中枢构造两套为已登记例外待收；类 3 仅 #1073 在案（已修），零残余。
- 本次静态扫**未发现类 1/类 3 的新同型**；类 2 除 #1265（D-3 取段）外无新两档形状；类 4 的批外残余集中在「027:25 否定线」一族。

---

## 类 1：跨级给值（上级→下级赋值，#965 传播型）

### 待烤（已裁定、待实装）

| file:line | 证据 | 状态 |
|---|---|---|
| `theta_v0/backtest/admission.rs:1156-1168` | `for g in levels.iter_mut().rev()`：`g.status == Closed` 置 `closed_above`，下级 `MissingCert && closed_above && has_unconf` ⟹ `g.status = ChainLevelStatus::Closed`——上级 Closed 给下级赋值升级，非该级自判。 | #1242 裁定 a′「停用传播、每级自判」；实装 #1270（OPEN，待修）。 |
| `theta_v0/backtest/admission.rs:1172-1185` | 缺/断极性 `seen_closed_above ⟹ Broken/Missing` 依赖上一处传播后的 status，属同一面（#1270 撤销后随每级自判重算）。 | 同上，随 #1270 收。 |

### 同型残余（扫）

- `ChainLevelStatus` 全仓写入点仅 `admission.rs`（:1126-1133 为每级自判、:1168 为跨级赋值），无第二处上级→下级状态赋值。
- 其余「传播」均为**增量缓存失效**（`pipeline.rs:731-877`/`tower_cache.rs` 的 cascade 脏源下界）或**链 source 选择**（`positioning_chain_fugue.rs:879`「高 source 优先，低 source 不降级」），不构成「上级判据写进下级」。
- **结论：无新同型。**

---

## 类 2：每级判据分档（宽严两档，#799/#804 类）

### 已修（#1228/#1229/#1230/#1234/#1249/#1261）

| 项 | 状态证据 | 状态 |
|---|---|---|
| #1228 sublevel_diverges 收敛进 div_cand | grep `sublevel_diverges` 生产调用方 = 0，仅注释残留（`bsp.rs:683`、`pipeline.rs:1684/1734`）。 | 已修（`b01a7b075b`）。 |
| #1229/#1249 T3-in-c 全窗后扫 | `t3_in_c_fixed_first_pair`（`signal.rs:447`）保留为诊断词汇，生产调用方 = 0（仅 `signal.rs:3718+` 测试）。 | 已修（`93af09344b`）。 |
| #1230/#1261 pan_provider 盘背取段定位+单判 | `level_view/pan_provider.rs:403-406`：先 D-3 定位唯一 A（窄锚结构存在取窄锚、不存在才 A′），再单判 Extreme 一次（判负即止，不换段重判）。 | 已修（`6a2a0737c2`）。 |
| #1234 执行段定位区间包含 | `pan_div_gate_pass`/`build_xzd_fallback` 已弃 `find_move_by_end_index` 改 `find_move_containing_index`；`descend.rs:110` 已带区间包含回退；`econ_positive.rs:1421/1631` 仅诊断归因路径（end== + 区间包含回退）。 | 已修（`308389032c`/`1eb0d64a81`）。 |

### 已登记例外（#804 在案实例，待第二批中枢概念票收）

| file:line | 证据 | 状态 |
|---|---|---|
| `classifier/center.rs:241` vs `:283` | 中枢构造两套、签名逐字相同：`center_from_segments`（含 `dir_alternates` 方向交替）vs `center_from_window`（无方向交替）。举证书在 `center.rs:266`「★诚实有效域」。 | 已登记例外（#804 ③，退场条件待第二批中枢概念票补齐）。 |
| `classifier/recursive_tower.rs:397`（compose_level）/ `:640`（compose_level_resume）/ `pipeline.rs:1066`（op_build） | `let build = if is_l0 { center_from_segments } else { center_from_window }` 三处分派点。 | 同上一例外；AGENTS.md 原记行号 `:304/:859` 已漂，订正为 `:397/:640`。 |

### 待烤

| file:line | 证据 | 状态 |
|---|---|---|
| `classifier/cand_predicate.rs:335-353` vs `classifier/signal/cert.rs:128-185` | D-3 取段两实现体：`div_cand` 条件2 = `rfind` 最近同向跨界段（`crosses(m) \|\| m.end_index <= c.start_index`，**无回中枢要件**，#883/#979 已重写）；PanDiv 窄锚多 `reenters` 回中枢要件（`cert.rs:141-151`/`:163`）。 | #1231 裁定 a「收成一份、回中枢要件裁掉」；实装 #1265（OPEN，待修）。 |
| `classifier/signal/cert.rs:113-125`/`:197-208`（`_allowing_unbroken_c` 变体） | #483 观测专用定位器（复用窄锚/A′ 机制但允许 C 端点严格在核心内），与生产 `locate_pan_div_structure` 同族。 | 观测专用，不参生产准入；归 #1265 同族收编面。 |

### 备注（非违规，登记）

- `classifier/pipeline.rs:320/341` 的 `detect_centers_complete`/`detect_centers_geometric` 两 helper 均为 `#[cfg(test)]`（#1053 后仅测试消费），是上表中枢构造两套的测试侧投影，不另计。
- `econ_positive/xzd.rs:67` 字段注释仍写「level==1 硬门参门项；level>=2 维持 C2-only」，与现行 `gate_pass()`（`xzd.rs:89`：`type2_confirmed && c3_new_center_breakout_ok && !force_exception_ok`，级别无关）**注释漂移**——零行为影响，登记待订正（可并入 #1268 类引文/注释订正小票）。

---

## 类 3：硬编码级别（参数写死成常数，「必须降到 a0」型）

### 已修（#1073）

| file:line | 证据 | 状态 |
|---|---|---|
| `recursive_t/divergence.rs:559-593` | `nest_chain_complete` 下钻循环 `for k in (min_level..cc).rev()`——停止级 `min_level = max(成本门级, 塔底级)` 作参数传入（:580），不再是 `(0..cc).rev()` 的常数 0 强制。 | 已修（#1073）。 |
| `recursive_t/divergence.rs:603-648` | `d_top` 同款：`min_level` 作参数下传 `nest_chain_complete`。 | 已修（#1073）。 |
| `recursive_t/rec_stream.rs:415` | `d_top` 调用点传 `min_level=0`——注释自述「阶段一 = 0」（塔底=a₀=0，成本门据 #907 不咬现役级别），是参数值非钉死判据。 | 已修（#1073，判据+参数分离）。 |
| `theta_v0/backtest/econ_positive/descend.rs` | `descend_type1_anchor_depth` 终止 = `BaseL0`（sub_moves 空，L0 天花板）+ `NoDivergence`（div_cand 判假，成本门），显式声明「不钉死级别——钉死 = 把参数写死成常数」。 | 已修（#817 N-2/#1076）。 |

### 同型残余（扫）

- 「递归到 a0」的 trading 赋格族（`nested_fugue.rs:135` 等 `rec_sub_evidence` 下探到 `FIRST_BSP_LADDER-1`）是递归**自然底**（塔底=a₀ 定义性，`find` 找到证据即停），非「必须降到 a0」的逐层强制，不计违规。
- **结论：无新同型。**

---

## 类 4：非结构判据代结构（价格代理，#1232 批已查 + 批外残余）

### 已裁合法载体（#1232 第四轮裁定「ok」，判据与代码不动，14 项）

A2（`cand_predicate.rs:353-360` Extreme）、A3（`signal.rs:293-343` judge_third_cert）、A4（`signal/gates.rs:80-93` first_structural_gates.extreme）、A5a（`signal.rs:363-387` trend_third_class_in_c 全窗后扫）、A6（`signal/cert.rs:243-264` pan_div_structure_extreme）、A7（`level_view/confirm.rs:362-372`、`level_view/pan_provider.rs:252-266` trend_confirm T2）、A8（`turn_class.rs:112-197` candidate_second_point）、A9a/A9b（`rmove_compose.rs:84-116` no_new_low/high，仅标注取值）、A10（`center.rs:325-336` classify_position）、B1（`econ_positive/xzd.rs:203-227`）、B3（`recursive_t/operator.rs:54-84` detect_type3）、B4（`recursive_t/divergence.rs:210-222` G 门）、B5（`recursive_t/divergence.rs:346-365` 条件2守沿）。

### 已修

| 项 | 状态证据 | 状态 |
|---|---|---|
| A1 rmove_dir 单中枢 fallback | `cand_predicate.rs:165-174/235-242`：三中枢逐对判（#900 F2）、单子走势恢复旧语义（F3）、例外条款落 `.chanlun/definitions/qushi.md`（F4，#804 ③）。 | 已修+已登记（#900，`9b17dbb5b1`/`56d81d9c3f`）。 |
| A5b t3_in_c_fixed_first_pair | 退役为诊断词汇（同类 2 #1229/#1249）。 | 已修。 |
| B2 xzd_force_exception 触发前件 | `econ_positive/xzd.rs:227-342`：例外臂判据落「后续走势类型发展」（新中枢上移/三买卖点确认），价格新高/新低不构成判据。 | 已修（#1220，`43e14c79d9`）。 |

### 待烤（已开票）

| file:line | 证据 | 状态 |
|---|---|---|
| `trading/nested_fugue.rs:400-406`（B7a 窗口清窗）/ `:461-467`（B7b 定位失效）/ `:498-515`（B7c 声部解栈） | 「价格越过 candidate 极值 ⟹ 否定」三消费点（`c > w.extreme` 清窗、`c > ext` located 失效、`negate_line` 破 ⟹ unwind_to）。 | #1232 第一条，等 #1263 探针读数（吻合/误杀/漏杀面），未裁。 |
| `trading/fatigue_gate.rs:87-94`（B8） | `new_high = c > high_water`，`new_high \|\| center_formed \|\| up_settled ⟹ Fresh`——close 创新高作清空路径 1。 | #1232 第二条裁定 a「补力度项（创新高 ∧ 不背驰）」；实装 #1264（OPEN，待修）。 |
| `trading/trend_exhaustion.rs:133-140`（B9a）/`:146-155`（B9b） | `down_unexhausted`/`up_unexhausted`：`cur < prev` 创新低 / `cur > prev` 创新高 ∧ 无盘背。 | #1232 第三条裁定 a「换结构判据『走势未完成』」；实装 #1267（OPEN，待修）。 |
| `recursive_t/divergence.rs:328/391/429`（B6 leg_strength 力度） | `leg_strength` 仍作判据力度分量（结构/盘整背驰 + 趋势结构滤网）；#1243 已裁定退役，实装 #1266（ForceL）已合入 main（`1b33968ebc`），但**晚于本基线**，基线代码未含。 | #1243 裁定；实装 #1266（`1b33968ebc`，本基线后合 main）；本基线待烤。 |

### 已登记/移出辖

| 项 | 证据 | 状态 |
|---|---|---|
| `buysellpoint.rs:272-277`（`make_type2_point`）/ `:841-846`（`build_type2_bsp`）（旧引擎 geom 合取，两副本） | `confirmed = geom && ...`，`geom` = 回调不创新低/反弹不创新高作**必要条件**（#816 B-2② 已判不得为必要条件）。注释自述「theta_v0 侧 #884 已拆；本旧引擎与 Python 同拍，T-6 落地时两侧一并改」。 | 已登记（#816 T-6/673-fix，待修）。 |
| B10a/B10b/B10c 风控门三处 | 入场结构止损逆侧复检（`backtest/signal.rs:176-191`）、持仓止损触及（`strategy/exec.rs:85-121/263-275`、`admission.rs:1338-1364/1491-1518`）、强平价格门（`recursive_t/t_engine.rs:957-975`、`nested_fugue.rs:477-486`）。 | #1232 第五轮裁定「移出本图辖」：非缠论判定（系统自设风控，#659），只登记不回改。 |

### 批外残余（本票新登记，#1232 批只查了 nested_fugue.rs 三处）

「027:25 否定线」同型复制族——`价格越过 candidate 极值 ⟹ 窗口作废 / 定位失效 / 声部解栈`，与 nested_fugue.rs B7a/b/c 逐字同构，分布在以下生产文件（均非 `bin/` 探针）：

**B7a 同型（窗口清窗 `c > w.extreme ⟹ nest = None`）：**

| file:line | 证据 | 状态 |
|---|---|---|
| `trading/positional_fusion.rs:819/823` | `nest_sell[k].is_some_and(\|w\| c > w.extreme) ⟹ None`（注释「① 打破背驰段 ⇒ 作废：卖窗创新高、买窗创新低」）。 | 批外残余，待烤。 |
| `trading/unified_recursive.rs:223/227` | 同上。 | 批外残余，待烤。 |
| `trading/unified_voice.rs:241/245` | 同上（:404 的「禁新高/新低」是 #1222 XZD 证伪线另一 site，本处 nest_forward ① 仍为价格否定）。 | 批外残余，待烤。 |
| `trading/isolated_fugue.rs:530/534` | 同上。 | 批外残余，待烤。 |
| `trading/axiom_voice.rs:228/232` | 同上（:221 注释「区间套三段式 ①破极值否定」）。 | 批外残余，待烤。 |
| `trading/nested_interval_fugue.rs:202/206` | 同上。 | 批外残余，待烤。 |
| `trading/recursive_nested_fugue.rs:191/195` | 同上。 | 批外残余，待烤。 |
| `trading/dual_voice.rs:265/269` | 同上。 | 批外残余，待烤。 |
| `trading/positioning_chain_fugue.rs:340/344` | 同上。 | 批外残余，待烤。 |
| `trading/unified_necessity.rs:1234/1238` | 同上（:75 自述「027:25 否定线在 candidate/located 维护中保留」）。 | 批外残余，待烤。 |
| `spiral/signal.rs:227/231` | 同上（注释「① 破极值否定（027:25）」；spiral 引擎 v2 复用 nest_forward 信号层）。 | 批外残余，待烤。 |

**B7b 同型（定位失效 `c > ext ⟹ located = None`）：**

| file:line | 证据 | 状态 |
|---|---|---|
| `trading/positioning_chain_fugue.rs:434/437` | `located_sell[k].is_some_and(\|e\| c > e.extreme) ⟹ None`（注释「破极值否定（027:25）：级联统一极值 ⟹ 整链同破」）。 | 批外残余，待烤。 |
| `trading/unified_necessity.rs:1366/1369` | 同上。 | 批外残余，待烤。 |
| `spiral/signal.rs:335/338` | 同上（注释「破极值否定（级联统一极值 ⇒ 整链同破）」）。 | 批外残余，待烤。 |

**B7c 同型（声部解栈 `negate_line 破 ⟹ unwind_to`）：**

| file:line | 证据 | 状态 |
|---|---|---|
| `trading/unified_recursive.rs:324-334` | B 否定扫描：`chain.iter().position(v.negate_line.is_some_and(...c >/< line))` ⟹ `unwind_to`。 | 批外残余，待烤。 |
| `trading/positioning_chain_fugue.rs:470-480` | 同上。 | 批外残余，待烤。 |
| `trading/nested_interval_fugue.rs:299-309` | 同上。 | 批外残余，待烤。 |
| `trading/recursive_nested_fugue.rs:294-305` | 同上。 | 批外残余，待烤。 |
| `trading/isolated_fugue.rs:643-651` | B 否定扫描（逐活跃 voice 破 027:25 极值线 ⟹ 关 voice + 子树）。 | 批外残余，待烤。 |

> 注：`unified_necessity.rs` 的操作层 B 否定扫描（voice 解栈）已按 2026-06-15 编排者裁定删除（`unified_necessity.rs:62-103/1426-1498`），仅 candidate/located 破极值清窗（B7a/B7b）保留；其余文件三态俱全。

---

## 去重对照（#1226 frontier 合表与已裁票）

| #1226 子票 | 类 | 本票登记位置 | 对照结果 |
|---|---|---|---|
| #1228 sublevel_diverges→div_cand | 2 | 类2 已修 | 去重（在案，已修）。 |
| #1229/#1249 T3-in-c | 2 | 类2 已修 | 去重（在案，已修）。 |
| #1230/#1261 pan_provider 定位+单判 | 2 | 类2 已修 | 去重（在案，已修）。 |
| #1231/#1265 D-3 取段两实现体 | 2 | 类2 待烤 | 去重（在案，待修）。 |
| #1234 执行段定位区间包含 | 2 | 类2 已修 | 去重（在案，已修）。 |
| #1232 价格代理批（27 项） | 4 | 类4 已裁/已修/待烤/移出辖 | 去重（在案）。 |
| #1233/#1269 类第二类点 | 4 | QuasiSecondCert 零载体，见下注 | 零构造点零消费（#1269 待落地判据），不属四类破坏形态（无判据可破坏），不重复登记。 |
| #1242/#1270 #965 传播 | 1 | 类1 待烤 | 去重（在案，待修）。 |
| #1243/#1266 leg_* 退役 | 4 | 类4 待烤（B6，已开票 #1266 晚于本基线合 main） | 去重（在案，实装已合 main `1b33968ebc`，本基线待烤）。 |
| #1073 硬编码停止级 | 3 | 类3 已修 | 去重（在案，已修）。 |
| #804 中枢构造两套（AGENTS.md 在案实例） | 2 | 类2 已登记例外 | 去重（在案，例外待第二批中枢概念票收）。 |
| #900 rmove_dir 单中枢 fallback（A1） | 4 | 类4 已修+已登记 | 去重（在案，已修）。 |
| #816 B-2② buysellpoint 旧引擎 geom | 4 | 类4 已登记（T-6/673-fix） | 去重（在案，待修）。 |
| —— | 4 | 类4 批外残余「027:25 否定线」11 文件 19 site | **新增（批外，未在任何票登记）**。 |

> 注：frontier 合表 `.chanlun/review-results/issue1226-frontier-scan-synthesis-20260825.md` 未入 main（引用自 #1228/#1230 票面），本票以 #1226 评论内合表（9 子票 + 6 research）与 #1250-1256 research 报告为准对表。

## 结论

1. **类 1（跨级给值）**：在案 1 处（`admission.rs:1156-1168`），#1242 已裁停用、#1270 待实装；全仓无新同型。
2. **类 2（判据分档）**：#1228/#1229/#1230/#1234 已修落地；#804 中枢构造两套为已登记例外（举证书在案，待第二批中枢概念票收，行号订正 `recursive_tower.rs:397/640`）；#1265（D-3 取段收成一份、回中枢要件退役）待实装。
3. **类 3（硬编码级别）**：#1073 已修（判据+参数分离落地于 `nest_chain_complete`/`d_top`/`descend_type1_anchor_depth`）；全仓无新同型。
4. **类 4（价格代理）**：#1232 批已全面裁定（合法载体 14 项不动、B2 已修、B8/B9 待修、风控门移出辖；B6 leg_strength 实装 #1266 于本基线后合入 main `1b33968ebc`，基线代码待烤）；**本票新登记批外残余：`trading/` 赋格族 + `spiral/` 引擎 11 文件 19 处 site 的「027:25 否定线」同型（价格越过极值 ⟹ 作废/失效/解栈），#1232 批只查了 `nested_fugue.rs` 三处**，建议由编排层决定并入 #1263 探针面或另开收编票。
5. 零代码改动；本报告仅为盘点登记，不代裁。
