# qthetav1-impl｜q_Θ v0→v1 升级消费 σ_higher 实装验收（g2）

**工位**：swarm/ws-qthetaimpl ｜ topo_address: swarm/ws-qthetaimpl ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix ｜ 基线：54ec3b6384（g1 冻结之后）
**parent_callback**：main ｜ **依据**：prereg-rev4-qtheta-sigma-higher-20260705.md（g1 冻结）
**认识论等级约定**（231号）：本报告标注 L0（定义推导）/ L1（代码静态核对）/ L2（经验/OOS，不在 g2 范围）。

---

## 〇、验收结论（PASS）

g2 验收四点**全部满足**：

| 验收点 | 状态 | 证据 |
|---|---|---|
| 1. w_dir 分级查表（3 CASE + 3 套预注册 config 选） | ✅ | `coverage.rs::dir_weight` + `theta_dir_slot`；`config.rs::ThetaDirPreset` |
| 2. ShortDiff 合法性断言测试（w_dir(ShortDiff)≡1.0） | ✅ | `w_dir_shortdiff_exempt_across_all_presets`（三套预设 × 18 类 ShortDiff 子集） |
| 3. neutral 套 bit-exact 自检（== v0 基线 af8910d062） | ✅（单元级根因） | `w_dir_neutral_is_identity_for_all_18_roles`（全 18 类 × depth ⟹ w_dir≡1 ⟹ leg_target==v0） |
| 4. cargo test --release --lib theta_v0 全绿 | ✅ | 全 lib 1511 passed / 0 failed（基线 1508 + 3 新测试），见 §五 |

**未 commit**（任务约束）——代码留工作区，本报告落 `.chanlun/review-results/qthetav1-impl-20260705.md`。

---

## 一、结论（实装内容）

### 1.1 w_dir 函数（coverage.rs，prereg-rev4 §4.1 逐 CASE 实装）

```
s_e = base_units × w_depth(depth) × w_dir(ℓ, δ_e, σ_higher(e), role(e))
```

`dir_weight(role, depth, config)` 三 CASE（优先序）：

- **CASE 1 ShortDiff 豁免**：`role.v == ShortDiff ⟹ 1.0`（§7.5 `s_g=s_α` 定理 1，优先于一切预设）。
- **CASE 2 根级豁免**：`σ_higher == 0`（`Vertical::Ambient`，父容器无方向）`⟹ 1.0`。
- **CASE 3 分级查表**：otherwise `⟹ Θ_dir[ℓ][sign(δ_e · σ_higher)]`。

σ_higher 从 `OperationRole` 隐式解码（prereg §3.3，**无新数据源**）：
- `FollowParent`：δ_e = σ_higher ⟹ σ_higher = `dir_sign(delta)`（+δ_e）
- `ShortDiff`：δ_e = −σ_higher（CASE 1 已截）
- `Ambient`：σ_higher = 0（CASE 2）

**代码性质注**（L1，非 bug）：本角色分类下 CASE 3 仅 `FollowParent` 可达 ⟹ sign(δ·σ_higher) 恒 +1
（sign=−1 的腿按定义即 ShortDiff，CASE 1 截）。三套预设的 sign=−1 槽存在但经 role 路径不可达——
是 `Vertical` 三分类（Ambient/FollowParent/ShortDiff 完全覆盖父方向关系）的性质，非实装缺陷。

### 1.2 三套预注册（config.rs::ThetaDirPreset，prereg-rev4 §4.3）

```rust
pub enum ThetaDirPreset {
    Neutral,                          // Θ_dir_neutral：w_dir≡1.0（v0 bit-exact 对照，default）
    Follow { eta_adv: Vec<f64> },     // Θ_dir_follow：顺上级保权，逆上级降权 η_adv[ℓ]
    Adversary { eta_same: Vec<f64> }, // Θ_dir_adversary：逆上级保权，顺上级降权 η_same[ℓ]
}
```

