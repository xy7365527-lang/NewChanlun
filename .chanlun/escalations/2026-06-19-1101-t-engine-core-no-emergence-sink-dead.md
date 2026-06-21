# 矛盾上浮：T 操作层 sink/recover/多重赋格依赖 core 涌现到高级别，但 core 结构性钉死在首个入场 ladder（翻转不升级）⟹ 次级别短差恒 0 = 声明膨胀

> 上浮人：T 翻转版 sink 修复工位（2026-06-19 11:01）
> 触发：执行任务「修复翻转版 t_engine 的 bug：翻转（同级别 BSP）不应阻止 sink 短差（次级别 BSP）。当前 drive() 翻转检查后有 return，导致 sink 分支永远执行不到，次级别短差=0 笔」。
> 已完成：按任务去掉 drive() 翻转后的 return（commit `b9fc5c541e`），单测 `翻转与次级别短差同bar并存` 证明 drive 层不再吞 sink（20 测试绿）。但 **L3 真实数据（BTC/CL）sink 仍恒 0**——任务诊断（return 是 sink=0 的原因）被证伪：return 只是必要前置，真因在更深的架构层。
> 分类（四分法）：**选择 + 语法记录**（core 如何随级别涌现升级 = 多个合理方案需价值判断 + 缠论级别归属语义未显式化）。
> 等级：**L3**（BTC/CL 真实数据，否定性结果）。
> 关联：`rust/src/recursive_t/t_engine.rs`（drive/flip_core/try_enter）、`rust/src/recursive_t/stream.rs:155`（ladder 映射）、`docs/unified_recursive_operator_T.md`、memory `project_t_short_leg_regime_function` / `project_t_flip_vs_clear_verdict`。

---

## 1. 停下来：为什么这不是可自决的定理或行动

任务把 sink=0 归因为「drive 翻转后 return 吞掉 sink」。我去掉 return 后，**单测层面**修复成立（同一 bar 翻转 + 次级别反向 BSP 时 sink 能触发，已加回归测试）。但 L3 回测 sink 仍恒 0。

继续推进会撞上 `no-workaround`：让 sink>0 的唯一途径是改变 core 的级别涌现语义，而「core 如何升级到高级别」有多个互斥的合理实现（见 §5），且涉及缠论「次级别短差的载体级别」语义裁决。任选其一硬编码 = 替编排者做架构价值判断。故走 /escalate。

（我已把诊断做到根因可定位、方案可选择的程度——不外包诊断，只请裁决方向。）

---

## 2. 精确描述矛盾

### 2.1 声明（模块文档 + drive 设计）

`t_engine.rs` 模块头大段声明引擎是「**递归多重赋格**」：
- core 持有某级别走势仓位；
- 该级别的**次级别**反向 BSP → `sink`（减 1/3 下放次级别做短差，core→Reduced）；
- 次级别同向 BSP → `recover`（折叠子链归还 core）；
- 子腿本身也是状态机，可再 sink（「每级别独立 ladder 降序扫描，多级别同时持仓核心 + 各级短差」）。

### 2.2 能力（L3 实测 BTC/CL，3 模式）

| 标的/模式 | trades | sink 腿 | recover | 强平 | flip(翻转) 腿 | core ladder 分布 | 入场 ladder |
|---|---|---|---|---|---|---|---|
| CL/structural | 5654 | **0** | 0 | 0 | 2826空+2827多 ≈ 全部 | **100% ladder 3** | ladder 3（1 次）|
| BTC/structural | 5073 | **0** | 0 | 0 | 2536空+2536多 ≈ 全部 | **100% ladder 3** | ladder 3 |

引擎实际退化为**单级别（ladder 3）机械翻多翻空**。「多重赋格 / sink / recover / 子腿递归」整套机制是**死代码**。

### 2.3 根因链（结构性，非数据偶然）

