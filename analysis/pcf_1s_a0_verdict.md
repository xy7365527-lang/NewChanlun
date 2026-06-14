# pcf 1s a0 接入判决：source 坍缩对 a0 不变（1s 救不了高级别 candidate 稀缺）

> 编排者 2026-06-14"我们的 nautilus 有这个框架，在这个框架下搞就行了""严格实现"。
> 上游：pcf v2 判决（`positioning_chain_fugue_verdict.md` §v2）决定性发现——1min a0 上
> source 83-87% 坍缩到 segment，根因隔离为「高级别 candidate 本身稀缺」，**边界条件**
> 列「改 a0 粒度」为未检验开放轴。用户切 1s a0 检验该轴的**更细**方向。
> 脚本：`analysis/positioning_chain_fugue_1s_a0.py`；数据：`data_cache/es_1s_databento_1y.json`
> （11,765,833 bar，2025-06-12→2026-06-12，Databento 免费档 cost $0.0）；
> 输出：`data_cache/pcf_1s_a0_ES.json`。

## 1. 结论

**1s a0 救不了 source 坍缩。塔变高了（recL3 涌现），但 source 仍坍缩到 segment
（85.6%）——坍缩对 a0 不变。必然性检验 PASS（验收标准）。**

关键架构事实（印证"在这个框架下搞就行了"）：**a0 粒度 = 输入 bar 分辨率，引擎零改动**。
buysellpoint.rs / nested_fugue 原语 / positioning_chain_fugue.rs 自底向上从喂入的 bar
构建塔（bi→segment→...→递归层）。切 1s a0 = 把 1min bar 换成 1s bar，无任何引擎/pcf 改动。

唯一变量 = a0 对照（同一份 ES 1s 序列，1s 塔 vs 墙钟分钟桶聚合 1min 塔，同窗同源同信号层）：

| 塔 | bars | seg_root% | src_levels | 塔高(max ladder) | strat% | BH% | trades | roots | spawns | sellpt |
|----|-----:|----------:|-----------:|-----------------|-------:|----:|------:|------:|-------:|-------:|
| **1s** | 11,765,833 | **85.6** | 4 | **recL3 (5)** | +17.6 | +23.2 | 3381 | 404 | 0 | 120 |
| **1min**(同窗聚合) | 353,430 | **91.3** | 2 | move(L1) (3) | +13.2 | +23.2 | 170 | 23 | 0 | 7 |
| 1min(在册 10y) | 5,589,928 | ≈87 | 3 | recL2 (4) | +176.9 | +594.3 | — | 409 | 3315 | 106 |

root 入场层分布（1s 塔）：segment 346 / move(L1) 54 / recL2 3 / recL3 1。
即 11.77M bar 全年，**仅 4 次入场落在 recL2+，346 次落在 segment**。

## 2. 决定性发现：source 坍缩是 a0 不变量（尺度不变性第二次显形）

三组 seg_root% 聚在 **85-91% 带**（1s 85.6 / 同窗 1min 91.3 / 在册 10y-1min 87）。a0
跨 60× 范围（1s↔1min）+ 窗口跨 10×（1y↔10y），segment 占比**几乎不动**。

**塔确实变高了**：同一 1 年窗口，1s 塔涌现到 recL3（ladder 5），1min 塔只到 move(L1)
（ladder 3）；src_levels 4 vs 2。⇒ 1s **确实造出了更多高层 ladder**（H_救的前置成立）。
**但 source 仍坍缩到 segment**：高层 ladder 涌现了，却罕被定位链选中为操作层。

机制（v2 根因在 a0 轴上的延伸）：高级别 candidate 在**时间上**罕见是市场结构属性，
不被 a0 细化改变。1s 塔把同一个墙钟反转分辨成更高的 recL3，但 recL3 candidate 在时间上
**和 1min 的 move candidate 一样稀疏**——任一操作时刻活着的最高 candidate 仍是塔底
（现在是更微观的 1s-segment）。这正是旧 1s nest 覆盖率实验 §5a"a0 细化不改变各层存活率，
只把床位推上高打破层"的**尺度不变性**，现在在自上而下级联 pcf 下复现（旧实验是自下而上
fusion_v）——**两种武装方向、两个引擎、跨 a0 一致**。