- `VoiceConfig` 增 `theta_dir: ThetaDirPreset` 字段，**default = Neutral**（保 frozen Θ v0 bit-exact）。
- η 数值表冻结是 g3 跑数前独立步骤（135号，prereg §4.3）——g2 只冻结构。
- 越界 level 视 1.0（保权，不意外零化腿单位；与 `depth_weight` 越界返 0 语义有意区分）。

### 1.3 两个 leg_target 变体同步升级（prereg-rev4 §3.2）

- `leg_target`（coverage.rs，主路径）：`w = depth_weight × dir_weight`。
- `leg_target_two_segment`（双段变体）：同步升级，保 bit-exact == leg_target（注释自述约束保持）。
- `role` 计算前移至 `w` 之前（无副作用重排，语义不变）。

---

## 二、定义依据

- **prereg-rev4 §4.1**（g1 冻结）：w_dir 三 CASE 函数形式——逐 CASE 实装，未偏离。
- **prereg-rev4 §4.3**（g1 冻结）：三套预注册结构（follow/neutral/adversary）——`ThetaDirPreset` 三变体一一对应。
- **prereg-rev4 §3.3**（g1 冻结）：σ_higher 经 role 隐式可得（不需新数据源）——`dir_weight` 内 match role.v 解码，无新参数透传。
- **prereg-rev4 §5.1 定理 1**（§7.5 `s_g=s_α`）：ShortDiff w_dir≡1.0——CASE 1 优先于 CASE 3，三套预设下均成立（测试 `w_dir_shortdiff_exempt_across_all_presets` 覆盖）。
- **prereg-rev4 §6.2**：neutral 套唯一 bit-exact 保留情形——测试 `w_dir_neutral_is_identity_for_all_18_roles` 单元级证明。

---

## 三、边界条件（结论翻转）

- **g2 实装翻转**（= bug）：仅当 neutral 套下存在某 (role, depth) 使 `dir_weight ≠ 1.0`——已被 `w_dir_neutral_is_identity_for_all_18_roles` 全 18 类 × 3 depth 否证。
- **formal-chain 翻转**（非 bug，选择类）：仅当编排者裁定采线性连续 w_dir（090号声明膨胀代价）或 ShortDiff 也缩放（§7.5 违反代价）——皆非 g2 范畴。
- **不翻转**：σ_higher 经 q_Θ 通道 / 禁一刀切 / §7.5 ShortDiff 定义 + s_g=s_α——四者皆 formal-chain 已结算，g2 实装严格遵循。
- **有效域边界**：σ_higher=0（Ambient）腿 w_dir=1.0（CASE 2，无 σ_higher 可消费）；越界 level 保权 1.0。

---

## 四、下游推论

- **对 g3（跑数工位）**：三套预注册（follow/neutral/adversary）可并行 OOS；η_adv(ℓ)/η_same(ℓ) 数值表须跑数前冻结（135号）；**首跑 Θ_dir_neutral 验证 OOS == v0 基线 af8910d062**（单元级已证 w_dir≡1，g3 跑经验 OOS 确认端到端传导链无意外）。
- **对 Π_max-full OOS（af8910d062）**：default config = Neutral ⟹ v0 基线 OOS 不变；切 Follow/Adversary 才产生新信号集（定理 2，bit-exact 不保留）。
- **对 leg_target 调用方**：default Neutral 下输出 bit-exact == v0（interp.rs:1881 测试调用、strategy_target_legs 生产路径均不受影响）。
- **对 135号冻结纪律**：g2 实装依赖 g1 prereg-rev4 冻结哈希；η 数值冻结是 g3 跑数前独立步骤。

---

## 五、测试证据（L1 + 运行实测）

### 5.1 新增三测试（coverage.rs::tests）

| 测试 | 验收点 | 覆盖 |
|---|---|---|
| `w_dir_shortdiff_exempt_across_all_presets` | 点 2 | 3 套预设 × {First,SameFollow,SameReverse}×ShortDiff×{±δ}×depth∈{0,1,2} ⟹ 全 1.0 |
| `w_dir_neutral_is_identity_for_all_18_roles` | 点 3 | Neutral × 全 18 类角色（3H×3V×2δ）× depth∈{0,1,2} ⟹ 全 1.0（== v0 bit-exact 根因） |
| `w_dir_case3_lookup_and_root_exemption` | 点 1 | Follow/Adversary 非中性槽生效 + Ambient 根级豁免 + 越界 level 保权 |

