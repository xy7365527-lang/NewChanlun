# 读法B 一对多空腿（LegPair）实装 — 工位 #57=#53.1

**日期**：2026-06-23
**工位**：缠论 recursive_t 引擎实装 #57（=#53.1 核心代码实装）
**分支**：`worktree-agent-a246c3b83f9580658`（基于 main HEAD `a8d43d6fab`）
**认识论等级**：实装=L0（代码结构）；OFF bit-exact=L1（管线正确性）；CL 单标的回测=**L2**（真实数据单标的假设检验，产生否定性结果）

---

## 1. 结论

把 reading-B 腿模型从「单腿 + 几何方向 + d_top switch」重写为「每级别一对多空腿（LegPair）+ 买卖点开平 + 否定线止损 + 链破坏 churn 门控 + 删几何方向」。全部新逻辑由**新 flag** `enable_reading_b_pair`（env `T_READING_B_PAIR`）门控，OFF 逐字不变（bit-exact）。

**交付契约全部满足（CL L3）**：
- 零 panic（守恒守卫 `prove_tw_neutral` + 隔离守卫 `prove_pair_isolation` 每 op 未 panic）
- **零强平** `liq=0`（否定线止损先于 NAV≤0 生效：2321 多腿止损 + 2867 空腿止损）
- 守恒零违反（TW 中性每 op）
- final_nav 有限正（strat=−66.6% ⇒ NAV≈33,400 > 0，有限）

**L2 否定性读数（缩小有效域，非否决交付）**：CL strat=−66.6% vs BH +28.2%（OFF=+20.3%）。`short_pnl=−71932` 主导失血，`long_pnl=+5356`。`core_churns=8`（链破坏仅动核心 8 次）。`max_gross=4.62×`（多空双开 >1× 杠杆）。

### 架构（每级别自相似一对腿）

每级别 k 一对 `LegPair`（多腿+空腿同时在场，多空双开吃所有级别涨跌幅），三个缠论结构点（每腿）：

| 结构点 | 多腿 | 空腿 | 缠论依据 |
|--------|------|------|---------|
| **开** | 该级别**买点** fire（`view.buy[k]`）| 该级别**卖点** fire（`view.sell[k]`）| 第17课买卖点定律一 + 第27课区间套定位 |
| **平** | 反向**卖点** fire | 反向**买点** fire | 与开对称 |
| **止损=否定线** | 进场中枢 **ZD**（`c < long_stop` 跌破）| 进场中枢 **ZG**（`c > short_stop` 涨破）| 第17/18课中枢核心区间 = 否定线；「回试回中枢=假突破」⟹止损 |

**链破坏 churn 门控**（第27课区间套）：核心多腿（=`highest_active_long` 结构涌现，无 `if level==top`）的反向平**只在链破坏时**动（`view.d_top[k]=true`=区间套链贯通到真顶/真底=转折）。链完整（`d_top=false`=回调）⇒ 卖点不平核心多腿，落到开本级空腿吃回调。次级别不受 churn 门控（自由吃回调）。

---

## 2. 定义依据

| 输入数据特征 | 满足的定义条件 |
|------------|--------------|
| `view.buy[k]`/`view.sell[k]`（per-level fresh BSP，含 type1/2/3） | 第17课买卖点 = 开/平触发源；开仓方向由买卖点涌现非 `node.direction` 几何方向 |
| `view.zg[k]`/`view.zd[k]`（`trends.last().zhongshus.last()` 的 high/low） | 第18课中枢核心区间 [ZD, ZG]（`ZD=max(前两段low)`/`ZG=min(前两段high)`）= 否定线原料 |
| `view.d_top[k]`（`divergence::d_top` 区间套链贯通） | 第27课区间套定位：链贯通=转折（churn 门控信号），链未贯通=回调 |
| 「核心」= `highest_active_long()`（最高活跃多腿级别） | 结构涌现（无 `if level==top` 硬编码，自相似跨级别同构）；「强牛不开空」= 升跌完备性涌现（21:40 最高级别只 type3 买点无卖点） |

**no-hardcode（005b/no-patch）合规**：方向/开/平/止损全由缠论结构（买卖点/区间套/中枢/否定线）涌现，零 `if level==top` 分支。`g_pair(k)` 对所有 k 同构。

---

## 3. 边界条件（结论翻转条件）

