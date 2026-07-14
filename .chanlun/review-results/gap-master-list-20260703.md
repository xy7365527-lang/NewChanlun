# 全策略未实装总清单（gap-master-list）

- **工位**：swarm/ws-gapaudit（task #118，parent_callback: main）
- **日期**：2026-07-03 · **HEAD**：c726336fc0 · **纯只读零 git**
- **编排者令**：「你得自己先排查整个策略还有哪里没实装的」
- **方法**：合并去重既有对照报告的「部分/缺口」项 + git log/TaskList 验证 stale（大量 7-02/03 已闭合）+ 三类买卖点 P0 核心核。
- **认识论**：本清单 = L0/L1（结构存在性 + 接线核对 + 任务状态核对）。alpha 有效性判定不在本工位范围。

---

## 0. 顶层结论（先说定案）

**「整个策略还有哪里没实装」的精确答案：完整 π 状态机已实装且被回测消费；剩余未实装项收敛为 1 个真 P0（8 谓词双实装未统一）+ 3 个 P0-激活类（已落机器/默认关，待热路由/口径切换）+ 一族 P1 增强。**

编排者最关心的「三类买卖点完整工作」经核查**不缺判据**——三类判据（`bsp.rs` `is_first/second/third`）全实装；一类在 BTC 300K **零触达**已被 #116 裁定为**市场事实（定义严格性），非实装缺口**。三类买卖点方向的唯一 live P0 是**一类背驰力度判定的连续细分（β 热路由 #115，现 inert）**。

**报告去重后的三态**：合并 6 组报告共约 130 条对照项，剔除 7-02/03 已闭合的 stale 项后，剩余未实装 = **P0×4 + P1×7**（下表）。

---

## 1. 未实装总清单（合并去重 + stale 验证）

图例：状态列 `OPEN`=未实装/未接线；`MACHINE-INERT`=机器已落但默认关/恒 None；`CLOSED*`=本轮已闭合（stale，列出防重复派工）；`MARKET-FACT`=经裁定非缺口。

### 1.1 P0 —— 三类买卖点完整 + 策略闭环 well-definedness

| # | 项 | 出处报告 + 锚点 | 当前状态 | 依据 |
|---|----|----------------|---------|------|
| **P0-A** | **8 谓词全互斥解释器双实装未统一**（`mutex::mutex_class` P1..P8 无生产消费者 vs `interp::interpret` ≺_Θ 三桶被 π 消费，未证等价） | full-strategy-pi §缺口 P0-1（`interp.rs`/`strategy/mutex.rs`） | **OPEN** | 近 20 commit 无 mutex 统一/桥接提交；`mutex_class(` 仍无调用点。**唯一未动的真 P0** |
| **P0-B** | **β 背驰强度热路由激活**（一类/背驰力度连续细分；`MuClass.force_state` 恒 None inert） | beta-bucket-design v2 §6 + full-strategy-pi P1-1；**task #115 pending** | **MACHINE-INERT** | #112 已落 `ForceStateA4` 原语+第8维+prereg（commit 71f78103c7），但 `signal.rs:543` 生产入口传空 dif/closes ⟹ force 恒 None。#115 做激活（BspPoint 加 force 字段 + 增量分类器真传 dif/closes + fill-rate 断言 + δ-共线逐层门） |
| **P0-C** | **真保证金 MM 口径默认关 → alpha 需真口径重冻结**（RiskMode 五态曾退化 equity≤0） | full-strategy-pi §缺口 P0-2 + margin-model-design v2 §4；**task #113 completed** | **MACHINE (default-off)** | #113 已实装 `MarginSchedule`/`margin_inputs`/M2/M3 `no_increase_cap` 真接线（commit 1896139eac），但**默认 `margin=None` bit-exact**（保 alpha 冻结）。剩余 = margin 开启后按 §4 清单重跑重冻结所有 RunResult/L3 产物并标口径。**判定逻辑已实装，非缺口；口径切换是待执行 L2 动作** |
| **P0-D** | **多重赋格持仓账本经济验收**（ShortDiff 只在分类/统计层，账本头寸层薄；overlay Π 是否 alpha 未验） | nesting-fugue §P0-3 + f2-impl §2；**task #114 completed / #117 in_progress** | **PARTIAL（诊断已落，验收在飞）** | f2 已落 `overlay_net_delta` ‖ΔN‖_1 只读诊断（commit c2533bf1ba）；#114 f3-fugue 实测**多声部结构薄 ShortDiff 1.2%**（34b4f64016）；#117 f3c-shortdiff 反事实 P&L 差分**在飞**（f3 最后验收项）。经济有效层（Π^overlay IR）待 #117 结论 |

