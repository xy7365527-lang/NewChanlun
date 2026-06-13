# Piece 2C：递归层 BSP 暴露 + 区间套定位型消费 + settle 门修正 BSP 重跑 NRF v4

> 任务（2026-06-13 编排者）：三个子步骤——① 递归层 BSP 暴露到交易层；
> ② 区间套定位型消费（Level-0 candidate 触发 → Level-1+ confirmed 精化）；
> ③ 在修正后 BSP（settle 门 on + 递归层定位）上重跑 NRF v4，看 CL 清仓是否
> 自然增加、OKLO 是否保持。判据：P1 ≥ v4 的 3/8，理想 8/8。
>
> **认识论等级：L3**（八标的真实数据交叉验证，否定性结果）。

---

## 0. 判决（先行）

| 子命题 | 结论 | 依据 |
|---|---|---|
| Step 1（递归层 BSP 暴露） | **已存在，无需新代码** | 信号层 `_level_bsps_with_divs` 早已把 ladder≥4 递归层 BSP 喂入 `bsp_events`；Piece 2A/B 已将生产路径接到 `LevelSnapshot.buysellpoints`（等价守卫通过） |
| Step 2（区间套定位型消费） | **已存在，无需新代码** | `nested_fugue.rs` 的 `nest_sell/nest_buy` 窗口（candidate 武装→confirmed 定位→deep_fire）即定位型消费；v4/v5 都有 |
| Step 3 唯一实质增量 | **settle 门开启**（信号层 `require_settled=True`） | `compute_organic_signals` 此前构造 orch 未传 require_settled |
| 交易基座 | **回退到 v4**（编排者裁决） | v5 清仓修复 L3 否证（`nrf_v5_clearance_fix.md`），v4 最优基座结算；`git checkout 5e761a5ad5 -- nested_fugue.rs` |
| **P1（任务判据）** | **3/8 = v4-off 3/8（零净增）** | 正域 {OKLO,QQQ,GC} 与 v4 完全相同 |
| **G1（CL 清仓自然增加）** | **否证** | CL sellpt 3→3 未解冻，strat +2.7%→+2.5% 未改善 |
| **G2（OKLO 保持）** | **通过且超预期** | sellpt 3→4 不过度，strat +695%→**+833%（Δ+137.9pp）** |
| **G3（SCm 确认滞后）** | **BTC 触发** | BTC Δ−202.6pp（<−100）= settle 门在超强牛重演确认滞后 |
| 核心结构发现 | **settle 门是 regime 函数** | OKLO +137.9 vs BTC −202.6 方向相反，由清仓敏感度（sellpt 变化）判别 |

**一句话**：settle 门作为 NRF v4 的信号修正，**P1 零净增（3/8=3/8）**，正域 {OKLO,QQQ,GC} 不变；
它是 regime 函数（清仓不敏感强趋势 OKLO 受益 +138pp，清仓敏感超强牛 BTC 受损 −203pp），
不是普适修正；任务"CL 清仓自然增加"的核心假设被 G1 否证。**v4（settle off）仍是最优基座**，
settle 门的唯一明确价值 = OKLO 标的的信号质量提升。

---

## 1. 实装（三 Step 实际状态）

### 1.1 Step 1/2 已存在的核实

任务卡假设递归层 BSP「暴露」和区间套「定位型消费」是待实装的缺口，核码后发现**两者均已存在**：

- **递归层 BSP（Step 1）**：信号层 `organic_signals.compute_organic_signals` 的 ladder≥4
  分支（L399-443）调 `_level_bsps_with_divs(prev, zhs, mvs, lid)`，把递归层 N≥2 的 confirmed
  BSP 喂入 `bsp_events[ladder]`。NRF 经 `bsp_events` 一直在消费递归层 BSP。Piece 2A/B 把引擎
  内生产路径（`LevelSnapshot.buysellpoints`）接通并等价守卫，但信号层消费路径未变（两者
  bit-exact）。**无需新 PyO3 暴露**——`current_recursive()` 三元组 + Python `_level_bsps_with_divs`
  已是生产消费路径。
