# C2 裁决：𝔖_Θ 三阶段（负成本挣股数）转移触发判据

- **negation_source**: heterogeneous（OpenAI Codex CLI，decide 模式）
- **task**: #136（C2 矛盾——三阶段触发判据）
- **codex 完整对话持久化**: `.chanlun/review-results/codex-decide-20260701-1646.md`
- **canonical 依据**: `~/Downloads/newchanlun-claude-code-result-package/FULL_USER_FORMULA_SOURCE.md` 第十一/十二节 + `strict_hybrid_state_machine_strategy.md` §10/§12/§16
- **认识论等级**: L0（触发结构是 spec 权威定义的直接 port；阈值参数值待 Θ_phase 绑定）

## 裁决：非 spec 真缺口，是 implementation gap

**C2 与 C1（Cand^δ）不同构。** C1 是 spec 缺失定义（无候选谓词权威定义 → codex 从数学本质给结构约束）；
C2 是 **spec 有权威触发定义，但 Lean/Rust port 缺失**。触发结构无需编排者裁——canonical 已逐字给出。
需编排者裁的仅是**阈值参数来源**（`R_⋆ / L^wc / m^unit / α / D_dist` 的配置或运行时输入），不是触发结构。

## 触发结构（canonical 权威，直接 port）

### 触发 1：`RecoverCapital(w)`（阶段一 → 阶段二，FULL_USER §11「安全取本条件」）

canonical `ReadyReturn_t`（5 项合取，逐字）：
```
ReadyReturn_t := [μ_t=Normal] ∧ [O_t=∅] ∧ [∀v≠r, a_{v,t}=0]
               ∧ [D_t^dist ≥ I_0 − W_t]
               ∧ [R_t − (I_0 − W_t) ≥ R_⋆ + L_{t+1}^wc]
```
提现口径（**权威，非全部利润、非 R 全额**）：
```
w_{t+1} = min{ I_0 − W_t, D_t^dist }
```
即「剩余未取本金 vs 可分配现金」的 min。**本金回收优先于挣股数**，取本后 `R=Π−A−W` 账本中 W↑、R↓。

### 触发 2：`EnterEarning`（阶段二 → 阶段三，FULL_USER §11「阶段二」末句 + §12 Φ 分类）

canonical 阶段二→三：`W_{t+1} ≥ I_0 且所有战术仓、订单均已清除时进入第三阶段`。
port 触发谓词：
```
ShouldEnterEarning := [W_t ≥ I_0] ∧ [∀v≠r, a_{v,t}=0] ∧ [O_t=∅] ∧ [openLegacyLegs=0]
```
**关键**：`EnterEarning` 判定用**取本结算后的 post-state** `W`（若有 pending withdrawal Y_t，必须等结算）。
**不要求 III_accretive**——`III_repair/protected/accretive` 都已是第三阶段子态，`m^unit/α` 只管**能否增根单位**（阶段三内子分类），不是进入阶段三的门。

## η/g/a 口径映射（问②核验通过）

| 递归式 `η_{n+1}=η_n+g_n−a_n` | canonical | 语义 |
|---|---|---|
| η_n | R_t（未分配已实现利润储备） | 储备 |
| g_n | g_t（一组短差净实现利润） | 挣的 |
| a_n | a_t（增根单位资本） | 投入股数的资本 |

即 canonical 记账递归 `R_{t+1}=R_t+g_t−a_t` 的对应。**取本 w ≠ a_t**：w 是本金回收（阶段二），a_t 是增根单位（阶段三），不可混同。

## 已确认的实现层现状（trust-but-verify 已核验引用真实）