1. `stream.rs:155`：`ladder = bsp.level + BASE_LADDER`，`BASE_LADDER = LADDER_MOVE = 3`。T 树 BSP 的 `level ≥ 0`（`mod.rs::iterate` 从 level 0 起迭代）⟹ **ladder ≥ 3**。`FIRST_BSP_LADDER = 2` 的 ladder 2 永远无 BSP。
2. `try_enter`：全局空仓时在「本 bar 最高 ladder 的 BSP」建 core。流式早期笔少，首个 BSP 必然是 level 0 ⟹ **入场 ladder 3**。入场只发生一次（之后永远翻转，不回 Empty）。
3. `flip_core(core, new_dir)`：翻转**在当前 core 层重建**（`add_at(layers, k=core, ...)`），core 级别不变。
4. 翻转检查 `reverse = (core..MAX_LADDER).any(|j| has_bsp(view, j, flip(d)))`：core 及更高 ladder 的反向 BSP（含 level≥1 的 ladder 4+ 高级别 BSP）**全被消费为「在 ladder 3 翻转」的触发**，而非升级 core。
5. 合并 2+3+4：**core 永远钉死在 ladder 3**。`sink` 的次级别 = `core−1 = ladder 2`，恒在 BSP 级别空间之外 ⟹ `has_bsp(view, 2, ·)` 恒 false ⟹ **sink 恒不触发**。

> sink 设计本意：core 在高级别（level≥1, ladder≥4），次级别（ladder≥3）有 BSP 可做短差。但 core 涌现升级机制**根本不存在**——这是缺口，不是 bug。

### 2.4 矛盾的两端

- **接受「声明」**（引擎是多重赋格）：必须实现 core 涌现升级，让 core 能到 ladder≥4，sink 才有次级别空间。但升级语义未定义。
- **接受「能力」**（引擎是单级别翻转）：则模块文档的「递归多重赋格 / sink / recover / 子腿」全是 `no-patch-mentality` 禁止的**声明膨胀**（声明代码不具备的能力），应删除这些声明与死代码。

二者不可同时为真。

---

## 3. regime 旁证（为什么这不只是「少了点短差」）

单级别机械翻转在 L3 暴露的正是 memory 反复记录的 regime 病：

| regime | 标的 | 三模式 strat | BH | 病灶 |
|---|---|---|---|---|
| 震荡 | CL | +259.8 / +308.5 / +275.8% | +28.2% | 翻转往返吃价差，超 BH（短差缺失不致命）|
| 强牛 | BTC | −349.1 / −581.1 / −1575.3% | +1380.4% | 机械 1:1 翻空腿巨亏(空 −909020 vs 多 +559877)，mdd 216–345% 爆仓 |

BTC 空头全部来自 `flip`（翻空核心），无一来自 sink。这复现 `project_t_short_leg_regime_function`「空头腿=亏损唯一来源、根翻空有效域⊂非上行」+ `project_t_flip_vs_clear_verdict`「强牛无界穿仓」。**sink（次级别小仓短差）本应是比「整仓翻空」温和得多的空头表达**——它的缺失把引擎逼成「要么满多要么满空」的二值暴露，正是强牛爆仓的结构根源。修好 core 涌现 → sink 复活，可能直接缓解 BTC 的翻空腿灾难（待验证）。

---

## 4. 涉及的定义 / 谱系

- **`docs/unified_recursive_operator_T.md` §1.5**：r*（涌现上界）由数据决定，`iterate` 确实产 level 0..r* 的多级 BSP。信号层**有**高级别 BSP，是**操作层 core 不消费它们做升级**。
- **缠师第 65 课 `aₙ=f(aₙ₋₁)`**：所有级别同构。操作层若只在 level 0 运转，违背 T 的级别不变性声明。
- **第 64 课 / T49（区间套定位）**：次级别短差的载体应是 core 的**直接下一级**——这要求 core 自身定位在某个明确级别上，而非永远钉在最低级。
- **`formalization-validity-domain.md`**：声明域（引擎对所有级别多重赋格）≠ 有效域（实测仅 level 0 单级翻转）。当前是有效域膨胀的极端例——有效域是定义域的 1/r* 个级别。
- **谱系 543**（`operation-as-word-not-hardcoded-cycle`，本目录 settled）：操作是「词」不是硬编码周期——core 钉死单级别正是把操作硬编码在 level 0。
- **memory `project_bsp_sublevel_settled_gate` / `signal_layer_duality`**：递归层 BSP 稀疏但存在（L2/L3 信号），引擎需正确消费两层身份。

