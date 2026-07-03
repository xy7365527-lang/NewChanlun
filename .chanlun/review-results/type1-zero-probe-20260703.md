# 一类买卖点零触达根因探针（type1-zero-probe，task #116）

**工位**：swarm/ws-bottomup · **日期**：2026-07-03 · **触发**：#108 发现 BTC 300K 过门 756 配对信号中 buy1/sell1 = 0 · **零生产改动**（复用 `acc_classification_level_hole_dx` 计数 + bsp.rs 判据阅读）

---

## 0. 结论（三态裁定：(a) 定义严格性 = 市场事实）

一类买卖点在 BTC 300K **门前分类层就 = 0（所有 level）**——不是门误杀，是**从未诞生**。根因在 `is_first()` 的 `below_last_center` 子条件（= 未离开中枢处的 MACD 背驰确认 C<A）在 BTC 该窗从不与其余前提合流。裁定 **(a) 定义严格性 / 市场事实**，排除 (b) 门结构误杀、(c) 掩码吸收。

## 1. 漏斗测量（BTC 300K，acc_classification_level_hole_dx）

| level | tower段数 | bsp_pre(门前) | 第一类 | 第二类 | 第三类 | Γ非Flat | sig_post(门后) |
|---|---|---|---|---|---|---|---|
| 0 | 2500 | 721 | **0** | 0 | 721 | 721 | 721 |
| 1 | — | 77 | **0** | 77 | 0 | 77 | 10 |
| 2 | — | 22 | **0** | 22 | 0 | 22 | 22 |
| 3 | — | 4 | **0** | 4 | 0 | 4 | 4 |
| 4 | — | 2 | **0** | 2 | 0 | 2 | 2 |

**① 门前一类总数 = 0（全 level）**。level0 全第三类（721），level1-4 全第二类（架构 mod.rs:247「上级层只产第二类」）。
**② 无「死在门」可追踪**——一类从未进 `ls.bsp` 列表，故不存在门/配对阶段的一类死亡。#108 的配对后 0 一类是上游 0 的直接传导（717/756 配对信号是 level0，全第三类）。

## 2. 根因（③ bsp.rs 一类判据子条件）

`is_first()`（bsp.rs:56）= `below_last_center ∧ !after_first_buy ∧ !left_center`：

- **瓶颈子条件 = `below_last_center`**：由 Θ_signal 在 MACD **背驰确认**（趋势末段面积相对前同向段严格变小 C<A，divergence.rs）后才置（bsp.rs:54-55）。
- **`left_center` 互斥**：一类需 `!left_center`（未离开中枢的背驰端点），三类需 `left_center`（离开中枢回试）——二者在 `left_center` 上结构互斥（bsp.rs:57/65）。BTC L0 门前 721 条**全落 `left_center=true`（第三类）**，故一类候选域为空。
- **StructBreak 旁路佐证**（bsp.rs:115-127，P2-R2）：破最后中枢但 `C≥A`（MACD 面积未衰减）的候选 ⟹ `below_last_center=false` ⟹ **零六 bit** ⟹ `bsp_cand_type` 判 StructBreak ⟹ `build_gate_certificate` 恒 None 门拒。即**几何破中枢事件存在，但多数不满足 MACD 背驰 ⟹ 不升为一类，落零 bit 被拒**。

## 3. 三态裁定（④）

| 态 | 判定 | 依据 |
|---|---|---|
| (a) 定义严格性=市场事实 | **✓ 成立** | 一类需「未离中枢 + MACD 背驰 + 无前置一买」四条合流；BTC 300K 门前该合流 = 0（bsp_pre_first=0 上游于任何门）。第三类（离中枢回试，721 条）是 BTC 该窗主导结构 |
| (b) 门结构误杀 | ✗ 排除 | 一类死在门前（never born），非门后。门（Nest/Xzd）从不见一类输入 |
| (c) 掩码吸收/标注归并 | ✗ 排除 | `bsp_pre_first` 直读 `p.bits.buy1\|\|sell1`——buy1 bit 从未置位，非「置了但被别的 bit 掩盖」。`endpoint_to_bsp` 对同一点同一 BspBits 置位，无平行路由 |

## 4. 边界条件（结论翻转）

1. 有效域 = BTC 300K 单标的 L2。其他品种/窗若破中枢时 MACD 面积衰减（C<A）更频繁，一类可 >0。
2. 若 `below_last_center`/divergence 判据（divergence.rs 的 C<A 阈值）本身过严（bug/阈值失校），则 (a)↔(b) 边界移动——但当前证据（StructBreak 旁路是 P2-R2 设计而非 bug，几何破中枢确实产候选只是无背驰）指向 (a)。**未量化**：StructBreak（零 bit 破中枢）候选数——若远大于第三类，说明「几何破中枢频繁但背驰罕见」，进一步坐实 (a)；若≈0，说明破中枢本身罕见。这是唯一未测子项（需加门前零 bit 计数器 + 重跑）。

## 5. 谱系引用

- [[project_oddeven_mu_identity]] / #108：全信号集第三类(L0)+第二类(L1-4)，零一类——本探针给出上游根因。
- P2-R2（bsp.rs:115，p2-plan-20260701）：StructBreak 旁路——破中枢无背驰候选的零 bit 归属，本探针佐证其为一类缺席的机制侧。
- 缠师第24课「第一类=破中枢+背驰」——本探针确认实装 `is_first` 忠实该定义，零触达是定义严格性在 BTC 的后果，非实装偏差。

## 6. 影响声明

**零代码改动**。纯测量：复用 `acc_classification_level_hole_dx`（bsp_pre_first 分级计数）+ 阅读 bsp.rs `is_first`/`endpoint_to_bsp`/P2-R2 注释。仅新增本结果包。

**复算**：`ECON_L2_MAX_BARS=300000 cargo test --release --lib acc_classification_level_hole_dx -- --ignored --nocapture`（读 level 表「第一类」列 = 0）。

## 认识论等级

**L2**（真实 BTC 单标的，可产否定性结果——「一类零触达」是否定性发现）。StructBreak 计数子项未测（§4.2），有效域 = BTC 300K。
