# 买卖点引擎层"次级别走势"缺口 — 盘点、定级与矛盾上浮

> 任务（2026-06-13 编排者）：将 BSP 检测中代理"次级别走势"的 segment（线段）替换为
> move（走势类型），消除"走势没完成就判"的伪买卖点。要求实装 + OKLO 回测对照。
>
> **本报告结论先行：任务前提（"三类 BSP 用 segment 代理次级别走势"）属实，但其修正
> 方向（"用该层 moves 替换"）是一次越级错配，且与三处已结算/已否证记录冲突。按
> `no-workaround` 规则停在矛盾处，不落码字面修正；真实开放轴是另一个位置（递归层
> BSP，§5）。** 认识论等级：本报告为 **L0/概念层**（定义溯源 + 谱系核对，未跑数据）。

---

## 0. 判决（先行）

| 子命题 | 真假 | 依据 |
|---|---|---|
| 三类 BSP 用 segment 作"次级别走势" | **真** | `buysellpoint.rs` type2/type3 用 `find_next_seg_by_direction`；type1 用 `div.seg_c_end` |
| segment 是"次级别走势的代理"，需替换 | **否（已判定结构同构）** | 段 = a₀ = Move[0] = 次级别走势**本体**（谱系 003/006，缠师 017:40 / 033:182） |
| "走势没完成就判 → 伪买卖点"现象存在 | **真** | `engine_bsp_gap_diagnosis.md §3`：生长期 C 段瞬态伪背驰（2559 fire / 1120 反悔） |
| 该伪买卖点根因 = segment-vs-move | **否** | 根因是 C 段被 move 边界截断 + 生长期评估；已由 **B2** 修复（在码） |
| "用该层 moves 替换 segment" | **越级错配** | 该层 moves = Move[1] = 中枢的**父级**走势，非其构件（次级别） |
| move 完成判定加入 BSP（= 任务承认的"确认滞后"） | **已 L2 否证** | `sublevel_confirmation_recursive.md`：SCm −1189pp，卖侧确认全谱系否证 |
| 真实开放轴 | **递归层 BSP（level≥2）** | 引擎只在 level_id=1 算 BSP，递归层产出 moves 却从不跑检测（§5） |

---

## 1. 盘点：BSP 检测当前如何使用"次级别走势"（任务第1步）

`buysellpoints_from_level`（`rust/src/buysellpoint.rs`）三类检测的"次级别走势"载体：

| 类 | 操作点 | "次级别走势"载体 | 代码 |
|---|---|---|---|
| type1 | 趋势背驰转折点 | `div.seg_c_end`（背驰 C 段的趋势极值段） | `detect_type1` L193 |
| type2 | 一买后回试端点 | `find_next_seg_by_direction` 找 rebound(单段)→callback(单段) | `detect_type2` L278-307 |
| type3 | 中枢破坏回抽点 | `find_next_seg_by_direction` 找 pullback(单段) | `detect_type3` L385 |

**type1 不属于本任务范围**：`seg_c_end` 不是"次级别走势代理"，而是背驰转折点的**价格定位**
（背驰本身在 move 上检测，`divergences_from_moves_v1` 逐 move）。任务"type1 也是单段线段"是误读。
真正用单段段作"次级别走势离开/回试"的只有 type2/type3。

## 2. 定义溯源：a₀=线段 是次级别走势"本体"，不是"代理"（任务第3步分析）

### 2.1 原文递归基（一级权威）

- **017:40**（缠师亲自封口）：「级别之次也不可能无限……对最后不能分解的级别，其缠中说禅
  走势中枢就不能用"至少三个连续次级别走势类型所重叠"定义，而定义为至少三个**该级别单位
  K线重叠部分**」。
- **033:182**：「**递归，最小级别的中枢用三根K线就完了。** 然后用类似 aₙ₊₁=f(aₙ) 的形式进行下去」。
- 推导链（`concept_movement_chain_canon_crosscheck.md` L147）：**a₀ = 线段**，aₙ₊₁ = f(aₙ)。

