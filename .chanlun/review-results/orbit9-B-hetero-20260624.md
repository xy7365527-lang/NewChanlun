# orbit9 B-hetero 异质审计报告

**status: codex-available-gpt-5.5**
**audit-date:** 2026-06-24
**target:** `rust/src/recursive_t/rec_engine.rs` commit `faaa0d6d93`
**auditor:** B-hetero 节点（codex CLI gpt-5.5，session `019ef9e3-76a2-7740-8bf6-6bd3c2eea8bf`）
**stance:** 默认证伪（异质否定优先）

---

## Codex 可达性

Codex CLI 版本 0.125.0 可达。模型 `gpt-5.5`（ChatGPT 账号，`o3` 不可用）。
Codex 执行了完整的 shell 导航——读取 `rec_engine.rs`、`t_engine.rs`、`prove_guards.rs`，
逐行检索了 `fn add`、`rec_reduce`、`rec_add`、`account_reduce`、`total_wealth`、`prove_tw_neutral`、
`orbit9_sub_trend_done`、`route_bsp` 的完整实现（共约 138KB 原始输出）。

审计不依赖 Claude 系列对代码的预判——Codex 独立 shell 导航后自行定位关键代码段。

---

## 四质询点逐条判定

### Q1：add 是否真 = 543 word `h⁺∘σ` 而非新原子？

**判定：PASS（否定不成立）**

Codex 观察：`fn add` 的核心操作序列为：
```
rec_reduce(&mut self.instances[sub], m, &mut free, c);     // σ：减 sub 短差腿
rec_add(&mut self.instances[parent], give, pdir, &mut free, c);  // h⁺：增 parent 同向
```
这与 `fn recover` 完全同形——`recover` 也是 `rec_reduce(sub, m) + rec_add(parent, give, pdir)`，
唯一差别是 `add` 用 `m = quota(u_sub) = u_sub/3`（机动仓 1/3），`recover` 用 `m = u_sub`（整条）。

`rec_reduce` 和 `rec_add` 是已有底层原语，均被 sink/recover/drain/add 复用。
Codex 确认 `add` 未引入任何 `{e, h⁺, h⁻, τ}` 以外的新不可约操作。

B 对「不动 h」与「h⁺∘σ」张力的解释（#39 §3.4 精化）成立：
- 543 定义 `h⁺∘σ`（径向递归跨级别）
- #39 §3.4 精化 O3 为「同级别部分重建，不动 h」
- 代码实现忠实于 #39：`prove_sink_descends(parent, sub)` 守卫确保 `sub < parent`（次级别），
  操作在同一级别树 branch 内——这是 `h⁺∘σ` 在「不跨父级别」条件下的特例，不是新原子。

**边界条件翻转：** 若 `add` 引入了向比 parent 更高级别的归还路径，则会构造新原子。代码中未见此模式。

---

### Q2：O3 τ镜像在不动 h 时莫比乌斯是否真不适用？

**判定：PASS with MEDIUM-obs（否定部分成立，但已有前例，非 add 引入）**

**操作层（真对称）：**

Codex 确认 `fn add` 主体完全无 `if pdir == Long` 分支：
```rust
let pdir = self.instances[parent].direction;   // 参数化
let mob = flip_pol(pdir);                       // τ 算子
// rec_reduce(sub[mob-dir]) + rec_add(parent[pdir])
// 二者均通过 pdir/mob 路由，无硬编码多空分支
```
单测 `add_short_τ镜像_父空卖回对称` 直接验证这一点（行 2671-2684）。

**账本层（结构性不对称，MEDIUM 观察）：**

`account_reduce(mob, realized, c)` 在 `mob=Short`（多头买回，pdir=Long）时走：
```rust
Polarity::Short => self.short_leg_pnl += realized
```
在 `mob=Long`（空头卖回，pdir=Short）时走：
```rust
Polarity::Long => // 降成本会计：core_cost_basis 更新 + 阶段转换
```
这两条路径语义不对称——Short reduce 仅累加 `short_leg_pnl`；Long reduce 更新核心成本基础并可触发阶段跳转（`RecStage::CapitalRecovered`）。

**但这不是 add 引入的新不对称。** Codex 对照 `recover`（行 1320-1360）确认：
`recover` 同样调用 `account_reduce(mob, realized, c)`，mob 的决定逻辑完全相同。
这是三阶段会计系统的结构性设计：Long reduce（核心高位卖出）≠ Short reduce（短差腿），
两者本就是不同的财务语义。不对称是 `account_reduce` 的设计，不是 O3 的越界。

**B 声称的 τ 镜像范围** = 操作层（无 if 多空）= 成立。
**账本层语义差异** = 已有设计，非 add 引入，不否定 B 的核心声明。

---

### Q3：账本过滤是否真在 route 之后（无 #35 C7 重犯）？

**判定：PASS（否定不成立）**

Codex 检索了 `fn route_bsp` 完整实现（行 1445-1515）。
route_bsp 的同父向分派分支（行 1468-1485）中：
```rust
// route_bsp 内部
if self.enable_orbit9_dispatch && !self.orbit9_sub_trend_done(j) {
    self.add(p, j, c);      // O3
} else {
    self.recover(p, j, c);  // O7
}
```
route_bsp 本身不含任何 `account_reduce` 调用——分派决定仅依据 `enable_orbit9_dispatch` 标志和 `orbit9_sub_trend_done` 判别接口（后者当前为占位 `fallback=true`），无账本状态门控。