### 5.2 cargo test --release --lib（全 lib，single-threaded）

```
test result: ok. 1511 passed; 0 failed; 125 ignored; 0 filtered out
```

= 基线 1508 + 3 新测试，**0 failed**。

### 5.3 theta_v0 子集（cargo test --release --lib theta_v0，single-threaded）

```
test result: ok. 934 passed; 0 failed; 91 ignored; 0 filtered out (611 non-theta_v0 filtered)
```

### 5.4 已知并行 flake（非本工位引入）

并行跑（默认多线程）时 `theta_v0::backtest::runner::tests::opsem_dump_env_gated_bit_exact`
**偶发**失败，失败点 `runner.rs:3082` `trades.jsonl 非空`（文件读 expect，非值断言）。
- **隔离单跑 PASS**（`--exact --test-threads=1`）。
- **重跑 PASS**（同一并行命令第二次 934 passed / 0 failed）。
- 根因：该测试 `std::env::set_var("OPSEM_DUMP_DIR", ...)` 全局 env 突变，并行兄弟线程竞争——**与本工位无关**（default Neutral ≡ v0，若本工位改了 sizing 输出，失败点应在 3069/3070 值断言而非 3082 文件读）。

---

## 六、影响声明

### 6.1 代码改动（工作区，未 commit）

- `rust/src/theta_v0/config.rs`：增 `ThetaDirPreset` 枚举（3 变体）+ `VoiceConfig.theta_dir` 字段（default Neutral）+ Default impl。
- `rust/src/theta_v0/strategy/coverage.rs`：
  - 增 `pub fn dir_weight(role, depth, config) -> f64`（三 CASE）+ 私有 `theta_dir_slot(preset, depth, sign)`。
  - `leg_target` + `leg_target_two_segment`：`w = depth_weight × dir_weight`（role 计算前移）。
  - 更新 `LegTarget.units` 字段 doc + `leg_target` 函数 doc（v0→v1 公式，诚实声明）。
  - 增 3 个 `#[test]`。
- import：coverage.rs `use super::super::config::{RiskConfig, ThetaDirPreset, VoiceConfig}`。

### 6.2 bit-exact 影响范围

- **default config（Neutral）**：w_dir≡1.0 ⟹ `leg_target` 输出 == v0 ⟹ `net_target_units`(p̃) / `pi_theta_position`(p*) / `schedule_order`(订单) / ledger / 信号集**全不变**（prereg-rev4 §6.2）。1511 测试全绿为证。
- **切 Follow/Adversary**：s_e 改 ⟹ p̃ 改 ⟹ p* 改 ⟹ 订单/ledger/信号集改（定理 2，bit-exact 不保留）——需新预注册（135号），是 g3 范畴。

### 6.3 未触动

- J_Θ / K_Θ 通道（prereg-rev4 §1.3 否决）：σ_higher 不经此二通道，本工位未碰 `pi_theta_position` / K_Theta risk gate 逻辑。
- depth_weight（voice.rs）：v0 既存，未改（w_depth × w_dir 正交，prereg-rev4 §4.4）。
- η_adv(ℓ)/η_same(ℓ) 具体数值：g3 跑数前冻结，g2 不冻。

---

**g2 实装验收结束**。q_Θ v0→v1 升级 = `s_e = base_units × w_depth × w_dir`，w_dir 分级符号查表（ShortDiff §7.5 豁免 + 根级 Ambient 豁免 + (ℓ, sign(δ·σ_higher)) 分级查表）× 三套预注册（Neutral default 保 v0 bit-exact / Follow / Adversary）。neutral 套 bit-exact == v0 已单元级证明（全 18 类 × depth，w_dir≡1）。全 lib 1511 测试全绿。代码留工作区未 commit，待 main 裁决。ready-for-shutdown。