### 2.2 已结算谱系

- **谱系 003**（线段两个口径，已结算）：**Move[0] = v1 Segment 为唯一口径**。即"线段级走势"
  在递归基处**就是线段对象本身**（`SegmentMoveAdapter`），1 段 = 1 个 Move[0]。
- **谱系 006**（级别递归，已结算）：递归级别为唯一路径，"每层由下层的走势类型组件构成"。

### 2.3 级别代数（关键）

```
笔(stroke)
└─ 线段(segment) = a₀ = Move[0]              ← 递归基：线段就是最底层"走势类型"
   └─ 线段级中枢(代码 level-1 zhongshu)        ← 由 segment(=Move[0]) 构成
      └─ 线段级走势(代码 Move[1])              ← 线段级中枢的 趋势/盘整
         └─ 线段级走势中枢(level-2 zhongshu)   ← 由 Move[1] 构成
            └─ Move[2] ...
```

对一个**线段级中枢**（= 代码在 level_id=1 检测 BSP 的中枢）：
- 其**次级别走势 = 构件 = segment = a₀ = Move[0]**。
- 当前 type2/type3 用 segment 作"离开/回试" → **与第18/30/53课逐字一致**（"一个次级别走势离开…
  一个次级别走势回试"，在递归基处次级别走势=段）。**这不是代理，是本体。**

### 2.4 既有诊断的同一结论

`engine_bsp_gap_diagnosis.md §2.3`（前序 session）逐字：
> 代码以"本级别段"作"次级别走势"的代理（段是本级别走势的次级别构件），**结构同构、可接受**。
> 真正的差距不在 type2 检测，在 type1 的存在论……

即：segment-as-次级别走势 已被分析并定级为"**结构同构、可接受**"。任务所指的缺口，前序
已判定**不在此处**。

## 3. 任务"伪买卖点"现象的真实根因（任务第4步影响面）

任务担忧"单段线段只是次级别走势的一部分，走势没完成就判"。该现象**真实存在**，但根因
被误归：

- `engine_bsp_gap_diagnosis.md §3`：2559 个 confirmed type1 buy **全部**产生于 pending move
  生长期——A 段完整、C 段在长 → `force_c<force_a` 几乎必然 → 伪背驰；C 段走完后 **1120/1121
  反悔**。这是"走势没完成就判"的**精确机制**：是**背驰 C 段的生长期瞬态**，不是 segment≠move。
- 根因二（§2.2）：settled 后离开段被下一中枢吸收 → C 段空 → type1 锚消失 → type2 源恒空。

**两者均已修复，且修复与"segment→move"无关：**
- **修复A**（`level.rs:217` 已落地）：递归层 move 边界对齐 level-1（末组 seg_end = num_components−1）。
- **B2 越界极值**（`divergence.rs:227/409` 已落地，缠师第24课"背驰段终于转折点"）：C 段终点 =
  窗口内趋势极值段，不被 move 边界截断 → type1 存活 1→374、type2 解锁 0→360（记忆
  `project_c_segment_fix_b2`）。

**结论**：任务想解决的伪买卖点，根因是 C 段定义，已由 B2 在 segment 口径内严格修复——
**换 move 既非必要也非充分。**

## 4. 为什么"用该层 moves 替换 segment"是越级错配（核心矛盾）

"该层 moves" = `Move[1]`（线段级走势）。但由 §2.3 级别代数：
- `Move[1]` 是线段级中枢所**从属的父级走势**，不是其**构件（次级别）**。
- 把 `Move[1]` 塞进"次级别走势离开线段级中枢"= 用**父级**充当**子级** = **越级**。

谱系/记忆先例：`project_trend_direction_proxy`——"swing 对象否定层级**越级 = 标签错配**，
非'弱于'"。越级不是"更严格的近似"，是**类型错误**。