- **区间套定位型消费（Step 2）**：`nested_fugue.rs` 的 `nest_sell/nest_buy: [Option<Win>; MAX_LADDER]`
  窗口（L339-435）即第27课「从大级别往下精确找买点」的定位型实装：candidate@k 武装窗口
  → 次级别证据下探触发 nf（located）→ confirmed 定位/破极值否定。`deep_fires` 计数即递归
  下探（第14环）。**定位型 ≠ 信号型**：candidate 已触发方向（武装），confirmed/located 只精化
  点位，不等高级别走势完成（规避 SCm 否证）。

### 1.2 唯一实质增量：settle 门接通信号层

| 文件 | 改动 |
|---|---|
| `organic_signals.py` | `compute_organic_signals` 加 `require_settled: bool = False` 参数（默认 off=在册零漂移）；构造 `RecursiveOrchestrator(require_settled_subseg=require_settled)`；docstring 标注有效域（仅 ladder3 受影响） |
| `nrf_v4_settle_gate_backtest.py` | 新建（实验臂）：`compute_organic_signals(require_settled=True)` + 在册 `nrf_v4_<SYM>.json`（settle off）对照 + 预注册 P1/G1/G2/G3/B1 |

**有效域（formalization-validity-domain）**：settle 门仅作用 ladder3（走势级段 BSP）——ladder2
笔全 confirmed ⟹ 门恒真无效；ladder≥4 递归层不开门（严格走势类型本就来自级别）。

### 1.3 交易基座回退 v4（编排者裁决）

当前 `run_nested_fugue`（mode="nrf"）在 commit 79dd56c0a8 为 v5（清仓参照系修复，已 L3 否证：
OKLO +695%→+52.6%、BTC +970%→+31.8% 过度清仓踏空）。编排者裁决回退 v4。
`git checkout 5e761a5ad5 -- rust/src/trading/nested_fugue.rs`（v4→v5 唯一 diff = C 清仓块，
75 insertions/18 deletions 精确反向）。C 清仓恢复为 `sell1@top ∧ located_sell[top]`（棘轮层合取，
非 v5 的 located 活跃层 top* + 区间套归零）。`cargo test nested_fugue`：10 passed（含恢复的
`liquidation_only_at_top_emergence_perfection`）。**no-patch**：删除否证的 v5 逻辑，不保留为 fallback。

### 1.4 settle 门生效证据（B1 指纹守卫）

| SYM | bsp fp (off→on) | Δbsp | div/flips/tflips |
|---|---|---|---|
| OKLO | 10706→10856 | +150 | 不变 |
| BTC | 118412→119932 | +1520 | 不变 |
| CL | 126489→128094 | +1605 | 不变 |

settle 门把生长期伪 confirmed 降级为 candidate（去重键含 confirmed ⟹ 新键入流），仅 BSP 事件
增加，背驰/方向流不变——精确命中 ladder3 段 BSP 的 candidate/confirmed 区分，无副作用。

---

## 2. 八标的对照表（settle on vs v4 off vs BH，L3）

| SYM | on % | off %(v4) | BH % | Δ(on−off) pp | P1 on | P1 off | sellpt on(off) | MDD on(off) | deep_on |
|-----|-----:|-----:|-----:|-----:|:---:|:---:|:---:|---:|---:|
| **OKLO** | **+833.0** | +695.1 | +307.1 | **+137.9** | ✓ | ✓ | 4 (3) | −58.4 (−55.2) | 295 |
| QQQ | +199.9 | +195.9 | +174.6 | +4.0 | ✓ | ✓ | 8 (5) | −15.7 | 499 |
| BRN | +44.2 | +40.7 | +87.4 | +3.5 | ✗ | ✗ | 6 (4) | −66.8 | 1324 |
| DX | +3.4 | +3.9 | +4.1 | −0.5 | ✗ | ✗ | 9 (7) | −15.6 | 1048 |
| ES | +534.1 | +547.8 | +594.3 | −13.7 | ✗ | ✗ | 9 (6) | −34.2 | 2519 |
| GC | +297.5 | +297.9 | +257.3 | −0.4 | ✓ | ✓ | 10 (6) | −39.8 | 3166 |
| CL | +2.5 | +2.7 | +28.2 | −0.2 | ✗ | ✗ | 3 (3) | −87.0 | 3000 |
| **BTC** | **+767.7** | +970.3 | +1380.4 | **−202.6** | ✗ | ✗ | 6 (3) | −82.5 | 2777 |

**P1 settle-on = 3/8（{OKLO,QQQ,GC}）= P1 v4-off 3/8——正域完全相同，零净增。**

