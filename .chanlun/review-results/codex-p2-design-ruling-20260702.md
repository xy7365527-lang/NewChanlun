# codex-p2 审计裁定：D1/D2 实装设计（full-strategy-pi p2）

- **工位**：ws-codexp2（goal g-full-strategy-pi，P0 两缺口修复设计审计）
- **日期**：2026-07-02
- **审计对象**：`.chanlun/review-results/full-strategy-pi-conformance-20260702.md`（p1）§p2 实装设计草案 D1/D2/D3
- **Codex 完整交互**：`.chanlun/review-results/codex-review-20260702-234153-e7df.md`（自动持久化，含完整 prompt+response）
- **裁定结论**：**D1 fail，D2 fail**（否定成立，非误判）

---

## 1. 结论

### D1（8 谓词解释器统一，P0-1）—— **fail**

D1 提议"为 `interp::interpret` 补桥接函数 + property test 证 interp 三桶结果 ⟺ mutex_class 派生动作"，这个方案本身建立在一个**范畴错误**之上：

- `mutex::mutex_class`（8 谓词 P1..P8）与 `closed_loop/mutex_interp.rs::choose_action`（9 谓词 p1..p9，忠实镜像 Lean）的输入是**单时刻全局布尔标量向量**，语义是"给定当前时刻状态，选出唯一一个动作"（互斥优先级链，一次只做一件事）。
- `strategy/interp.rs::interpret` 的输入是**候选集 Γ**，按 `(level, 方向)` 分 slot 独立 fold（`interp.rs:940-998`），**同一时刻可以产出多个 close + 多个 open**（不同级别的声部可同时开/关，无冲突）。

这两个构造不在同一输入空间上，"证等价"这句话没有良定义的比较对象。可直接构造反例：D 桶有一条反向平仓腿、O 桶同时有另一 level 的新开候选 ⟹ 投影到 8 个布尔后多个为真，`mutex_class` 按优先级只选一个（如平多屏蔽开多），但 `interp` 会**同时执行两者**——不需要等 property test 跑出来即可证伪"等价"。

**追加发现（p1 报告遗漏）**：代码库实际存在**第三个**全互斥解释器实装——`closed_loop/mutex_interp.rs::choose_action`（9 谓词，产 10 类 `ActionClass`）。经 grep 核实，`mutex_class(` 与 `choose_action(` 均**只在各自模块的 `#[cfg(test)]` 内被调用，零生产消费者**。p1 报告"两个实装"的表述不完整，实际是三套语义（8 谓词/9 谓词/生产三桶 fold），且两个非生产实装的谓词数都不一致。

### D2（RiskMode 账户层输入接线，P0-2）—— **fail**

代码已诚实占位（`runner.rs:489-496`、`exit.rs:108-117` 均注释"账户层输入未建模，置 0/false 占位"）。D2 提议用"名义仓位 × config 保证金率"反推 `maint_margin`，这不是接入真实账户数据，而是**引入一个从未经真实数据校准的新 Θ_risk 参数**：

- 接入后会真实改变 Deleverage/Liquidation 触发时机 ⟹ 改变回测交易/平仓行为 ⟹ 改变 μ̂ 估计管线的输入分布——与 p1 报告"补输入流，不改判定逻辑"的定性不符，实际影响范围覆盖到 alpha 测量结果。
- `liq_flag` 声称"由模拟撮合产"，但回测侧当前 `AccountState`（`strategy/mod.rs:119`）只有 `nav/voice_qty`，不存在任何"模拟撮合生产 liq_flag"的模块或契约——D2 把问题名义上转移给一个不存在的模块。

### D3（β 强度分箱，P1）—— needs_work（非致命，方向基本可行）

β 分箱是 `μ(Z)→μ(Z,K)` 的 estimand 细化（不是新增独立维度，是对已冻结桶的进一步切分）。代码兼容性上是向后扩展，但**统计上不是免费扩展**——若用于置信 alpha 判定需新冻结/新 family 校正（FWER）；若仅作 exploratory 诊断可后置且标明。p1 报告"后置"的处理基本合理，但需在实装时明确标注这是 estimand 变更而非纯代码扩维。

---

## 2. 定义依据

- PDF《缠论的全互斥定义策略》Part B 四（P1..P8 固定优先级互斥化 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`）：定义在**单时刻全局谓词向量**上（谓词是"是否处于某状态"的标量判断，非"某个候选是否属于某类"）。
- spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md` §11-§12（`interp.rs` 契约锚）：定义在**候选集 Γ(x)** 上，`(𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))` 是集合到三元组划分的映射，不是标量到标量的映射。
- 两个定义域不同一——D1"证等价"缺少把二者投影到同一对象的严格构造，桥接函数把"集合"坍缩成"单布尔"必然丢信息（如 D 桶多条腿如何坍缩成 `close_long: bool` 一项未定义坍缩规则）。
- formalization-validity-domain 规则（231号）：D2 的"名义仓位×保证金率"是把定义域（sizing 已有的名义仓位公式）套用到有效域外（真实保证金约束需要真实交易所/账户数据，非可从缠论/sizing 参数推导）——有效域 ≠ 定义域的又一实例。

## 3. 边界条件