**唯一能让"次级别走势 = move（真走势类型）"成立的位置是 level-2**：检测对象是**线段级
走势中枢**（由 Move[1] 构成），其次级别走势 = Move[1]。这不是"在 level-1 替换 segment"，
是"把 BSP 抬到 level-2"——见 §5。

### 4.1 任务自承的"确认滞后"已 L2 否证

任务明言：「moves 在该层的生成可能有滞后……这就是确认滞后的信号层根源」。
`sublevel_confirmation_recursive.md`（L2，OKLO 447K，6 变体消融）判决：
- **SCm −1189pp**（master 卖出场加次级别 move 完成确认）：强趋势顶出场延迟毁灭性。
- 机制：**confirmed 语义已是次级别结构完成的编码**，再加 = 双重确认 = 纯延迟。
- **"卖侧/出场侧的次级别确认全谱系否证，该方向关闭"**（R1/R3/P6/SC 四案收敛）。

字面修正（segment→move）等价于把"等 move 完成"塞进**每一个** BSP 的检测时点，
= 在信号生成层重演已否证的确认滞后机制。

## 5. 真实开放轴：递归层 BSP（level≥2）

引擎事实（已核码）：
- `RecursiveOrchestrator` 只在 `level_id=1` 算 BSP（`compute_bsps`/`update_bsps_incremental`）。
- 递归栈 `LevelEngine.process` 产出 `LevelSnapshot{ zhongshus, moves }`——**无 buysellpoints 字段**。
- 即：修复A 已让递归层 moves 边界正确，但**递归层从不运行 BSP 检测**。

**真正"用 move 作次级别走势"的严格形式**：在 level≥2 运行 `buysellpoints_from_level`，
入参为该级别的 `LevelZhongshu` + `Move[k-1]`（真·走势类型作次级别）。这是把字面任务
**抬到正确级别**后的版本，与 §2.3 级别代数自洽，且不越级。

但它是一个**独立的、更大的改动**，且前序诊断 §4 已将其相邻项（递归层 type1 解锁）
标注为"口径漂移，待编排者确认"。其已知性质：
- level-2 中枢稀疏（数据中 L2 settle ~100–200，vs L0 ~350K）→ BSP 极稀 → 信号近 buy-hold。
- 与 `sublevel_confirmation` 的"等待型 vs 信号型"二分需重新对账（level-2 BSP 是信号型还是
  区间套定位型？）。

---

## 6. 矛盾上浮（四分法分类：选择 + 语法记录）

**矛盾**：任务规格的前提与三处已结算/已落地/已否证记录冲突，无法在不重开谱系的前提下落码。

- **若接受 A（任务字面：level-1 segment→move）**：则
  (1) 否定谱系 003（Move[0]=Segment 唯一口径）；
  (2) 构成越级错配（Move[1] 当次级别，§4）；
  (3) 重演 `sublevel_confirmation` 已否证的确认滞后（§4.1）。
- **若接受 B（前序结论：segment=a₀ 是次级别走势本体，B2 已修伪买卖点）**：则任务前提撤回，
  伪买卖点已在 segment 口径内严格解决，无需改动。
- **若接受 C（抬到正确级别：递归层 BSP，level≥2）**：则需接受口径漂移（在册回测重跑）+
  信号稀疏，且需先裁决 level-2 BSP 的操作语义（信号型/区间套型）。

**为什么必须上浮而非自决**：A 需要重开已结算谱系 003/006（`原则0` 下只有编排者有此合法性，
触发 `020号` 阻断）；C 是方向选择（口径漂移 + 稀疏性的价值判断）。二者均落在
`no-unnecessary-escalation` 允许提问的"定义真实矛盾 / 方向选择"。B 是定理（前序已结算），
但 A/C 的取舍需编排者裁决。

---

## 结果包六要素