Δ 符号分布：
- **改善 {OKLO+137.9, QQQ+4.0, BRN+3.5}**（幅度集中 OKLO）
- **恶化 {DX−0.5, ES−13.7, GC−0.4, CL−0.2, BTC−202.6}**（幅度集中 BTC）

---

## 3. 判据裁决

### P1（任务判据）：3/8 = v4-off 3/8，**达成下限但零净增**

任务判据「P1 ≥ v4 的 3/8」**达成**（=3/8，不低于），「理想 8/8」**未达成**。正域 {OKLO,QQQ,GC}
与 v4 settle off 逐字相同——settle 门未改变任何标的的 P1 真假，只在正域内重分布 alpha。

### G1（CL 清仓自然增加）：**否证**

任务核心假设「settle 门去伪信号 ⇒ 清仓判据用更干净信号 ⇒ CL 油崩域该清多清」。实测
**CL sellpt 3→3 未解冻，strat +2.7%→+2.5% 未改善**。机制：CL 的伪 confirmed 降级量最大
（Δbsp +1605），但这些 candidate（type2/type3）不在清仓判据路径上——清仓需 `sell1@top`
（type1 背驰），type1 settle 门只移除 4.8%，几乎不影响。**settle 门动的是 type2/type3，清仓
靠 type1，两者不交**——CL 清仓太少（+2.7% 跑输 BH +28.2%）的病根在 type1 背驰频率，settle 门
治不了。

### G2（OKLO 保持）：**通过且超预期**

OKLO sellpt 3→4（不过度清仓，对比 v5 的 27 次），strat **+695%→+833%（Δ+137.9pp）**，
not_collapsed。settle 门去伪信号（type2 29.6%）**改善**了 OKLO 短差质量——清仓几乎不变
（敏感度低），去伪信号纯增益。这是 settle 门的唯一大幅正贡献。

### G3（SCm 确认滞后）：**BTC 触发**

强趋势标的逐项：OKLO Δ+137.9（not replay ✓）、QQQ Δ+4.0（✓）、ES Δ−13.7（>−100，✓）、
**BTC Δ−202.6（<−100，scm_replay=True）✗**。BTC sellpt 3→6 翻倍——settle 门改变 candidate
分布，经 located/nest 传导使清仓增加，在超强牛市（BH +1380%）多清=踏空。这是 SCm −1189pp
确认滞后机制在 BTC 的复现：定位型消费在 OKLO 规避了延迟，但在 BTC 上 settle 门反而**制造**了
清仓（非延迟出场，是错误出场）。

### B1（指纹守卫）：**通过**

settle-on fp 记录入 json（§1.4），div/flips/tflips 不变，bsp 增量 = candidate 入流，符合预期。

---

## 4. 核心结构发现：settle 门是 regime 函数

**OKLO +137.9pp vs BTC −202.6pp 方向相反**，判别量 = **清仓敏感度**（settle 门引起的 sellpt 变化）：

| 标的 | Δbsp | sellpt off→on | settle 门效应 | Δ pp |
|---|---|---|---|---|
| OKLO | +150 | 3→4（+1，近不变） | 去伪信号改善短差 | +137.9 |
| CL | +1605 | 3→3（不变） | candidate 不入清仓路径 | −0.2 |
| BTC | +1520 | 3→6（×2 翻倍） | 清仓增加→超强牛踏空 | −202.6 |

同样量级的 candidate 入流（CL +1605 ≈ BTC +1520），但对清仓频率的传导方向不一致——CL 不动、
BTC 翻倍。**清仓不敏感的标的（OKLO/CL）settle 门效应小或正；清仓敏感的标的（BTC）settle 门致踏空。**

这与 v5 否证**同构但更精细**：
- v5 强制清仓参照系解耦 ⟹ **全标的**清仓频繁化 ⟹ 普遍踏空（OKLO/BTC/ES/GC 同步坍塌）。
- settle 门只让**部分标的**（BTC）清仓增加 ⟹ 该标的踏空，其余微动。

两者都印证 `project_signal_layer_duality` 的诊断：**清仓敏感度是 regime 判别量**，清仓判据的
信息含量是 regime 函数（第三/N 例）。settle 门不是「清仓判据 regime 门控」（v5 否证开放轴所求），
它是另一个 regime 函数——治不了 v4 的核心病（CL 清仓太少），还在 BTC 引入新病（清仓太多）。

