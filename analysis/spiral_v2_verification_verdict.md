# 螺旋引擎 v2 真实数据验证裁决（Step 6+7）

> 脚本：`trading_system/backtest_spiral_stream.py`（spiral 真流式 + unn 同 sig 流差异对照）
> 对账：`trading_system/compare_spiral_unn.py`（CL 全量 bit-exact 已独立复核）
> 数据：`trading_system/data_cache/`（8 标的 1min OHLC，全量）

## Part 1：bit-exact 对照（L2 表达力验证）—— ✅ 通过

CL 全量 5,528,156 bar，单一 `StreamingSignalReader` 逐 bar 产出 sig，同一 sig 同时喂
`UnnStream` 与 `SpiralStream`：

- 逐 bar 吐单分歧：**0 bar**
- trade 表逐行：**完全相等**（15 笔）
- final_nav：unn=95252.990325，spiral=95252.990325（**逐位等**）

判决：**操作 = 群作用** 的 L2 验证通过。群作用抽象（C 走 `GroupAction::ChiralSeam`、
E spawn 走 `σ⁻¹∘τ`、closure 走 `Δr=−1`）足以表达 unn 全部已验证操作，A 强平非群
（gap G2）经类型系统显形（`Operation::Liquidate.group_action()` 返 None）。

## Part 2：全量 8 标的 spiral 回测（L3）

| 标的 | bars | spiral% | BH% | MDD% | BH_MDD% | P1 | cross_lvl(Δr=−1) | prove panic |
|------|------|---------|-----|------|---------|-----|------|------|
| BTC  | 4.6M | +136.9 | +1380.4 | −76.5 | −84.0 | fail | 115 | 0 |
| CL   | 5.5M | −4.7   | +28.2   | −5.4  | −94.0 | fail | 14  | 0 |
| ES   | 5.6M | +218.2 | +594.3  | −20.8 | −36.0 | fail | 24  | 0 |
| OKLO | 447K | +256.8 | +307.1  | −74.6 | −76.8 | fail | 37  | 0 |
| QQQ  | 728K | +176.9 | +174.6  | −23.3 | −25.6 | **PASS** | 3 | 0 |
| GC   | 5.5M | +227.7 | +257.3  | −39.8 | −45.6 | fail | 8  | 0 |
| DX   | 2.0M | +6.0   | +4.1    | −14.3 | −16.8 | **PASS** | 2 | 0 |
| BRN  | 2.4M | −28.6  | +87.4   | −61.2 | −78.8 | fail | 25 | 0 |

- **P1（strat ≥ BH）：2/8 {QQQ, DX}**——与 unn 逐位相同（bit-exact 推论）。
- **MDD：8/8 全优于 BH_MDD**（与 unn 相同；027 否定线支配保证金线的复现）。
- **prove panic：8/8 全 0**——N1–N8/群关系/cross_level 每 bar panic 守卫，全程无违反
  ⇒ 必然性在群作用驱动下成立（L2）。

## Part 3：P-close（H¹ 闭合）效果验证 —— 差异 = 0（结构性恒等）

| 标的 | strat Δ(spiral−unn) | 强平 spiral=unn | Δ强平 | 空头 wins spiral=unn | 空头 pnl spiral=unn |
|------|------|------|------|------|------|
| BTC  | 0.0 | 89=89 | **0** | 9=9 | −206757=−206757 |
| CL   | 0.0 | 0=0   | 0 | 4=4 | −22474=−22474 |
| ES   | 0.0 | 22=22 | 0 | 3=3 | −66793=−66793 |
| OKLO | 0.0 | 36=36 | 0 | 0=0 | −98431=−98431 |
| QQQ  | 0.0 | 0=0   | 0 | 0=0 | −151=−151 |
| GC   | 0.0 | 0=0   | 0 | 4=4 | +4332=+4332 |
| DX   | 0.0 | 0=0   | 0 | 2=2 | +1874=+1874 |
| BRN  | 0.0 | 14=14 | 0 | 7=7 | −106266=−106266 |

### 任务预期 vs 实测（直面，no-workaround）

任务 Part 3 预期：「v2 的核心改进是 H¹ 闭合……BTC 的 E spawn 子空头是否不再全亏
（wins>0？）……强平是否减少/消失」。