所有 `account_reduce` 调用均位于原子函数（`sink`/`recover`/`drain`/`add`/`enter`）内部，
是操作自带的会计核算，不是 route 层的分派前置条件。

注释（行 1477）亦明示：「account 过滤是 route 之后独立 gate（此处零 if regime/account，τ 对称 pdir 参数化）」。

**#35 C7「账本过滤先于路由」重犯检验：无。**

---

### Q4：add 是否双计 NAV（违反 prove_sink_recover_balance 守恒）？

**判定：PASS（否定不成立，初始分析误差已更正）**

Codex 完整读取了 `total_wealth`（行 1017-1020）和 `nav`（行 985-1014）的实现，确认：
- `nav = free + Σ_inst sign(d)·u·c + withdrawn`
- Long instance：`+ u*c`；Short instance：`- u*c`

`add` 的 TW 中性逐步推导（多头买回场景，pdir=Long，mob=Short）：

| 操作 | free 变化 | NAV 贡献变化 | TW 净变化 |
|------|-----------|-------------|-----------|
| `rec_reduce(sub[Short], m)` | `free -= m*c`（平空成本） | sub Short `-m*c` 减少 → TW `+m*c` | **0** |
| `rec_add(parent[Long], give)` | `free -= give*c` | parent Long `+give*c` | **0** |
| 合计 | `free += m*c - give*c` | `+give*c + m*c` | `0` |

**逐步展开：**
- rec_reduce(sub[Short])：
  - `free -= m*c`（买回空头需付现金，Short 方向）
  - sub 的 NAV 贡献为 `-u*c`，减少 m units → 贡献从 `-(u)*c` 变为 `-(u-m)*c`，增加 `+m*c`
  - 半步 TW 变化：`-m*c + m*c = 0` ✓
- rec_add(parent[Long], give)：
  - `free -= give*c`
  - parent 的 NAV 贡献增加 `+give*c`
  - 半步 TW 变化：`-give*c + give*c = 0` ✓
- **整体 TW = 0**，与 `give` 值无关（sb≠pb 时 give≠m，但两半步各自中性）

`prove_tw_neutral` 在每次 `add` 调用后 panic 守卫，容差 `1e-4 * |tw_pre|`，为数值精度冗量。
5 个 orbit9 单测全部通过，其中 `add_部分买回_不污染父向极性` 和 `add_short_τ镜像_父空卖回对称` 直接验证 TW 中性。

**`prove_sink_recover_balance` 的范围**：Codex 确认该守卫（`on_recover` 调用）仅在 `fn recover` 内触发。
`add` 调用 `guards.note_op("add")`（触发源记录），不调用 `on_recover`——
因为 `add` 是「部分平短差升回」，与 recover「整条配对清空」的守恒不同，
两者都不会产生双计（注释行 1440 明示：「add 是部分平短差升回 ⇒ 不调 on_recover（整条配对守卫）」）。

---

## 综合结论

**异质审计 PASS：四质询点均无真否定。**

| 质询点 | 结论 | 说明 |
|--------|------|------|
| Q1 add = 543 word 非新原子 | PASS | rec_reduce+rec_add 复用，与 recover 同构，m 量不同 |
| Q2 O3 τ镜像莫比乌斯不适用 | PASS + MEDIUM-obs | 操作层对称成立；账本层不对称为 recover 既有设计，非 add 引入 |
| Q3 账本过滤在 route 之后 | PASS | route_bsp 无 account_reduce 调用，原子内自带核算 |
| Q4 NAV 无双计 | PASS | 两半步各自 TW 中性，prove_tw_neutral 守卫通过 |

**MEDIUM 观察（不阻塞 #52 B-cryst）：**
`account_reduce(mob, ...)` 在 add_short（pdir=Short，mob=Long）时更新 `core_cost_basis`，
即空头卖回的 PnL 进入降成本账本而非 `short_leg_pnl`。
这是与 recover 共享的三阶段会计设计，不是 add 的独立引入。
下游 B-cryst 应在结晶时注明：O3 add_short 的 PnL 路由与 O3 add（多头买回）不对称，
原因是三阶段会计以 Long/Short 方向划分，不以 buy/sell 操作划分。

---

## 结果包（简化版——纯技术性产出）

**结论：** codex 可达（gpt-5.5），四质询点 PASS，一个 MEDIUM 观察（账本层不对称为既有设计，非 add 引入）。

**边界条件：**
- Q1 翻转条件：add 引入跨 parent 级别以上的仓位归还路径（当前 prove_sink_descends 守卫禁止）
- Q2 翻转条件：账本路由按操作类型（buy/sell）而非仓位方向（Long/Short）分支（当前设计选择以方向分支）
- Q3 翻转条件：route_bsp 添加账本状态前置条件（当前代码无此分支）
- Q4 翻转条件：rec_reduce 或 rec_add 中 free 的符号约定与 nav 中 sign(d) 的符号约定不一致（当前两处一致）

**影响声明：**
- 本审计不修改任何代码或谱系文件
- B-cryst（#52）可无条件推进
- MEDIUM-obs 建议在 B-cryst 的结晶备注中标注 add_short 的 PnL 路由语义