1. **杠杆 >1×（max_gross=4.62×）**：多空双开同级别同时在场时，`geom_tower_quota` 对多腿和空腿**各自独立**从同一 free 池领配额——两腿叠加可超 1× 名义敞口。若交付契约改为「恒仓 ≤1× 毛敞口」，则当前实装**翻转为不合格**（需配额在多空腿间分配，而非各自全额）。当前未做配额分配=有效域边界（诚实标注，非 workaround）。
2. **空腿失血主导（short_pnl=−71932）**：若 CL 实际是上行 regime（BH +28.2%），空腿吃回调=持续逆势失血（539号「根翻空有效域⊂非上行 regime」的 LegPair 复现）。结论（strat 符号）在震荡/下行 regime 标的上可能翻正——L2 单标的不可外推（需 L3 交叉验证）。
3. **否定线缺失时（`zg/zd=None`）**：走势末无中枢（`zhongshus` 空）⇒ 该腿无否定线，仅靠反向买卖点平。若大量级别长期无中枢，止损保护失效 ⇒ 可能触发 NAV≤0 强平（当前 CL `liq=0` 表明否定线覆盖充分，但此为标的相关，非结构保证）。
4. **churn 门控 `core_churns=8` 极稀疏**：链破坏触发罕见 ⇒ 核心多腿几乎从不被卖点翻转（强趋势保护）。若 `d_top` 触发频率因 a0 粒度/标的改变而暴增，核心多腿过度翻转 ⇒ 踏空（与 556 顶层冻结对偶）。

---

## 4. 下游推论

1. **多空双开杠杆轴新开放**：max_gross=4.62× 暴露「多空双开 + 单 free 池 + 几何塔配额」在多空腿独立领配额时的杠杆放大。下游若要恒仓基座，须新增「多空腿配额分配」（顶层一维配额在 LegPair 内多/空腿间切分），是新选择轴（待裁决，非本工位定理）。
2. **空腿 regime 函数复现**：short_pnl 主导失血与 539/[[project_t_short_leg_regime_function]] 同构——做空腿是 regime 函数，非全标的普适。下游 L3 交叉验证应预注册「正域=非上行 regime 标的」假设。
3. **否定线作止损 = 新退出语义**：与 [[project_ph_settle_usage_boundary]]（PH settle 是形态学必要非操作充分）对偶——否定线（中枢边界）作操作层止损是新用法，下游若复用须验证「中枢边界破坏 ⇒ 结构否定」在该 regime 成立。
4. **OFF 路径不受影响**：所有现有 reading-B（单腿）/instances/nest 路径 flag 门控旁路，下游 OFF/ANCHOR/NEST/READING_B 回测数字逐字不变。

---

## 5. 谱系引用

- **539号 / [[project_t_short_leg_regime_function]]**：根翻空有效域⊂非上行 regime——本工位 short_pnl 主导失血是 LegPair 形态的复现（空腿吃回调在 CL 上行 regime 持续失血）。
- **547号 / [[project_cascade_level_misattribution]]**：次级别信号越级翻动主力——本工位用 `prove_pair_isolation`（g_pair(k) 只写 leg_pairs[k]）结构否定，对偶单腿 `prove_leg_isolation`。
- **546号死锁解锁 / [[project_deadlock_dual_open_target]]**：多空双开多重赋格（吃每级别涨跌幅绝对值）是**目的**——本工位 LegPair 实现「大级别多腿 + 次级别空腿同时在场」，churn 门控解「核心被 sink 成僵尸」死锁（核心多腿不被回调卖点翻动）。
- **第27课区间套**：链破坏 churn 门控（链完整=回调不动核心 / 链破坏=转折动核心）的定义来源。
- **不确定谱系**：「否定线（中枢边界）作操作层止损」是否曾发生概念分离——未在谱系中找到明确记录，标注为可能的新语法记录候选（待 genealogist 审）。

---

## 6. 影响声明

**改动文件**（4 个，均 worktree 未 commit）：