### 1.2 P1 —— 细分优势 / 闭环完整性增强（依赖 P0，非 well-definedness 必需）

| # | 项 | 出处 + 锚点 | 当前状态 |
|---|----|-----------|---------|
| P1-1 | 持仓节点完整 §13 四元组 `(carrier,γ,σ,generation)`（f2 选简化 §14 单 carrier 单 instance） | nesting-fugue P0-2 / f2 §1 | **DEFERRED**（加仓/分批/双开未实装前 YAGNI，doc 已声明升级触发） |
| P1-2 | frontier 未确认必重算（裁决②；增量塔 sealed prefix `b_t` 停太晚） | nesting-fugue A8 / 区间套.pdf p10-12 / memory `frontier-resume-bt-too-late` | **OPEN**（实现 bug，非定义冲突；修后 depth≥2 是否上升待 L2 重测） |
| P1-3 | 双视图 K_i carrier forest（`host^op`/`host^struct` 分离；全格 BSP 进操作解释器 Γ_i^K≅B_i） | nesting-fugue B3/B4/B5 / 子声部.pdf | **OPEN**（依赖 P1-1；`voice_eat.rs uncovered>0` 已探测 gap-B） |
| P1-4 | β 力度族补 TV 全变差 + 递归次级别力度（𝒜₄→𝒜_ℓ，ForceStateA4→ForceState） | beta-bucket-design v2 §2.3 诚实缺口 | **DEFERRED**（`ForceStateA4` 是宽近似，升级路径已声明） |
| P1-5 | 保证金 SPAN 全场景 / portfolio margin / ADL·funding 强平 / 借券费 | margin-model-design v2 §1.3 诚实清单 | **DEFERRED**（BTC perp 下 ADL/funding 缺口不 binding；股票短卖借券费扩标的时必补） |
| P1-6 | H 轴（Horizontal）进 default selection（现只在 canonical Z，selection 保 bool） | z-bucket §2.3 / codex #81 `conditional` | **BY-DESIGN**（抗 winner's curse，OOS-gated 才升，非缺口） |
| P1-7 | 显式 L1/L2 正则化回归（现 μ̂=分桶均值+shrinkage） | full-strategy-pi §12 | **BY-DESIGN**（分桶+收缩是等价维数控制路径，非缺口） |

### 1.3 已闭合（stale——列出防重复派工）

| 曾报缺口 | 出处 | 闭合 commit/task |
|---------|------|-----------------|
| σ_p/role/短差 alpha 桶键未接入（回测跑 Y 粗投影） | full-mutex-b1 P0 / z-bucket | **#83 z-bucket**（`econ_positive.rs` 桶键升 `MuClass`=Z） |
| 下沉触发状态机缺失（高级别背驰→触发） | nesting-fugue P0-1 | **#108 NestTrigger 枚举**（commit f7c60de5da，三路 dispatch 显式化） |
| RiskMode 五态判定输入退化 | full-strategy-pi P0-2 | **#113 margin-impl**（判定逻辑接真实 MM 输入；见 P0-C 口径待切换） |
| 一类买卖点零触达 = 疑似门误杀/掩码吸收 | acc-level-hole / #108 | **#116 裁定 MARKET-FACT**（门前 bsp_pre_first=0 定义严格性，非 bug） |
| C3 小转大死门（level==1 same_center=0） | acc-level-hole §C3 | **codex #44/#55/#56 终局**：level==1 硬门=C2∧C3新中枢突破，lvl≥2 C2-only 真封；命中恒 0 = 市场几何事实 |
| overlay 净额可见层度量缺失 | nesting-fugue B14 | **f2 `overlay_net_delta`**（commit c2533bf1ba，见 P0-D） |

---

## 2. P0 实装路线（按依赖排序，标可并行）

**依赖关系**：P0-A（解释器统一）、P0-B（β 热路由）、P0-C（margin 口径切换）三者**互无数据依赖 → 可并行**；P0-D（多重赋格验收）依赖 #117 在飞结论，串在其后。

