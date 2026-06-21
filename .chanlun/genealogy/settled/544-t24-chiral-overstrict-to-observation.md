# 544 号：T24 手性交替 L0 守卫降级为观测 + sink/recover 方向相容前提补齐

status: settled
date: 2026-06-17
level: L0→观测（formalization-validity-domain 修正）+ L0（操作适用域前提）
负责工位: CC session（fugue_v3 T24 prove panic 调研+修复）

## 概念分离

**被分离的两个概念**：

| 旧（被否定，543号实装） | 新（结算，本号） |
|----------------------|----------------|
| 手性交替「相邻占用级别方向严格相反」是 **L0 不变量**（prove_chiral_alternation，违反=panic） | 手性交替是 **regime 依赖观测量**（count_chiral_violations，非 panic）——相邻同向在建仓阶段合法 |
| 「相邻交替 ⟹ Σ\|units\| Casimir 守恒」（543 line 35） | **non-sequitur**：守恒由 |units| 转移（reduce_at 一减 + add_at 一加）独立保证，**与方向无关**（prove_conservation 用绝对值）；交替与守恒无因果 |
| sink/recover 可无条件施加（τ word 总适用） | sink/recover 有**方向相容前提**（适用域）：子层/父层方向不相容 ⟹ word 不适用 ⟹ 拒绝（return false，N4 类比） |

## 分离的依据

1. **缠论原文（级别递归 + 买卖点）**：上涨走势初期各级别**都做多**（核心仓 H⁰），尚未出现卖点 ⟹ 相邻级别同向（都 Long）是合法涌现态。某级别出卖点后才翻空做短差（σ⁻¹∘τ 下沉到子级别）⟹ 此时才相邻反向。「永远交替」把 regime-specific 条件误当全局不变量。
2. **formalization-validity-domain 规则（222/223/230号同构）**：手性交替守卫的**定义域**=所有相邻占用级别对；**有效域**仅=「子级别正承载父级别下沉短差」的 regime。声明 L0 不变量（有效域=定义域）是声明膨弛（090号）。本号是该模式第四例。
3. **L2 经验否证**：OKLO/CL/ES/GC 4/8 标的在真实数据 panic（相邻同向 Long），直接否证 L0 声明——否定性结果缩小有效域边界。

## 否定关系

- **否定 target**：543号 line 35「手性交替（相邻方向相反）⟹ Σ\|units\| 守恒（prove_chiral_alternation）」+ line 36「OKLO/CL 零 panic」过早声明。
- **by**：544（本号）。
- **scope**：partial——543 的「操作=word」主命题完全保留；仅修正其手性交替 L0 声明（过强）与守恒因果（non-sequitur）。
- **驱动**：用户裁决 2026-06-17「上涨初期各级别都做多→相邻同向合法；某级别出卖点后才翻空→此时才交替；所以永远交替太严格」。

## 工程实现（落地证明）

1. **prove.rs**：`prove_chiral_alternation`（panic）→ `count_chiral_violations(&[Layer]) -> usize`（观测计数，137号 make-decision-observable）。两个 #[should_panic] 测试重写为计数断言。
2. **operate.rs**：per-bar 守卫改为写入 `FugueResult.max_chiral_same_dir`（镜像 max_concurrent_voices）；σ-ascend 处守卫删除。
3. **cycle.rs（深层修复，移除 chiral panic 后浮现的 add_at panic@bar 220368）**：sink_chunk/recover_chunk 补**方向相容前提**——
   - sink：子层占用且方向 != flip(d_k) ⟹ return false（不能向已持 Long 的子层开 Short 短差）。
   - recover：子层非 Short ⟹ return false（无短差可升回）；父层占用且方向 != Long ⟹ return false。
   - add_at 层内方向一致性断言（accounting.rs:142，「一个 Layer 单一 direction」）**保留**——这是真实不变量，与跨级别手性交替无关。
4. **验收**：cargo test 434 passed / 0 failed；OKLO(447K)/CL(5.53M)/ES(5.59M)/GC(5.54M) 8/8 全量零真 panic（守恒/NAV/per-layer 互斥/递归一致性全程成立）。

## 能指碰撞（source-audit 标注）

**T24 在两引擎是不同命题**（同号异义，类比 527号 σ=h²³ 手性能指碰撞）：
- spiral/prove.rs：T24 = `τ²=e`（手性对合，D∞ 群关系 prove_tau_involution）；541号「4永久残余 T24/T36/T41/T46」。
- fugue_v3：T24 = 「父多子空⟹无对冲净额」（手性交替）。
- T24 在 docs/necessity_derivation.md **无定义**——fugue_v3 借用编号赋新义。建议后续 source-audit 消歧。

## 认识论等级（formalization-validity-domain）

| 命题 | 旧声明等级 | 修正后等级 |
|------|----------|-----------|
| 相邻占用级别方向相反 | L0 不变量（错） | L2 regime 观测（count_chiral_violations） |
| Σ\|units\| = n_base 守恒 | L2 守恒 | L2 守恒（不变，由 |units| 转移保证，prove_conservation） |
| sink/recover 方向相容前提 | （缺失） | L0 适用域（τ word 良定义前提） |
| 4 标的零 panic | — | L3（真实数据验收，可否证） |

## 上游谱系

- 543号（操作=D∞ word）：本号修正其手性交替 L0 声明（主命题保留）。
- 541号（spiral covering space）：T24 能指来源（τ²=e）。
- 222/223/230号（formalization-validity-domain）：有效域 ⊊ 定义域模式，本号第四例。
- 137号（make-decision-observable）：panic 守卫 → 观测计数器的方法论依据。
- 090号（严格性语法规则）：声明膨胀禁止（543 line 36 过早零 panic 声明）。

## 待结算开放轴（诚实标注，no-patch）

1. **max_chiral_same_dir 观测的下游消费**：当前仅记入 FugueResult 报告，「非建仓 regime 高值=word 路由 bug」的判据未实装——观测已暴露，判定待编排者裁决。
2. **T24 能指碰撞消歧**（source-audit）：spiral τ²=e vs fugue 手性交替共用 T24，待源头审计统一编号体系。
3. **sink/recover 拒绝的策略影响**：方向相容前提拒绝了部分 sink/recover（建仓相邻同向期），是否影响 alpha 未单独消融——属 regime 行为非 panic，独立于本号正确性。