**1s-segment 操作上比 1min-segment 更差**：1s 塔 segment held 575,884 bar-holds / 346
入场 ≈ 28 分钟/腿，但 bsp churn 在秒级分辨率上发生（deep_fires 4178 vs 1min 228，18×）。
held 时间 97.6% 压在 move(L1)（11.48M bar-holds，少数长腿），segment 是高频微churn——
"1s 不是操作床位"判决第四次加固（CL 秒级 a0 → 加速测量 → 旧 nest 覆盖率 → 本轮 pcf）。

## 3. 必然性检验（验收标准，L2）：PASS

引擎内 `prove_chain`（F/C/D/E 每操作点 N1 完整级联链 / N2 因果 / N3 链顶一致）+
守恒律（§8.1 每 bar Σunits=N_base）违反即 panic。

- **1s 塔 11.77M bar + 1min 塔 0.35M bar 跑通零 panic** ⇒ N1∧N2∧N3∧守恒在 1s 数据上
  成立（L2 运行时证明）。
- 这是必然性检验首次在 **1s a0 + 同窗对照** 上验证——级联不变量（连续前缀 + 统一极值
  + 高 source 优先）在更高塔（recL3）上仍构造成立。

## 4. 回测（L2 有效域读数，非验收）

- **P1（≥BH）两塔皆否**：1s +17.6% / 1min +13.2%，均 < BH +23.2%（ES 1y 是温和牛年
  +23.2%、浅回撤 -9.8%，两塔皆小幅跑输 BH）。
- **spawns=0 两塔**：ES 1y 太平静，未触发降成本 spawn（v2 在册 10y ES spawns=3315）⇒
  v2 的"降成本 churn"死因在此窗口不显形，死因退化为 segment 微churn 拖累（churn 成本 <
  温和牛趋势收益，故 strat 仍 +正但 < BH）。
- 1s 塔 strat（+17.6%）略优于同窗 1min（+13.2%）+ MDD 略浅（-8.0% vs -9.8%）——但这不是
  "1s 修复 source"，是 1s 塔 seg% 略低（85.6 vs 91.3）带来的边际差异，远不足以破坍缩
  或超 BH。

## 5. 定义依据

- **a0**：定位链递归基的 bar 分辨率。`FIRST_BSP_LADDER=2`（segment）= 链底递归基；
  bi(ladder 1) 由 a0 bar 经包含处理构造（buysellpoint.rs），segment 由 bi 经特征序列
  构造——整塔从 a0 bar 自底涌现。a0 = `compute_organic_signals` 的输入 OHLC 分辨率。
- **source 坍缩**：pcf `chain_source` = 最高有 located 的层（自上而下级联保证连续前缀
  `[segment..=S]`）。"坍缩到 segment" = 操作时刻链顶 S 多数 = segment（FIRST_BSP_LADDER）
  ⇒ 操作层退化到塔底（`positioning_chain_fugue.rs` §级联不变量 + verdict §v2）。
- **级联武装**（第14环严格形式）：nf@k 触发 ⇒ `cascade_arm` 武装 `located[FIRST_BSP..=k]`
  全层。高级别 candidate 单独即可级联出高 source（单测 `high_candidate_alone`），不需
  中间层独立对齐——故坍缩**非**武装机制问题，是 candidate 频率问题（v2 已隔离，本轮在
  a0 轴确认）。
- **唯一变量对照**：墙钟分钟桶聚合（first/max/min/last）= 旧 `nest_coverage_1s_a0.aggregate_1min`
  逐字同构（同数据源、同信号层、同 settle 门、同引擎、同 floor，唯一变量 = a0）。

## 6. 边界条件（结论翻转条件）

1. **单标的单窗口（ES 1y）**：L2，非 L3。ES 1y 是温和牛 + 浅回撤——spawns=0 使 v2 的
   churn 死因未显形。若换深崩/区间标的（CL/BRN 油链，v2 跨引擎正域）或更长 1s 窗口
   （Binance BTC 1s ~31.5M bar 在册可拉），seg% 与 strat 符号可能不同——但 seg% ≥85%
   的坍缩是 candidate 频率结构属性（跨 1s/1min/10y 三组一致），**预期不翻**。
2. **同窗对比方向**：本轮发现同窗 1s seg%（85.6）< 1min（91.3），与 v2 边界条件"更粗→
   source 上移"的方向**弱相反**（更细反而 seg% 略低）。但两方向都落在 85-91% 带内——a0
   单调性效应 <6pp，远小于坍缩量级（segment ≈ 6:1 主导），故"a0 救坍缩"两方向皆否。
