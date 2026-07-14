# P2 R2 实装审查报告
**审查者**: p2-review（异工位，不修改代码）
**审查时间**: 2026-07-01
**被审对象**: p2-impl(#32) 的 R2 实装
**依据文件**:
- 定稿方案: `.chanlun/review-results/p2-plan-20260701.md`
- Codex 七护栏: `.chanlun/review-results/codex-review-20260701-2251.md`

---

## 构建与测试结果

```
cargo build       → SUCCESS（增量 0.03s，0 error，0 warning 块阻断）
cargo test 汇总:
  theta_v0::classifier::signal      28/28 pass，3 ignored（性能）
  theta_v0::strategy::interp        25/25 pass，3 ignored
  theta_v0::classifier::divergence  28/28 pass
  theta_v0::closed_loop::transition 15/15 pass
```

---

## 逐护栏审查

### G1：candidate_dir 回退是严格零 bit

**结论**: PASS

**实证**（`interp.rs`）：

```rust
fn candidate_dir(point: &BspPoint) -> VoiceSide {
    let bits = &point.bits;
    let base = root_sel(RootCandidates {
        long_trigger: bits.conf_plus(),
        short_trigger: bits.conf_minus(),
    });
    if !bits.conf_plus() && !bits.conf_minus() {
        if let Some(side) = point.struct_break_dir {
            return match side {
                Side::Long => VoiceSide::Long,
                Side::Short => VoiceSide::Short,
            };
        }
    }
    base
}
```

守卫条件是 `!conf_plus() && !conf_minus()`，NOT `root_sel()==Flat`。

(1,1) 双触发冲突（conf_plus=true AND conf_minus=true，root_sel 返回 Flat）绕过守卫，保持 Flat——测试 `double_trigger_conflict_stays_flat_not_recovered` 验证通过。

**边界条件**: 若将守卫改为 `root_sel()==Flat` 则结论翻转（(1,1)冲突被误恢复）。

---

### G2：class_index 冻结（struct_break_dir 不进 bucket key）

**结论**: PASS

**实证路径**:
- `bsp.rs`: `pub struct_break_dir: Option<Side>` 字段在 `BspPoint` struct 中
- `bsp.rs` doc 注释明确: "NEVER enters BspBits/MuClass/class_index/bucket key"
- `MuClass::class_index()` 仅读 `level/delta/i_class/parent_dir/short_swing/position` 六位
- 无任何 `struct_break_dir` 引用路径通向 `class_index()`
- 测试 `struct_break_dir_recovers_direction_without_touching_class_index` 验证：Some(Long) 零 bit 候选 `class_index()==0`

**边界条件**: 若将 `struct_break_dir` 加入 `BspBits` 或 `MuClass` 序列化，bucket key 改变，bit-exact battery GOLDEN 翻转（本次 GOLDEN 更新 **没有** 因 bucket key 改变）。

---

### G3：Weak_Θ 层位 + buy1 诚实命名

**结论**: PASS（含已申报缺口）

**buy1 命名审查**:
- `divergence.rs` `segments_diverge` 函数顶部注释：明确标注"MACD-area 仅作背驰代理，第17课原文指 MACD 为辅助，力度判据尚未接通生产"
- buy1/sell1 语义未膨胀为"完整背驰"

**Weak_Θ 原语审查**:
- `divergence.rs` 实装 `WeakThetaMode`（MacdArea/Dif/PriceAmplitude/Lex）、`weak_theta()`、`ForceFeatures`、`segment_dif_peak()`、`segment_price_amplitude()`、`segment_price_speed()`、`force_features()`
- Lex 模式: DIF 主判据 ▷ area 次判据（第17课/第34课顺序）
- 测试 `weak_theta_lex_dif_dominates_area_secondary` 通过

**已申报缺口（out-of-R2-scope）**:
- `selector.rs` / `econ_positive.rs` 中 **无任何** `WeakTheta`/`ForceFeatures`/`weak_theta` 调用
- grep 验证: `grep -rn "WeakTheta\|weak_theta\|ForceFeatures\|force_features" rust/src/theta_v0/` 除 `divergence.rs` 外零命中
- p2-impl 在代码注释中明确标注此为"原语实装 ✓，端到端接通 ✗（标 gap）"
- 不接通原因诚实申报：`assemble_gamma_with_tower` 仅有 `BspPoint` 作用域，无 segment close 序列供 amplitude/speed 计算

**边界条件**: 若视"force features 端到端接通"为 R2 完成的必要条件，则此护栏 FAIL（需返工）。若接受 p2-impl 的 out-of-R2-scope 申报，则为 PASS（缺口留给独立工位）。

---

### G4：GOLDEN 诚实更新

**结论**: PASS

**实证**:
- `signal.rs` 中 `bit_exact_battery_digest` GOLDEN 从 `0x37d2_45a7_cdc5_505a` 更新为 `0x56ed_dd65_1c59_5733`
- 更新原因：`BspPoint` 增加 `struct_break_dir` 字段，`derive(Debug)` 导致 Debug 输出变化
- 注释明确标注："BspPoint Debug 扩展字段导致字符串变化，六 bit 值本身不变"
- 六 bit 值不变通过 per-case 测试 `extract_signals_bit_exact_vs_orig_per_case` 逐案验证
- 并非通过自定义 Debug 隐藏字段（隐藏字段 → GOLDEN 不变但行为静默改变，此方式反而更危险）

**边界条件**: 若六 bit bucket key 值改变，per-case 测试会失败，GOLDEN 更新不能遮掩。

---

### G5：入口传播（extract_signals 路径）

**结论**: PASS

**实证**:
- `extract_signals_with_features` 和 `extract_signals_with_hist_features`（旧的 sidecar 元组路径）已 **删除**
- 唯一保留路径: `extract_signals` → `extract_signals_with_hist` → `judge_first_cached` → `make_first_point(source_index, bits, pivot_price, struct_break_dir)`
- 生产调用链: `econ_positive.rs` → `assemble_gamma_with_tower` → `extract_signals_with_hist(...)` → `BspPoint { struct_break_dir: Some(...) }` → `candidate_dir(point)` 读取
- struct_break_dir 在生产路径 **真实流动**

---

### G6：选择偏差声明收窄

**结论**: PASS

**实证**:
- 代码注释 + doc 仅声明：几何门 ∧ A/C 可配对的结构突破候选不再因 MACD 预删
- 未声称"Γ_struct 全量无偏"（Γ_struct 是所有结构突破候选，几何门 ∧ A/C 可配对是子集）
- L2 有效域标注：选择偏差消除的有效域 = 几何门通过 ∧ A/C 可配对，认识论等级 L1（合成管线验证），需 L2 真实数据验证

---

### G7：StructBreakFeature 死代码 / force features 生产接通

**结论**: 分拆为两部分

**G7a：StructBreakFeature 死代码删除 → PASS**
- `signal.rs` 中 `StructBreakFeature` struct **已完全删除**
- `judge_first_cached` 返回 `Option<BspPoint>`（旧元组 `.0` 丢弃 sidecar 已消除）
- 无残留 dead field

**G7b：force features 生产接通 → HONEST GAP（未接通）**
- `ForceFeatures`/`weak_theta`/`WeakThetaMode` 仅在 `divergence.rs` 内部存在
- grep 验证（已执行）: 全仓库 theta_v0/ 目录下除 `divergence.rs` 外零调用
- `selector.rs` 无任何 force primitives 消费
- p2-impl 诚实申报为 out-of-R2-scope（需独立大工位改造 Candidate pipeline）

**边界条件**: 若 codex R3 审计认定"force features 未接通=R2 未完成"，则 G7b FAIL，需返工。

---

## 综合评定

| 护栏 | 结论 | 类型 |
|------|------|------|
| G1 candidate_dir 严格零 bit | PASS | 硬约束，已闭合 |
| G2 class_index 冻结 | PASS | 硬约束，已闭合 |
| G3 Weak_Θ 层位 + buy1 降格 | PASS（含缺口） | 软约束，原语实装，端到端未接 |
| G4 GOLDEN 诚实更新 | PASS | 硬约束，已闭合 |
| G5 入口传播 | PASS | 硬约束，已闭合 |
| G6 选择偏差声明收窄 | PASS | 软约束，已闭合 |
| G7a StructBreakFeature 删除 | PASS | 硬约束，已闭合 |
| G7b force features 接通 | HONEST GAP | 软约束，out-of-R2-scope 申报 |

**所有硬约束（G1/G2/G4/G5/G7a）已闭合。**
**软约束 G3 原语已实装但端到端未接；G7b 诚实申报为独立工位。**

**审查结论**: 可进 #34 codex 审计。codex 审计需额外关注：
1. G7b 的 scope boundary 是否被 codex 接受（force features 仍为死代码的认识论影响）
2. G3 Weak_Θ 原语的 L1 等级是否满足 P2 完成标准（还是需要 L2 验证后才能接通）

---

## RTAS 影响声明

- 改动文件: `signal.rs`, `bsp.rs`, `interp.rs`, `divergence.rs`, `transition.rs`, `econ_positive.rs`, `mod.rs`
- struct_break_dir 已在生产路径流动（L1 合成管线验证）
- force features 为 L1 dead code（无生产消费者）
- bit-exact GOLDEN 诚实更新，六 bit bucket key 不变（per-case 测试保证）
- 无其他回归