**实测全部否证**：
- BTC E spawn 子空头：spiral wins=9 = unn wins=9，pnl −206757 逐位相同——**P-close 未改善**。
- BTC 强平：spiral=89 = unn=89，Δ=0——**P-close 未减少/消失**。
- 全 8 标的：spiral 与 unn 逐位 bit-exact，strat Δ=0。

### 为何必然为 0（定理，非偶然）

这不是 P-close 失效，而是**任务前提与已验证设计互斥**：

1. spiral 的设计目标（架构 §2 + mod.rs §2）= **bit-exact 复现 unn**，作为「操作=群作用」
   的 L2 否证测试。spiral **复用 unn 的 `CenterBook`/`DepthRef` 成本门与会计**，群作用层
   （ChiralSeam/σ⁻¹∘τ/Δr=−1）是**类型化路由**，不改变操作的执行落点。
2. closure.rs 的 `Δr=−1`（P-close）是 unn **既有**跨级别闭合（N5⊥N7：confirm fire 两路
   消费——级联 located→根 F/C / 逐层 nf→任意 voice E 降成本）的**群论 re-description**，
   不是新机制。`cross_level_closures` 计数 = Δr=−1 兑现次数，是 spiral 唯一新增**可观测量**，
   但它**只观测、不干预**。
3. ∴ **bit-exact ⟺ P-close 零 P&L 增量**。两者不能同真：
   - 若 spiral bit-exact（已验证 8/8）⟹ Part 3 差异恒 0（本结果）。
   - 若 P-close 真改闭合落点 ⟹ spiral 必 **不** bit-exact ⟹ Part 1 必失败。

这正是「扬弃」（089 号）的形态：否定（不再是 unn 之外的特权机制）+ 保留（操作落点不变）
+ 提升（操作的存在方式从 F/C/D/E 函数变为群作用类型）。**表达力 L2，非 alpha L3**
（与既有谱系 `project_spiral_engine_v2_bitexact` 一致）。

## 结果包六要素

1. **结论**：spiral v2 在 8 标的全量真实数据上与 unn **逐位 bit-exact**（trade/nav/强平/空头胜负
   全等），prove panic 全 0。Part 1（L2 表达力）✅、Part 2（L3 回测表）✅、Part 3（P-close 差异）
   = **结构性 0**。
2. **定义依据**：架构 §2.4 判据2「操作=群作用」L2 否证测试 = bit-exact；closure.rs §6
   三类闭合（CrossLevel/ValidityDomain/Category），Δr=−1 仅对 CrossLevel 兑现且为
   unn 既有跨级别闭合的 re-description。
3. **边界条件**：结论在「spiral 复用 unn CenterBook/DepthRef 且群作用层为类型化路由」前提下
   成立。若改 closure.rs 使 Δr=−1 **改变操作落点**（如子空头 spawn 落到 r=k−1 而非 unn 的
   原级别），bit-exact 翻转为 MISMATCH，Part 3 才可能产生非 0 差异——但那是**新设计**，非本次验收对象。
4. **下游推论**：unn 可作为 spiral 的永久 bit-exact 对照基线保留（mod.rs §2 已声明）。
   spiral 的价值是**表达力/可证性**（群关系 panic 守卫 + A∉M 编译期显形），不是 P&L 改善。
   BTC/CL/BRN 的 P1 fail + 强牛踏空是 **unn 引擎机制的有效域读数**（539 号 regime 正交），
   spiral 既 bit-exact 就完整继承，**P-close 不构成修复路径**。
5. **谱系引用**：`project_spiral_engine_v2_bitexact`（8/8 bit-exact，表达力 L2 非 alpha）、
   `project_unn_btc_spawn_throwback`（BTC E spawn 强牛过度做空亏损——本结果继承）、
   `project_spiral_closing_level_verdict`（闭合级别几何，7/8 跑不赢 BH）、089 号扬弃、
   222/223/230 号有效域≠定义域。
6. **影响声明**：新增 `trading_system/backtest_spiral_stream.py`、本报告、
   `data_cache/spiral_stream_*.json`（8 个）。无引擎代码改动（纯验证）。

## 认识论等级

- Part 1 bit-exact = **L2**（真实数据，可否证，已通过——证明表达力，非证明假设成立）。
- Part 2 回测表 = **L3**（8 标的交叉，spiral=unn ⇒ 继承 unn 的 L3 读数）。
- Part 3 差异=0 = **定理**（从 bit-exact L2 结果逻辑必然推出，信息增量 = 揭示任务前提与设计互斥）。