| 位置 | 现状 | 缺口 |
|---|---|---|
| `formal/Origin/CovariantCapital.lean:368` `ReadyReturn` | 已 port，但**只含资本侧**两不等式 `(I0−W)≤D ∧ (Rstar+Lwc)≤R−(I0−W)` | 缺 `μ=Normal / O=∅ / ∀v≠r a_v=0` 三门（已 Read 核验:366-390 属实） |
| `formal/Origin/TotalWealth.lean:120,245` `TWEvent/twStep/LegalEnterEarning` | raw 事件代数 + `openLegacyLegs=0` 必要 gate | 非完整触发谓词，只是入口证书 |
| `rust/.../strategy/ledger.rs:152` `TwEvent` | 有 `RecoverCapital/EnterEarning` 变体 | — |
| `rust/.../closed_loop/transition.rs:120` `schedule_adapter` | 用 `RecoverCapital(1)` 占位，**不产 EnterEarning** | 三阶段永停 rank≤1，EarningShares 不可达（本 C2 矛盾的物质形态） |

## 对 gap3 步骤3 的指令

1. **不新增 spec 缺口 escalate**——触发结构已有 canonical 权威定义，直接 port。
2. **触发计算放调度/分类层，不放 twStep**（twStep 保持纯 raw 账本转移，与 Lean §诚实 rawStep/legalStep 双层一致）。用不可变输入结构（codex 称 `PhaseTriggerInput`）计算两纯谓词 `ReadyReturnFull` / `ShouldEnterEarning`。
3. **`ReadyReturnFull` = `CovariantCapital.ReadyReturn`（资本侧）∧ `μ=Normal` ∧ `O=∅` ∧ `∀v≠r a_v=0`**——补齐 CovariantCapital 缺的三门（不是重写 ReadyReturn，是在调度层合取三个非资本门）。
4. **`schedule_adapter` 删除 `RecoverCapital(1)` 占位**：`ReadyReturnFull=true` → 发 `RecoverCapital(min{I0−W, D_dist})`；`ShouldEnterEarning=true`（post-state）→ 发 `EnterEarning`。
5. **阈值参数（`R_⋆/L^wc/m^unit/α/D_dist`）声明为 Θ_phase 绑定或运行时账户输入**——不硬编码。若无配置/运行时来源，声明为**未绑定参数缺口**（这是唯一需编排者裁的部分：参数来源，不是触发结构）。
6. **补桥不变量**：`TWState.withdrawn` ↔ `LedgerState.W_t` 需桥接不变量，否则双账本漂移（codex 边界条件）。

## 边界条件（结论翻转条件）

- canonical 后续改写 `ReadyReturn_t` 或阶段完全分类 Φ_t → 推翻此 port。
- `D_dist/L^wc/R_⋆/m^unit/α/LotCost` 无配置或运行时来源 → 不能硬跑，声明未绑定参数缺口（不硬编码）。
- 存在 pending withdrawal Y_t → `EnterEarning` 必须等结算后用 post-state `W≥I_0` 判定。
- `TWState.withdrawn` 与 `LedgerState.W_t` 无桥接不变量 → 先补桥。

## codex 排除的误方案（简化质询确认成立）

- ✗「交编排者裁触发结构」——触发结构已有 canonical 定义，裁的是参数来源。
- ✗「只用 `openLegacyLegs=0` 触发」——只是必要 gate，缺 W≥I0/清仓/清订单。
- ✗「负成本 `W+R>I0` 就进三阶段」——负成本是结果性质，不替代取本完成 + 清仓清订单。
- ✗「`RecoverCapital(1)` 占位」——违反 no-workaround。
- ✗「只有 III_accretive 才 EnterEarning」——III_repair/protected 也是阶段三，只是禁增根单位。

## 影响声明

- 涉及模块：`formal/Origin/CovariantCapital.lean`（ReadyReturn 三门补齐位置）、`rust/.../closed_loop/transition.rs` schedule_adapter（触发发出）、`rust/.../strategy/ledger.rs`（TwEvent 消费）。
- 不改定义文件。不改 Lean twStep raw 层。
- 对 task#132（GAP3 TW 提现端 W）：解锁——触发结构已定，步骤3 可执行 port。