1. **结论**：BSP 三类检测确以 segment 承载"次级别走势"（type2/type3），但该用法 = a₀=Move[0]=
   次级别走势**本体**（谱系 003/006，缠师 017:40/033:182），非代理；前序 `engine_bsp_gap_diagnosis
   §2.3` 已定级"结构同构、可接受"。任务想消除的伪买卖点根因是 C 段生长期瞬态/截断（§3），
   已由 **B2**（在码）严格修复，与 segment-vs-move 无关。字面修正"用该层 moves 替换"= 越级错配
   （Move[1] 是父级非次级别）+ 重演已 L2 否证的确认滞后。**未落码**；真实开放轴是递归层 BSP
   （level≥2，§5）。
2. **定义依据**：第17课中枢定义（"至少三个连续次级别走势类型重叠"）；017:40 递归基封口
   （最低级别用单位 K 线）；033:182 aₙ₊₁=f(aₙ)；第53课三类买卖点定理（"一个次级别走势类型离开…
   回试"）；第24课背驰段终于转折点（B2 依据）。代码依据：`buysellpoint.rs:278-385`、
   `divergence.rs:227-417`、`orchestrator.rs:408-471`、`level.rs:217`。
3. **边界条件（结论翻转条件）**：(a) 若编排者裁定重开谱系 003，认定线段级中枢的次级别走势
   应为 Move[1]（即承认线段不是合格走势类型、最低操作级别须上抬），则 §4 越级判定翻转、
   任务字面修正成立；(b) 若递归层 BSP（§5）在 L2/L3 回测中**净正且非 buy-hash 伪影**，则"信号
   稀疏 = 无价值"翻转；(c) 若某标的上 C 段生长期伪背驰在 B2 后仍残留（B2 只修 settled 侧，
   pending 生长期瞬态按流式快照本性不变），则需独立处理 pending 期 type1，但仍非 segment→move。
4. **下游推论**：(a) 生产 BSP 引擎零改动（segment 口径正确）；(b) "等待型/信号型操作点二分"
   （`sublevel_confirmation` §4）扩展为 BSP **检测层**语法：检测时点用 move 完成判定 = 信号型确认
   = 先验否证方向；(c) 若开 §5 递归层 BSP，须先定其操作语义并预注册否证判据。
5. **谱系引用**：003（Move[0]=Segment 唯一口径）、006（级别递归唯一路径）、029（Move C 段覆盖）、
   031（Move 范围语义）、526（a₀ 递归存在论区分）；分析记录 `engine_bsp_gap_diagnosis.md §2.3/§3/§4`
   （修复A/B2）、`sublevel_confirmation_recursive.md`（确认滞后否证）、`concept_movement_chain_canon_crosscheck.md`
   （a₀=线段推导）。**未核对到专门针对"segment 作次级别走势载体"的独立谱系条目**——本报告
   建议结晶为新谱系条目（候选编号 536），由 genealogist + 编排者裁定。
6. **影响声明**：新建本报告 1 个文件。**未改动任何代码**（`buysellpoint.rs` 零改动）。
   未跑回测（无"修正前后 BSP 对照"——因字面修正未实施；§3 给出已落地 B2 的 type1 1→374/
   type2 0→360 作为相关对照）。触发矛盾上浮（§6）。

**认识论等级**：**L0**（定义溯源 + 谱系核对，零数据）。§3 引用的 B2 对照数字承自前序 L2
（`project_c_segment_fix_b2`），本报告未独立复跑。

---

# 第二部分：编排者裁决后的实装（2026-06-13）

> 编排者裁决（重构矛盾，非 A/B/C 三选）：**两个都做**。并纠正本报告一个盲点——
> a₀=线段让 segment=次级别走势在**形式**上成立，但 **segment 的生成 ≠ 走势完成**；
> 一段线段可能 `settled=false`（生长中），用未 settle 的线段判 BSP = 伪信号。代码里
> Segment 有 confirmed 字段但 **BSP 判定从不读它**（SegView 丢弃了它）。
>
> 三层架构（同一/差异在递归两层各自处理）：
> - **Level 0**（现 level_id=1 段 BSP）：confirmed 加 `Segment.confirmed`（=走势已完成，
>   第65课）合取门——candidate（同一的差异侧）。
> - **Level 1+**（递归层 level≥2）：补 BSP 检测，次级别走势=完成的 Move（严格走势类型）
>   ——confirmed（差异的消解）。
> - 两层经**区间套**关联（高级别 confirmed 用低级别 candidate 定位精确点，第27课）。