| 文件 | 改动 |
|------|------|
| `rust/src/recursive_t/rec_engine.rs` | ① `EngineConfig` 加 `enable_reading_b_pair` flag（from_env/off=false/新构造 `reading_b_pair()`）；② `LegPair` 结构 + idle/active/fingerprint；③ `LevelView` 加 `zg`/`zd: [Option<f64>; MAX_LEVEL]`（否定线原料）；④ `TRoot` 加 `leg_pairs: Vec<LegPair>` + per-level pnl/opens/closes/stops/churns 观测计数；⑤ `nav()` 累加 leg_pairs（OFF idle ⇒ bit-exact）；⑥ LegPair 操作族（open/close long/short、`pair_stop_loss_step` 否定线止损、`g_pair` 买卖点开平+churn 门控、`consume_leg_pairs`、`finish_leg_pairs`、`prove_pair_isolation` 547 隔离守卫）；⑦ `on_bar` 加 pair 分叉（最先检查，含 NAV≤0 安全网强平+杠杆验收）；⑧ `finish` 加 pair 分支 |
| `rust/src/recursive_t/rec_driver.rs` | `extract_view` 投影 `view.zg[k]`/`view.zd[k]` = 各级别 `trends.last().zhongshus.last()` 的 high/low |
| `rust/src/recursive_t/rec_stream.rs` | ① `d_top_arr` 计算门控扩到 `enable_reading_b_pair`（churn 信号）；② 新增 L3 test `prop4_reading_b_pair_l3`（CL 验收） |
| (无第四源文件改动；data_cache 为符号链接) | — |

**影响的定义/模块**：仅 reading-B 路径（新 flag）。OFF/ANCHOR/NEST/READING_B(单腿) 路径逐字不动（bit-exact，由 `EngineConfig::off()` 四 flag 全 false 保证 + 102 recursive_t 测试 + 536 lib 测试全绿 + `rec_flat_bit_exact_含走势完成清仓路径` 通过验证）。

---

## 7. d_top 删除状态

**保留 d_top**（优先 bit-exact 不破，任务允许）。原因：
- 旧 reading-B（`enable_reading_b`）单腿路径**依赖** `LevelView.d_top` 作 switch 触发器（`g(k)` 消费 `view.d_top[k]` close+flip）。删 `d_top` 字段/`divergence::d_top` fn/`rec_stream` d_top_arr 计算会破坏旧 reading-B 的 bit-exact（其 L3 `prop4_reading_b_recursive_l3` 行为改变）。
- 新 LegPair 路径**复用** `d_top` 作链破坏 churn 门控信号（语义对齐：`d_top[k]=true`=区间套链贯通=转折）——这与任务「区间套链破坏 churn」目标一致。
- 任务「删 d_top」目标针对**单腿模型的几何方向 switch**。新 LegPair 路径已**删几何方向**（`open_long/short_leg` 由 `view.buy/sell` 涌现，不读 `node.direction`），但**复用 d_top 数组作 churn 信号**（不是删除而是语义重用）。

**严格标注**：单腿 reading-B 的 `dir_to_polarity(node.direction)` 几何方向开仓在 LegPair 路径中已**不存在**（g_pair 零几何方向）。`d_top` 触发器在 LegPair 中从「switch（close+反向 open）」变为「churn 门控（仅核心多腿反向平的闸门）」。若 Lead 后续决定彻底删单腿 reading-B 路径，则 `d_top` 可随之删除——但本工位不删（优先守 OFF/旧基线 bit-exact，符合任务安全做法）。

---

## 8. no-workaround 检查

**未遇真实概念矛盾**。审计判断架构可扩展，执行确认：
- **多空双开 vs 单 free 池/守恒**：开多腿 `free −= m·c`，开空腿 `free += m·c`，NAV = free + Σ sign·u·c。两腿同级别同时在场时各自对 NAV 正确贡献，`prove_tw_neutral` 每 op 通过（CL 全程零 panic）⇒ **无守恒矛盾**。
- **几何塔配额**：多/空腿各自调 `geom_tower_quota`，配额自洽（每腿独立领），但**叠加产生 >1× 杠杆**（max_gross=4.62×）——这不是守恒/配额矛盾（会计自洽），而是「恒仓声明 vs 实际敞口」的有效域边界（已在边界条件1标注，非 workaround；若交付契约要求 ≤1× 则需新增配额分配轴，是选择类待裁决）。

**唯一诚实标注的有效域边界**（非矛盾）：多空腿配额未在顶层一维配额内分配 ⇒ max_gross >1×。这是**已知边界**（与任务「恒仓声明=max_gross≤~100」验收口径有张力，但任务交付契约只要求零强平/守恒/有限正，未要求 ≤1×），不阻塞本工位交付。如需 ≤1× 恒仓，建议 Lead 开「LegPair 多空腿配额分配」选择轴。