```
并行波次 1（无相互依赖，立即可 spawn）:
  P0-A  interp≺_Θ ⟺ mutex P1..P8 等价证明（+bridge property test）
        └ 裁定点：若 ≺_Θ 序 ≠ 谓词优先级 ⟹ escalate（PDF 四写谓词优先级，倾向 mutex 为规范）
        └ 最小 diff：strategy/mutex.rs::bridge_from_interp + 1 property test（不改主链）
  P0-B  #115 beta-route 激活（BspPoint.force 字段 + 增量真传 dif/closes + z_of_candidate_with_force
        + dx signals_dx 同步 + perm_test fullz fill-rate 断言 + δ-共线逐层门）
        └ 护航：1415 lib + dx bit-exact（force 旁挂不改 bit 判定）
  P0-C  margin 口径切换：开 real-MM 跑一遍 → 按 margin §4 清单重冻结
        RunResult 全字段 + L3 管线，产出标「真实分级 vs MM=0」口径

串行波次 2（依赖波1/在飞）:
  P0-D  待 #117 f3c-shortdiff 反事实 P&L 差分收口 → 判 Π^overlay 经济有效层
        └ 若 ‖ΔN‖_1≈0（净额不可见）⟹ overlay 账本无意义，不建 OverlayState（先诊断后账本）
```

**优先级说明（275 局部依赖，不做全局排序）**：三波次-1 项各管各的直接依赖，同时 spawn。P0-A 是唯一「well-definedness 未闭合」项（双实装漂移风险），P0-B/P0-C 是「已落机器待激活/切口径」，P0-D 是「验收在飞」——性质不同但无先后强制。

---

## 3. 与在飞工作对齐

| 在飞任务 | 状态 | 覆盖本清单哪些项 |
|---------|------|-----------------|
| **#115 beta-route** | pending | **P0-B 完整**（β 热路由激活 = 一类背驰力度连续细分落地）+ P1-4 的前置 |
| **#116 type1-zero-probe** | completed | 关闭「一类零触达」P0 疑云（裁定 MARKET-FACT，非缺口）——三类买卖点判据完整性已确认 |
| **#117 f3c-shortdiff（=f3-C）** | in_progress | **P0-D 经济有效层**（ShortDiff 反事实差分 → overlay 是否 alpha） |
| #113 margin-impl | completed | P0-C 判定逻辑侧（口径切换是残余 L2 动作） |
| #114 f3-fugue | completed | P0-D 诊断侧（多声部结构薄 1.2% 已实测） |

**未被任何在飞任务覆盖的 P0 = P0-A（8 谓词双实装统一）**——需新派工位。

---

## 4. 结果包六要素

1. **结论**：完整 π 已实装被回测消费；未实装项去重后 = P0-A（解释器双实装未统一，OPEN，唯一未派工真 P0）+ P0-B（β 热路由 inert，#115 覆盖）+ P0-C（margin 默认关，口径待切换）+ P0-D（多重赋格验收 #117 在飞）+ P1×7（多为 by-design/deferred）。三类买卖点判据完整，一类零触达经裁定为市场事实。
2. **定义依据**：逐项引报告出处 + 代码锚点 + git commit/task 状态；三态按「构造存在∧被消费」判。stale 验证用 `git log --oneline -20` + TaskList。
3. **边界条件**：P0-A 若 ≺_Θ 序被证 = P1..P8 谓词优先级则从「双实装」降为「已合规仅需见证」，若分叉则升「合规矛盾」走 escalate；P0-C 若不切真 MM 口径则所有 alpha 数字停在 MM=0 口径（务实思维 161 号禁止长期混用）；P0-B 若 OOS 证 ForceStateA4 无 μ 增益则从桶键删维。
4. **下游推论**：P0-A 需新工位（无在飞覆盖）；P0-B/C/D 分别由 #115/margin 口径动作/#117 承载。波次-1 三项可并行 spawn。
5. **谱系引用**：full-strategy-pi-conformance / nesting-fugue-conformance / z-bucket-impl / full-mutex-b1-design / margin-model-design v2 / beta-bucket-design v2 / f2-impl / type1-zero-probe / acc-classification-level-hole；memory `project_interval_nesting_not_called_in_backtest`、`project_gap3_l2_unreachable_architecture`、`frontier-resume-bt-too-late`、`project_oddeven_mu_identity`、`project_iclass_delta_collinearity_perm_degeneracy`。
6. **影响声明**：纯只读零 git，产出本清单一份。下游：Lead 按 §2 波次派工 P0-A（新工位）；#115/#117 继续在飞；margin 口径切换排 L2 动作。未改任何代码/定义/谱系。
</content>
</invoke>