## 7. Piece 1 实装：Level-0 段 settle 合取门（已完成，L2 验证）

### 7.1 代码改动

| 文件 | 改动 |
|---|---|
| `divergence.rs` | `SegView` 加 `settled: bool` 字段（= `Segment.confirmed`，第65课线段破坏） |
| `buysellpoint.rs` | `detect_type1`/`make_type2_point`/`make_type3_point` + `build_type1/2/3_bsp` 加 `require_settled` 参数：confirmed 合取 `(!require_settled \|\| anchor_seg.settled)`；`buysellpoints_from_level` + `IncrementalBsp` 透传 |
| `segment_layers.rs` | `IncrementalSegBsp` 加 `require_settled` 字段（new 形参），透传 3 个 build 调用 |
| `orchestrator.rs` | `RecursiveOrchestrator` 加 `require_settled_subseg` 字段 + new 形参；`seg_views`/`seg_view_mirror` 填 `settled: s.confirmed`；`compute_bsps` + `inc_bsp` 透传 |
| `lib.rs` | PyO3 `_RecursiveOrchestrator.new` 加 `require_settled_subseg=false` 形参；批量入口透传 false |
| 其余 SegView 构造点 | bi 级（笔全 confirmed→settled=true）、c_segment_verify、批量入口全部补字段 |

**消融控制非兼容垫片**：默认 `false` ⟹ 在册口径零漂移（bit-exact Python 移植契约保留 +
全量差分测试 full==incremental 不变）。这是 L2 消融控制（按 `sublevel_confirmation` 既定
方法论 + SCm 确认滞后风险，先测后定默认），非保留已知错误代码——若 L3 净正则翻默认。

### 7.2 验证

- `cargo test --release`：**301 passed / 0 failed / 7 ignored**（含 2 个新 settle 门单测
  type1/type3 + 既有全量差分 full==incremental 不变 ⟹ 默认 off 零漂移）。
- 新单测 `require_settled_tests`：构造未 settle anchor 段，断言 require_settled 翻 confirmed
  false、settle 后放行。

### 7.3 OKLO 447K 流式消融（L2，`data_cache/require_settled_OKLO.json`）

**关键方法论修正**：settle 门在**终态快照**上是 no-op（终态只剩 settled 存活者，门对其
恒真）——伪信号是**流式瞬态**（§3 pending 生长期伪背驰，C 段走完即反悔消失）。交易层逐
bar 消费 `current_buysellpoints()`，瞬态伪信号在流中被消费。故测量口径 = 流式逐 bar
首次 confirmed 事件键 (kind,side,seg_idx) 去重（复刻交易层 dedup），bsp_epoch 门控扫描。

| type | confirmed 事件(base) | confirmed 事件(gated) | 伪信号(生长段瞬态) | 占比 |
|---|---|---|---|---|
| type1 | 374 | 356 | **18** | 4.8% |
| type2 | 189 | 133 | **56** | **29.6%** |
| type3 | 432 | 432 | 0 | 0.0% |
| **合计** | **995** | **921** | **74** | **7.4%** |

**结构性发现（三 type 的差异化反应）**：
- **type2 受影响最大（29.6%）**：近 1/3 的 type2 confirmed 落在生长中的回试段上——
  "回试走势没完成就判"是 type2 的主要伪信号源（编排者论点的实证）。
- **type3 完全免疫（0%）**：type3 的 confirmed 判据本就是"回抽后存在延续段"，有延续段
  ⟹ 回抽段非末段 ⟹ 必已 settle。**type3 判据自带 settle 保护**（结构冗余）。