---

## 5. 结果包六要素

1. **结论**：settle 门（信号层 `require_settled=True`，仅作用 ladder3 走势级段 BSP）作为 NRF v4
   信号修正，**P1 3/8 = v4-off 3/8**（正域 {OKLO,QQQ,GC} 逐字相同，零净增）；是 **regime 函数**
   （OKLO +137.9pp vs BTC −202.6pp 方向相反，清仓敏感度判别）；任务「CL 清仓自然增加」核心
   假设 **G1 否证**（sellpt 3→3）；**G3 BTC 触发** SCm 确认滞后（Δ−202.6pp，清仓 3→6 翻倍踏空）。
   v4（settle off）仍最优基座，settle 门唯一明确价值 = OKLO +137.9pp。Step 1/2（递归层 BSP 暴露 +
   定位型消费）核码后确认**已存在**，唯一实质增量 = settle 门。
2. **定义依据**：第65课线段破坏（settle = `Segment.confirmed`，settle 门前提）；第27课区间套
   「从大级别往下精确找买点」（定位型消费 = nest 窗口 candidate→confirmed）；第53课三类买卖点
   （type1 背驰=清仓词汇，type2/3=回试/回抽，settle 门只动后者）；`docs/nested_fugue_accounting.md`
   §6 清仓（sell1@top∧located）/§9（其他卖点走 E）。代码依据：`organic_signals.py:239`（orch 构造）、
   `nested_fugue.rs:459-466`（v4 C 清仓块）、`orchestrator.rs:946-975`（settle 门透传）。
3. **边界条件（结论翻转）**：(a) 若某标的清仓判据从 type1 背驰扩展到 type2/type3（settle 门作用面），
   则 settle 门可能传导到清仓频率，G1 可能翻转——但这需改清仓词汇（重开 §6）；(b) 若区间套定位
   接口改为「Level-1+ confirmed 强制等待」（信号型而非定位型），则 OKLO 的 +137.9 会翻转为 SCm 否证
   （定位型是 OKLO 不坍塌的前提）；(c) 若 BTC 的 candidate→清仓传导链被切断（settle 门不影响 BTC
   located），则 BTC Δ−202.6 翻转——这是 settle 门 regime 依赖的物理根因待精化轴。
4. **下游推论**：(a) settle 门**不是** v5 否证开放轴所求的「清仓判据 regime 门控」——它治不了 CL
   清仓太少，还在 BTC 引入清仓太多；真正的清仓 regime 门控需直接作用清仓判据（type1 背驰频率/级别），
   非信号层 candidate/confirmed 区分；(b) OKLO +137.9 证明 settle 门在**清仓不敏感的强趋势标的**上是
   纯信号质量增益——可作 OKLO 类标的的白名单修正（per-asset，非全局默认）；(c) settle 门默认应保持
   off（在册零漂移），on 仅作 OKLO 白名单。
5. **谱系引用**：`project_nrf_v4_strict_accounting`（v4 最优基座 P1 3/8）；`project_nrf_v5_clearance_fix_falsified`
   （v5 否证，清仓敏感度 regime）；`project_signal_layer_duality`（清仓敏感度=regime 判别量，settle 门
   双重性）；`sublevel_confirmation_recursive.md`（SCm −1189pp 确认滞后，G3 依据）；
   `bsp_engine_sublevel_fix.md`（Piece 1/2A/2B：settle 门 7.4%/type2 29.6% 消融、递归层 BSP 稀疏度）。
   **本报告建议结晶新谱系条目**：「settle 门 = regime 函数（清仓敏感度判别）」（候选编号待 genealogist 裁定）。
6. **影响声明**：改动 2 个文件——`organic_signals.py`（加 require_settled 参数）、新建
   `nrf_v4_settle_gate_backtest.py`；回退 1 个文件 `nested_fugue.rs`（v5→v4，编排者裁决）；新增 8 个
   `nrf_v4sg_<SYM>.json` + `nrf_v4sg_summary.json`。**在册回测零漂移**（require_settled 默认 off，
   bit-exact 移植契约保留）。回退 v5 撤销 commit 79dd56c0a8 的 NRF v5 清仓修复代码（v5 已 L3 否证，
   定理执行）。**认识论等级 L3**（八标的真实数据交叉验证，否定性结果：P1 零净增 + regime 函数）。