---

## 5. Lead 的候选方案（请裁决方向，不请代写）

| 选项 | 内容 | 缠论语义 | 取舍 |
|---|---|---|---|
| **A. 翻转升级 core** | `flip_core` 改为在**最高反向 BSP 的 ladder** 重建 core（而非当前 core 层），core 随大级别反向信号上移 | 「更大级别走势结束 → core 升到该级别再操作」 | core 会上移到 ladder≥4，sink 次级别(≥3)有信号；但 core 在高低 ladder 间漂移，需定义下移条件 |
| **B. 独立涌现升级算子** | 翻转保持同级别；另加：core 同向更高级别 BSP（level≥1）→ core 升级到该 ladder（低层转为子腿/归并）| 「次级别走势升级为本级别」（封装语义，第65课步骤d）| 语义最贴合 T，但需定义升级时低层仓位如何处理（归并 vs 子腿）|
| **C. 入场即最高涌现级** | `try_enter` 延迟到 r* 稳定后在最高涌现 ladder 建 core；或周期性按当前 r* 校准 core 级别 | 「在最大可识别级别上持仓，向下做短差」| 改入场时机，core 起点高；但「等涌现」与「流式即时入场」张力 |
| **D. 接受单级别，删声明** | 承认引擎是 level-0 单级别翻转引擎，删除 sink/recover/子腿死代码 + 模块文档多重赋格声明 | 「T 操作层暂只在走势级运转」| 最诚实（消除声明膨胀），但放弃多重赋格设计意图，BTC 强牛病无解 |

**Lead 倾向**：**B**（独立涌现升级算子）。理由：
- 最贴合 T 的第 65 课封装语义（`Move(k) ≡ Level-(k+1) 笔`）——core 升级 = 次级别走势被封装为本级别单元，是 T 已有的结构操作在操作层的对偶；
- 与「翻转（同级别走势结束）」正交解耦，不污染翻转语义（A 把两件事耦合进 flip_core）；
- 保住 sink 的设计意图（多重赋格），且让 BTC 强牛的「温和空头短差」有复活可能。

若编排者认为多重赋格当前不值得（YAGNI），则选 **D**，我立即删死代码 + 改声明（严格，不留半成品）。

**A/B/C 我都不会擅自实现**——它们改变 core 的级别归属语义，是「语法记录」（已在 sink 设计中隐含运作但从未显式化「core 级别如何升级」），需编排者裁决后才落码。

---

## 6. 需要编排者决断的问题（缠论语言）

1. **core 的级别归属**：core 应该（a）钉在首个入场级别、（b）随最高反向走势结束上移（选项 A）、还是（c）随次级别走势封装而升级（选项 B，第65课步骤d 对偶）？
2. **次级别短差的载体级别**：sink「次级别」应是 core 的直接下一级（要求 core 定位在明确级别）。当 core 已在最低级别（level 0 走势级），它**有没有**次级别短差空间？还是 level 0 core 就该禁 sink（只在 core≥level 1 时 sink）？
3. **多重赋格的去留**：当前实测有效域是单级别。是投入 core 涌现升级让多重赋格真正运转（A/B/C），还是承认 YAGNI、删除声明与死代码（D）？
4. **drive return 修复的定位**：我已 commit 的去 return 是「core 涌现修好后 sink 不被翻转吞」的**必要前置**（保留），还是在选 D 时连同 sink 一起删除？

---

## 附录：已完成的工作状态

- **commit `b9fc5c541e`**：`drive()` 去翻转后 return，先翻转核心再按新方向独立处理次级别 sink/recover；新增回归测试 `翻转与次级别短差同bar并存`（sell@5+buy@4 → 翻 FullShort@5 后 sink → ReducedShort@5 + FullLong@4）。20 单测绿，Σ守恒 / NAV 中性守卫不变。
- **该修复正确且必要**（drive 层不再结构性吞 sink），但**不充分**（真实数据 core 不涌现 ⟹ sink 无次级别信号）。
- **逐笔导出**：`analysis/data_cache/t_engine_{BTC,CL}_{structural,and,or}_trades.json`（含 origin 字段）。诊断脚本 `/tmp/t_short_breakdown.py`。