判定翻转的条件：
- **D1**：若编排者裁定"生产语义应收窄为单一动作"（即禁止 interp 同时产多个 open/close），则可以重新设计 interp 使其退化为单动作输出，届时"等价"命题才良定义可比较；或者反过来，若明确宣布 mutex_class/choose_action 只是 PDF 定理的 L0/L1 辅助验证工具（不要求与生产接线的 interp 等价），则 D1 不需要"证等价"，只需要判定去留（删除零消费者实装 or 保留作独立验证）。
- **D2**：若团队有明确计划接入真实 venue/broker 的保证金数据源（而非 config 常数反推），D2 判定翻转为可行；纯 config 常数反推的方案维持 fail。
- **D3**：若确认只做 exploratory 诊断（不进 L2/L3 confirmed alpha 判定），后置方案维持有效；若拟直接用于交易决策，需先走新预注册。

## 4. 下游推论

- p2 阶段不能按原 D1/D2 设计直接实装。需要编排者先裁定两个前置问题：(a) interp 的"多候选并行动作"生产语义是否是 PDF 要求的合规实现，还是需要收窄为单动作；(b) 两个零消费者的谓词链实装（mutex.rs + mutex_interp.rs）是否保留、保留几个、以何种身份（辅助验证 vs 待接线候选）存在。
- 在 (a)/(b) 裁定前，P0-1 的"双实装未统一"应从"设计草案已定，待审"降级为"需要编排者价值判断的选择/矛盾"——这本身可能触发 `/escalate`（定义之间的真实矛盾：生产语义 vs PDF 举例谓词链是否同构）。
- D2 需要重新设计：诚实降级（保留占位 0，不接虚构保证金率）vs 明确开一个新的、独立标注的 `SimRiskModel`/`EmpiricalDomain` 分支（不混入现有 `RiskModeInput` 视为"真值"）。二选一需编排者/工位裁定。
- p1 报告"49 已实装"中至少 3 项抽查判定需要修订（见下）。

## 5. 谱系引用

- formalization-validity-domain 规则（231号）：有效域 ≠ 定义域——D2 的适用。
- no-patch-mentality（090号语法规则）：两个零消费者实装同一定理，是否构成"补丁式重复"需先决断，再谈统一——不应把死代码直接升级为"等价见证"。
- p1 工位产出：`.chanlun/review-results/full-strategy-pi-conformance-20260702.md`（本裁定的直接上游）。
- 待新增谱系条目（建议 genealogist 工位处理）：D1 范畴错误发现 + 第三实装遗漏，属于新的概念发现（p1 报告"两个实装"表述需要谱系层面的订正记录）。

## 6. 影响声明

- 本工位纯只读审计 + Codex 调用，未修改任何生产代码。
- 产出：本裁定文件 + Codex 完整交互记录（`.chanlun/review-results/codex-review-20260702-234153-e7df.md`）。
- 下游消费：p2 实装工位（阻塞，需先解决 (a)/(b) 前置裁定）；genealogist（如需登记第三实装发现为独立谱系条目）。

---

## p1「49 已实装」抽查复核（team-lead 要求项，附加输出）

Codex 独立复核后，以下判定建议修订：

| 节 | p1 原判定 | 复核意见 |
|----|----------|----------|
| A-6/B-十一 细分优势定理 V(Z)≥V(Y) | 已实装（作为设计原则） | 应改判 **N/A**——数学命题不是可"实装"的对象，判"已实装"是范畴误用 |
| B-十三 元素数量 \|E\|=O(n) | 已实装（结构层） | 应降级为 **部分**——仅"存在 coverage 模块"不构成复杂度证明，需显式复杂度证明或 profile 数据支撑才能判"已实装" |
| C2-11 φ:Z→U 限制有效维数 | 已实装（当前桶键本身就是 φ 压缩） | 应降级为 **部分**——φ 压缩的定义要求信息论意义上的压缩最优性论证，当前 `MuClass` 只是一个有限维投影，未论证最优性，现判定构成声明膨胀 |
| A-9 RiskOK（χ 选择器三项） | 已实装（三项齐） | 应改注——RiskOK 输入本身退化（本报告 P0-2），"三项齐"的表述掩盖了 RiskOK 实际只 discharge equity≤0 子集的事实，建议措辞订正为"三项接线齐，但 RiskOK 的输入退化" |

---

```yaml
---stance-declaration---
verdict: fail
review_target: full-strategy-pi p2 (D1/D2/D3 设计草案)
stances:
  d1_equivalence_bridge: reject
  d1_category_error: confirmed
  third_mutex_interpreter_omitted_from_p1: confirmed
  zero_consumer_mutex_implementations_no_patch_risk: needs_decision
  d2_synthetic_margin_input: reject
  d2_liq_flag_producer_missing: confirmed
  d3_beta_bucket_estimand_change: needs_work_but_directionally_ok
  p1_49_implemented_spotcheck: 3_of_4_need_revision
escalation_needed:
  - "interp 生产语义(多候选并行动作) vs PDF P1..P8 举例(单动作谓词链)是否同构——编排者价值判断"
  - "mutex.rs + mutex_interp.rs 两个零消费者实装的去留裁定"
concessions: []
---end-stance---
```