- **type1（4.8%）**：B2 修了 settled 侧后残留的 18 个 pending 生长期伪背驰（§3 的尾巴）。

**认识论等级 L2**（OKLO 447K 单标的真实数据，流式口径，可否证）。**信息增量**：门非 no-op
（终态假象已排除），识别出 74/995 流式伪信号，且伪信号分布**非均匀**（type2≫type1≫type3=0）
——这是新的结构知识（type3 判据自带 settle 保护，type2 是伪信号主域）。

### 7.4 待 L3：交易层 PnL 影响（SCm 预注册否证判据）

Piece 1 单独的交易影响**未测且不应单独裁决**——编排者的区间套架构要求两层并存
（Level-0 candidate 由 Level-1+ confirmed 定位），孤立测 Level-0 settle 门 = 测半个架构。
**预注册 SCm 否证判据**（承 `sublevel_confirmation` §4）：settle 门移除 type2 confirmed 事件
= 延迟/减少卖侧信号，若在强趋势标的（OKLO）上传导为出场延迟，可能重演 SCm −1189pp。
故 type2 的 29.6% 移除必须在**完整区间套**（Piece 2）下评估，不在 Piece 1 孤立评估。

## 8. Piece 2 计划：递归层 BSP（level≥2）+ 区间套关联

### 8.1 关键发现：递归层 BSP 适配器**已存在**

`c_segment_verify.rs::level_bsps` 已实现递归层 N≥2 的 BSP 计算（`per_level_bsp.py` 的 Rust
复刻）：次级别走势 = 下级 Move（i0=first_seg_s0/i1=last_seg_s1，**component-index 跨度作
duration** ——递归层力度 duration 语义已在适配器中解决，521号拓扑代理），中枢=LevelZhongshu，
df_macd=None。已在 ignored 测试 `c_segment_oklo_447k_per_level_bsp` 中跑通（ladder4 出现
type1，旧恒空）。

### 8.2 Piece 2 缺口（待实装）

1. **接线到 orchestrator 活态输出**：`LevelSnapshot` 当前只有 `{level_id, zhongshus, moves}`
   ——加 `buysellpoints` 字段，`LevelEngine.process` 调用 `level_bsps` 等价逻辑（移到 level
   核或 buysellpoint 层，从 c_segment_verify 提升为生产路径）。
2. **区间套 candidate↔confirmed 关联机制**（编排者新架构元素，机制未完全指定）：高级别
   confirmed BSP「用低级别 candidate 定位精确点」的具体接口——**这是 Piece 2 的设计分叉**，
   需在实装前定清（候选：高级别 BSP 的 bar_idx 重定位到其 zs 范围内的低级别同向 candidate
   的极值点；对应第27课"从大级别往下逐级收缩到精确买点"）。
3. **交易层消费**：trading 层从消费 level-1 confirmed 改为消费 level≥2 confirmed（信号）+
   level-1 candidate（定位），需与 `sublevel_confirmation` 的"等待型/信号型"二分对账。

### 8.3 Piece 2 预注册判据

- level-2 中枢稀疏（OKLO L2 settle ~100–200）⟹ level≥2 confirmed BSP 极稀 ⟹ 若交易层只
  消费 level≥2 confirmed，信号近 buy-hold（`backtest_benchmark_falsifiability` 不可证伪陷阱）
  ——故区间套必须 candidate（密）+ confirmed（稀）联合，非单用 confirmed。
- 与 SCm 对账：level≥2 confirmed 作信号型还是区间套定位型？若信号型（等 level-2 走势完成
  再动）⟹ 先验否证方向（确认滞后）；若定位型（candidate 已触发，confirmed 只精化点位）
  ⟹ 等待型，安全。**这是 Piece 2 成败的概念判据，须预注册后再实装。**

**影响声明（第二部分）**：改动 7 个 Rust 文件（§7.1）+ 2 个新单测 + 1 个 OKLO 消融测试 +
`require_settled_OKLO.json`。默认 off ⟹ 在册回测零漂移。**认识论等级**：Piece 1 = L2（OKLO 流式消融）。

