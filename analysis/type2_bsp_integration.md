# type2 买卖点操作路径设计 + 实现 + OKLO 判决

> 任务（2026-06-11 编排者）：C段修复后 type2 BSP 从 0 解锁到数百个，但交易层
> 没有消费 type2 信号。调研原文操作含义 → 确定多重赋格中的角色 → Rust 实装
> （config 可独立消融）→ OKLO 回测验证。
>
> 认识论等级：O0 守卫 L1（管线等价）；变体结论 **L2**（OKLO 单标的真实数据，
> 1min 447,738 bar，无 oracle）。T2o 新增腿样本 N=5——效应方向清晰但样本极小，
> 不构成统计显著性声明（P3 判决先例：小 N 禁均值±σ检验）。

## 一、结论

| 变体 | 配置 | 复利% | Δ(vs P5) pp | Δ(vs V2r) pp | rev对数 | rev胜率% | rev净现金 |
|---|---|---|---|---|---|---|---|
| P5（基线锚） | — | +1088.8 | — | — | — | — | — |
| V2r（在册基线） | 震荡型+θ=1%+配对闭腿 | +1499.2 | +410.4 | — | 183 | 52.5 | +54,591 |
| **V2rT2o** | + sell2_open | **+1644.5** | **+555.7** | **+145.3** | 188 | 53.7 | +67,567 |
| V2rT2c | + buy2_close | +1498.4 | +409.6 | −0.8 | 183 | 52.5 | +54,464 |
| V2rT2 | + 两者 | +1643.7 | +554.9 | +144.5 | 188 | 53.7 | +67,440 |

（BH=+307.1%；O0≡P5 四面对账 PASS，195 笔 / 317 trace 行——type2 轴零侵入
legacy 路径。两轴贡献严格可加：T2o+T2c≈T2 组合，无交互项。）

**判决**：
1. **T2o（Sell2 开腿）成立**（L2）：6 次 Sell2 参与触发 → 5 条净新增腿（1 次与
   Sell1/盘背同 bar 共现），**5/5 全胜、全部经 ZD 触线闭腿**（zd_close 38→43），
   平均每腿 +2,595 净现金，REV 胜率 52.5→53.7%。复利 +145.3pp 来自降成本现金
   在 master 腿上的复合放大。
2. **T2c（Buy2 闭腿）有效域近空**（L2 否定性结果）：183 对腿中仅 **1 次** 同锚
   confirmed Buy2 在腿存活期内出现并闭腿，净现金 −127（该腿若等 t6 会在更好
   价位回补——t6 计数 116→115 确认替代关系）。错配持仓 bar 18→15。
3. type2 事件密度（磁带层，candidate+confirmed 去重行）：L2 sell2=589/buy2=560，
   L3 89/108，L4 7/8——交易层可消费的 confirmed 子集远小于此（见三-2）。

## 二、原文操作语义（调研结论）

- **定义**（第17课）：第二类买点 = 第一类买点后第一次次级别回调的低点（不重回
  中枢）；"这样的买点是绝对安全的，其安全性由走势的'不患'而保证"。卖点反之。
- **定律一**（第17课）："任何级别的第二类买卖点都由次级别相应走势的第一类买点
  构成"——type2 本质是次级别 type1 的递归体现。
- **操作角色**：确认点。type1 说"趋势可能结束"，type2 说"反向段确实开始"
  （走势必完美保证其后至少一段同向次级别运动）。
- **多重赋格映射**：
  - Sell2 → voice REV **开腿**候选：顶部确认后开反向段空腿，确认强度高于 Sell1
    （已消化次级别回升不创新高）→ T2o 轴。
  - Buy2 → voice REV **闭腿**候选：底部确认 = REV 空腿假设被结构性否证 → 回补
    → T2c 轴。优先序 Buy1 之后、ZD 触线之前（证据强度：Buy1 本体 > Buy2 确认
    > 纯几何触线）。

## 三、机制解读

1. **T2o 为什么赢**：Sell2 触发的腿天然满足"中枢仍存活 + 价格已确认顶部"——
   开腿点在回抽高点附近（比 Sell1 的背驰点更靠近顶部回声），到 ZD 的几何兑现
   空间更满。5/5 经 ZD 触线闭腿印证：这些腿走完了完整的"顶部确认→回落穿中枢"
   路径，与域腿 77% 胜率同机制（几何兑现）。attempts 1821→2082（+261）但开腿
   仅 +5：深度门（+142 拒）和存活中枢条件过滤掉绝大多数 Sell2。