3. **若 located 改 candidate 武装（非 nf 触发武装）**：v2 边界条件列的另一开放轴（分离
   intent/confirm，牛市买 located 持久）——本轮未触碰（用户精确指向 a0 轴，非武装赋值点）。
4. **跨塔区间套耦合（1min 锚 × 1s 确认）**：旧实验列为"1s 价值唯一存活路径"，本轮**整塔
   替换**判决不否证跨塔耦合（双塔并行架构，交易层在 1min 塔，确认词汇从 1s 塔注入）——
   仍是开放轴，实装成本显著高于整塔替换。

## 7. 下游推论

- **v2 开放轴"改 a0 粒度"在更细方向关闭**：1s a0 不破 source 坍缩 ⇒ pcf 的定位链驱动
  层选择在 1s/1min a0 上一致坍缩到 segment ⇒ a0 不是修复 source 坍缩的轴。URS 的
  E*+最高 θ 层入场仍是在册最优（v2 已结论，本轮加固——换 a0 不改变 pcf<URS）。
- **NautilusTrader 1s 床位的存在论位置被收窄**：框架支持 1s a0（数据/引擎/必然性全通），
  但 pcf 在 1s 上不产生高级别操作锚 ⇒ 1s 床位的价值不在"用更细 a0 救高级别定位"，而
  （若有）在跨塔耦合的确认时序——与 README 已知边界"1s 床位崩溃恢复重放成本不可接受
  （R3），checkpoint 序列化立项前不上实盘"叠加：1s 整塔上实盘的两个独立否定（无 alpha
  增益 + 重放成本）。
- **必然性检验机制可移植到任意 a0**：prove_chain + 守恒在 1s 数据零 panic ⇒ 该验收机制
  与 a0 无关（级联不变量构造性成立），可作任意分辨率接入的标准验收门。

## 8. 谱系引用

- **pcf v2 判决**（`positioning_chain_fugue_verdict.md` §v2）：直接上游。其"source 坍缩
  = candidate 频率结构必然，与武装方向无关"结论，本轮在 a0 轴确认（跨 a0 不变）；其边界
  条件"改 a0 粒度"开放轴在更细方向关闭。
- **旧 1s nest 覆盖率实验**（`1s_a0_nest_coverage_results.md` §5a）：尺度不变性"a0 细化不
  改变各层存活率"——本轮在自上而下级联 pcf 下复现（旧实验自下而上 fusion_v），两武装方向
  跨 a0 一致 ⇒ 尺度不变性升格为引擎无关的结构常数候选。
- **CL 秒级 a0 判决 / CL 秒级 maker 执行 / 旧 1s nest 覆盖率**："1s 非操作床位"第四次
  加固（本轮新增：deep_fires 18× + held 97.6% 压 move + 整塔 pcf 无 source 增益）。
- **539号 A′**（`project_constitutive_throughput_falsified`）：pcf 同族踏空家族——本窗口
  spawns=0 故 churn 死因未显形，但 source 坍缩根因与 A′ 同（located 驱动层选择无法稳定
  锚定高级别操作）。
- **形式化有效域规则**：本判决 = **否定性结果**（1s 不救坍缩），缩小有效域边界——按规则
  否定性结果比确认性结果信息增量更高（L2：单标的 1y 多 regime 不足，深崩标的 + 长 1s 窗口
  为 L3 升格路径）。

## 9. 影响声明

- 新增 `analysis/positioning_chain_fugue_1s_a0.py`（1s/1min 双塔对照回测，唯一变量 a0）。
- 新增数据 `analysis/data_cache/es_1s_databento_1y.json`（657MB，gitignored，cost $0.0）+
  结果 `analysis/data_cache/pcf_1s_a0_ES.json`。
- 本报告 `analysis/pcf_1s_a0_verdict.md`。
- **零引擎/在册改动**：buysellpoint.rs / nested_fugue 原语 / positioning_chain_fugue.rs /
  config.rs 未触碰；pcf v2 八标的在册数字零接触；URS/nif/其余引擎零接触。a0 接入纯由
  输入 bar 分辨率实现（印证"在这个框架下搞就行了"——框架已支持，无需改 a0 参数）。
- 部署矩阵零变更：1s 整塔臂未超越任何在册配置（strat < BH 且 source 坍缩未破）。
- 认识论等级：必然性检验 = L2（12M bar 运行时证明）；source 坍缩判决 = L2（单标的 1y
  同窗对照，否定性结果——a0 轴在更细方向关闭，有效域边界收窄）。