## 9. Piece 2 Step A+B 实装：递归层 BSP 接入活态输出（已完成，L2）

编排者裁决 Piece 2 消费机制 = **定位型**（第27课，安全，匹配 sublevel_confirmation 二分）。

### 9.1 Step A：递归层 BSP 接线到 orchestrator 活态输出

| 文件 | 改动 |
|---|---|
| `orchestrator.rs` | `LevelSnapshot` 加 `buysellpoints` 字段；`level_buysellpoints` 生产函数（提升自 `c_segment_verify::level_bsps`）；`LevelEngine` 加 `prev_buysellpoints` 缓存字段，`process` 逐 bar 计算（短路键缓存）。次级别走势 = `in_moves`（下级 Move=严格走势类型），SegView.settled = Move.settled，递归层不开 require_settled（strictness 来自级别本身=Level 1+ confirmed 侧） |

### 9.2 Step B：OKLO 447K 稀疏度 + 生产路径等价守卫（L2）

**生产路径等价守卫通过**：`orch.recursive()[i].buysellpoints`（生产路径）逐位等价于
`level_bsps` 适配器（kind/side/seg_idx/confirmed/price bit-exact）——提升为生产路径无行为漂移。

| 级别 | n_moves | n_zhongshus | confirmed BSP（t1/t2/t3） |
|---|---|---|---|
| ladder3 (L1) | — | — | ~995 流式（密） |
| ladder4 (L2) | 15 | 47 | 43（8/8/27） |
| ladder5 (L3) | 1 | 3 | 4（1/1/2） |

**稀疏度实证（验证区间套必要性）**：Level-2+ confirmed 极稀（47 vs Level-1 ~995）。
若交易层只消费 Level-2+ confirmed ⟹ 信号近 buy-hold（`backtest_benchmark_falsifiability`
不可证伪陷阱）。**故区间套必须 candidate(密,Level-0) 触发 + confirmed(稀,Level-1+) 定位
联合**——第27课"逐级收缩到精确点"的数据依据，定位型消费的实证支撑。

**认识论等级 L2**（OKLO 447K，递归层 BSP 稀疏度真实数据 + 生产路径等价差分守卫）。

### 9.3 Step C（待实装）：区间套定位接口 + 交易层定位型消费

引擎侧 Piece 2 已完成（Level-0 candidate 经 `orch.buysellpoints()` + Level-1+ confirmed 经
`orch.recursive()[i].buysellpoints` 均可访问）。剩余为**消费侧**（交易层增量）：

1. **PyO3 暴露**：`orch.recursive()` 的 Python 快照加 buysellpoints（交易层读取）。
2. **区间套定位接口**：给定 Level-k confirmed BSP，返回其范围内同向 Level-0 candidate 的极值
   定位点（第27课"从大级别往下精确找…区间不断缩小"）。这是定位型的核心接口。
3. **交易层定位型消费**：Level-0 candidate（密）触发方向/入场，Level-1+ confirmed（稀）精化
   点位——非"等 confirmed 再动"（信号型=SCm 否证方向），而是"candidate 已动，confirmed 定位"。
4. **交易回测 + SCm 预注册判据**：完整区间套下评估 Piece 1 的 type2 29.6% 移除是否传导为
   净正（定位型应规避 SCm 出场延迟，但须 L3 实证）。

**影响声明（§9）**：改动 `orchestrator.rs`（LevelSnapshot + LevelEngine + level_buysellpoints）
+ 1 个递归层稀疏度/等价守卫测试 + `recursive_bsp_OKLO.json`。引擎活态输出新增递归层 BSP，
默认配置（require_settled_subseg=false）下 level-1 输出零漂移（递归层 BSP 是**新增**字段，
不改既有 zhongshus/moves）。Step C（交易层）未实装。**认识论等级**：Step A+B = L2。