2. **T2c 为什么近空**：同锚 confirmed Buy2 要求 type1 买前体 + 次级别回调完成，
   而 V2r 腿的闭腿竞争者（t6 candidate Buy3 占 62%、ZD 触线占 21%）几乎总是
   先到——Buy2 形成时腿早已闭合。剩下 1 次触发还劣于 t6 替代路径。**这与
   区间套配对修复 P6 的判决同构：买回侧定位 = 锚 ZD 触线/Buy3 承载，次级别
   确认类信号在闭腿端不增益。**
3. **与 sell2_trigger（legacy 轴 R2/K9）的关系**：legacy 轴把 Sell2 塞进段终结
   三触发（盲开，无锚配对），本轴在配对模式下消费（震荡型锚解析 + 深度门）——
   两轴互不相通，配置位独立。

## 四、实现摘要（纯 Rust 链路）

| 文件 | 改动 |
|---|---|
| `rust/src/trading/config.rs` | `sell2_open`/`buy2_close` 两位（默认 false）+ 变体 `V2rT2o`/`V2rT2c`/`V2rT2` |
| `rust/src/trading/level_operating_unit.rs` | `rev_open_signal` 加 Sell2 臂；`rev_paired_open` 触发源 osc_sell2（trigger 掩码 bit3）；`step_down_paired` 加 buy2_paired 分支（reason=10，Buy1 之后 ZD 之前）；单测 ×2 |
| `rust/src/trading/types.rs` | 计数器 `n_rev_sell2_open`/`n_rev_buy2_close` + py_items 导出；日志位掩码/reason 文档 |
| `rust/src/trading/runner.rs` | capability guard：T2o/T2c ∧ ¬rev_paired → fail-fast |
| `analysis/type2_bsp_backtest.py` | OKLO 回测脚本（O0 守卫 + 磁带指纹守卫 + 增量续跑） |

关键实现事实：type2 事件的 `cs/zd/zg` 从其 type1 前体逐字段复制
（`buysellpoint.rs::_make_type2_point` 移植），同锚判据与 Buy1 同语义可比。
Sell2 共现使 trigger 掩码 ≠ 2，腿自动退出 R2/R3 作用域（type1 卖腿保护不变，
任务硬约束）。

## 五、结果包（六要素）

1. **结论**：见§一。T2o 成立（+145.3pp，5/5 全胜），T2c 有效域近空（1 次，−0.8pp）。
2. **定义依据**：第17课 type2 定义 + 买卖点定律一（type2 = 次级别 type1）；
   `BspClass::Sell2/Buy2`（types.rs C6 六变体）；配对开闭腿语义
   （level_operating_unit.rs rev_paired 路径，2026-06-10 编排者任务）。
3. **边界条件**：
   - T2o 结论翻转条件：多标的（QQQ/BRN）复验中新增腿净负；或 N=5 的全胜在更大
     样本中回归到 <50%（小样本不可排除）；或数据基底变化（OKLO 447K
     alphavantage，强趋势标的——regime-dependent 风险与版本 I 判决同构）。
   - T2c 结论翻转条件：在闭腿竞争者更弱的配置（如 pre_type3=false 关掉 t6）下
     Buy2 获得非空作用域且为正——当前"近空"是 V2r 闭腿集竞争下的结果，不是
     Buy2 信号本身无效的证明。
4. **下游推论**：若 T2o 在多标的复验成立，V2rT2o 应接替 V2r 成为 REV 配对基线
   （编排者判决事项）；Sell2 的"顶部确认"语义可进一步用于 master 出场升级的
   候选研究（当前 master 只消费 sell1 行）。
5. **谱系引用**：532号（段终结触发轴拆分）；C段修复谱系（BSP 缺口根因诊断 →
   B2 越界极值，type2 0→360 的解锁来源）；REV 配对修正判决（同锚 Buy1 闭腿）；
   P6 判决（买回侧定位=ZD 触线，次级别确认不增益——T2c 近空与之同构）。
6. **影响声明**：新增 2 个 config 位 + 3 个变体 + 2 个计数器 + 1 个 close
   reason（10）+ 1 个 trigger bit（3）；默认全关，O0≡P5 守卫 PASS 证明在册
   对账面零漂移；不改动任何既有变体语义。

## 六、复现

```bash
PATH="/usr/bin:$PATH" cargo test --lib t2          # 单测（rust/）
PYTHONPATH=src .venv/bin/python analysis/type2_bsp_backtest.py   # OKLO 回测
# 数据：analysis/data_cache/type2_bsp_backtest.json / _tables.md
```
