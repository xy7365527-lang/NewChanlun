# XZD C2-only 消费口径复审（#148 后 C3 脱 0 新基线）

- **任务**：Task #170（ws-xzdc3b 接力 ws-xzdc3）
- **日期**：2026-07-03
- **文件**：`rust/src/theta_v0/backtest/econ_positive.rs`
- **背景**：#148 中枢升级语义落地（f9b3e41636）后 XZD C3 脱 0——BTC 全历史 `c3_new_center_exists` 0→13、`breakout_ok` 0→3；#56 死门已按自设翻转条件重封新基线 C3 (13,3) + lvl>=2 (217,433)（07ad9d2c1c）。
- **编排者令双条**：①完整零简化，fable 写代码；②复审判据必须对照 PDF 原文（`区间套.pdf` + `一类买卖点.pdf` Q4 承接路由段），口径选择以 PDF 原文为判据源，原文未明确则 codex 提裁。

---

## 一、结论

**三个 XZD 证书消费点全部走 `XzdEvidence::gate_pass()` 单一来源，不存在「C3 恒 0 时代」残留的分散硬编码 C2-only 假设。C3 脱 0 后行为已由 `gate_pass()` 内部的 level 分级逻辑正确承载，无需修复。**

口径选择空间（level>=2 C2-only 保留 vs C3 硬门全 level 启用）**属 codex 裁定域**——PDF 原文（Q4）只规定「XZD 是盘整背驰的承接通道之一」，未定义 XZD 内部 C2/C3 的 level 分级口径；现行 level 分级来自 codex #41/#44 裁定，非 PDF。故本工位**报数 + 提裁，不自决**（no-unnecessary-escalation「选择类」）。

**代码改动：0 行**（复审确认现状严格正确，无补丁可加——加任何东西都违反 no-patch-mentality）。前任遗留的 lvl>=2 C3 探针 hunk（3 处）经复审质量合格，保留。

---

## 二、XZD 证书消费点逐点复审

全仓 `XzdEvidence::gate_pass()` 消费点（`grep -rn` 全库确认，`runner.rs:1247` 的同名 `gate_pass` 是无关局部变量）：

| # | 位置 | 语境 | C2-only 硬编码残留？ | 判定 |
|---|------|------|---------------------|------|
| 1 | `econ_positive.rs:368` | 生产 `collect_signals` 二通道准入门（`GateCertificate::Xzd(ev) => ev.gate_pass()`） | 无 | ✅ 单一来源 |
| 2 | `econ_positive.rs:1240` | Q4 盘整背驰承接门 `pan_div_gate_pass` 通道2（`xiaozhuanda_confirm(...).gate_pass()`） | 无 | ✅ 单一来源 |
| 3 | `econ_positive.rs:3854` | dx 手写门探针 `acc_classification_level_hole_dx`（与生产 bit-exact 对拍，`ev.gate_pass()`） | 无 | ✅ 单一来源 |

**核心发现**：`gate_pass()`（`econ_positive.rs:1135-1137`）是唯一的 level 分级决策点：

```rust
pub(super) fn gate_pass(&self) -> bool {
    self.type2_confirmed && (self.level != 1 || self.c3_new_center_breakout_ok)
}
```

分级语义内置于此单一函数：
- `level == 1`：C2 ∧ C3(新中枢突破) 硬门（`c3_new_center_breakout_ok` 参门）。
- `level != 1`（含 lvl>=2 与 lvl==0）：C2-only（`c3_*` 字段不参门，纯诊断）。

三个消费点**都不重新实现分级逻辑，都委托给这个函数**。所谓「C3 恒 0 时代的消费逻辑」在代码层从未以「跳过/降级 C3 路径的硬编码分支」形式存在——C3 恒 0 是**数据事实**（`c3_new_center_breakout_ok` 计算恒返回 false），不是**代码假设**。因此 C3 脱 0（数据变化）后：
- `level == 1` 分支：`c3_new_center_breakout_ok` 从恒 false 变为可 true ⟹ 硬门自动可通行（无需改代码）。
- `level != 1` 分支：本就不读 C3 字段 ⟹ 行为不变。

**②盘整背驰承接路由（#145 PanDiv→N^δ/XZD）的 XZD 分支**：`pan_div_gate_pass`（消费点 2）的 XZD 通道也走同一 `gate_pass()`。C3 可用后，承接门的 XZD 通道在 level==1 从「C3 恒 0 ⟹ 恒靠 C2 单独判」变为「C2 ∧ C3 硬门」——但这是 `gate_pass()` 内部语义，承接门代码零改动、零硬编码假设。承接门注释（`econ_positive.rs:1199-1200`）已如实声明「level==1 C2∧C3 硬门 / 其余 C2-only」，声明与实际一致（no-patch-mentality 诚实性）。

---

## 三、PDF 原文对照（编排者铁律②）

### 3.1 `一类买卖点.pdf` Q4 盘整背驰承接段（p.7）

> Q4. 盘整背驰与第一类。裁决：标准第一类买卖点应锚定趋势背驰；**盘整背驰不能消失：它必须被某级别买卖点或小转大/区间套证书承接。** … Route it through `N^δ_{ℓ↓e}` or a "pan-divergence / small-to-large" certificate：`PanDiv^δ_ℓ ⟹ ∃e<ℓ, Conf^δ_e`，or：`PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}`。

**↔ 代码映射**：

| PDF Q4 判据 | 代码位置 | 一致性 |
|-------------|----------|--------|
| `PanDiv^δ_ℓ ⟹ ∃e<ℓ Conf^δ_e`（Nest 通道） | `pan_div_gate_pass:1220`（`descend_type1_anchor_depth(...).is_some()`） | ✅ 逐字对应 |
| `PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}`（XZD 通道） | `pan_div_gate_pass:1229-1240`（`xiaozhuanda_confirm(...).gate_pass()`） | ✅ 逐字对应 |
| 「任一通过 ⟹ 承接成立；两门皆闭 ⟹ 诚实丢弃」 | `econ_positive.rs:447-451`（`if !pan_div_gate_pass(...) { continue; }`） | ✅ 一致 |

**判据边界（关键）**：Q4 原文规定 XZD 是承接通道，但**未规定 XZD 通道内部 C2/C3 判据的 level 分级**。PDF 在此处把 XZD 当作一个已定义的证书原语（`XZD^δ_{ℓ↓e}`）引用，其内部构造（C1/C2/C3 各自作用与 level 依赖）不在 Q4 的规范范围内。

### 3.2 `区间套.pdf`（N^δ rung 定义域，佐证 XZD 无 depth 口径）

> 区间套的本体是 `J^δ_e ⊆ J^δ_{e+1} ⊆ … ⊆ J^δ_ℓ`（区间包含链）。递归步：`N^δ_{k↓e} = Cand^δ_k(J_{k-1},t) ∧ N^δ_{k-1↓e}`。

**↔ 代码映射**：区间套的 rung 是「子区间被父区间包含」的多级嵌套（`区间套.pdf` p.3-4、p.8 最终裁决 `J_child ⊆ J_parent`）。XZD 通道**不属于**区间套语义——它是「本级无一类精确点无法下沉定位」时的替代承接（知识库 L410）。故代码里 XZD 通道 `nest_depth=None`/`origin_level=None`（`econ_positive.rs:404-405`、`461`「同 Xzd 口径」）与 PDF 一致：XZD 无 rung 下沉概念。

### 3.3 PDF 判据源结论

**XZD 内部 C2/C3 的 level 分级口径在两份 PDF 中均未明确** ⟹ 按编排者令「原文未明确则 codex 提裁」。现行 level 分级（level==1 硬门 / 其余 C2-only）的判据源是：
- codex #41 裁定（level>=2 C2-only，前提是旧判据结构性死门）；
- codex #44 终局裁定(c)（level==1 C2∧C3 硬门，第43课「背驰后新中枢+反向突破」语义）。

这两条不是 PDF 原文，是异质裁决。C3 脱 0 后是否翻转分级（把 level>=2 也并入 C3 硬门）需回 codex 复审——见 §五。

---

## 四、测试与验证

### 4.1 XZD 单元测试（全绿）

`cargo test --release --lib xzd_`（初次运行，#166 破坏前）：

```
test xzd_c2_cross_entry_finds_cobsp_second ... ok
test pan_div_gate_xzd_channel_passes_via_type2_confirmed ... ok
test xzd_c3_last_sublevel_zs_type3_only ... ok
test xzd_gate_pass_level1_c3_breakout_hard_gate ... ok
test xzd_c3_new_center_breakout_cases ... ok
test result: ok. 5 passed; 0 failed
```

其中 `xzd_gate_pass_level1_c3_breakout_hard_gate`（`econ_positive.rs:1708-1722`）钉住了分级语义的全部四象限：
- `mk(1,true,true)` ⟹ 通过（level1 C2∧C3 真）
- `mk(1,true,false)` ⟹ 拒（level1 C3 假，硬门参门）
- `mk(2,true,false)` ⟹ 通过（level2 C2-only，C3 不参门）
- `mk(2,false,true)` ⟹ 拒（level2 C2 假）

单元测试已完整覆盖 C3 脱 0 后 level 分级的正确行为，无遗漏。

### 4.2 lvl>=2 C3 命中量化探针（被 #166 阻塞，待补测）

`acc_classification_level_hole_dx`（300K 窗 L2 探针，`--ignored`）**当前无法运行**：全库编译失败，根因是**并发工位 #166（A9-posnode，in_progress）的在飞半成品**——`interp.rs` 的 `ActiveLeg` 新增 `entry_certificate`/`generation` 字段（+67 行，未 commit），但 8 个消费点（`coverage.rs` ×8、`mutex.rs`、`persistent.rs`）未同步改，触发 E0063。

**这不属于 #170 任务范围**（不同文件、不同工位、在飞中），按 no-patch-mentality + 混批防护**不予触碰**。

lvl>=2 C3 命中探针的静态复审（替代运行）：
- 探针字段 `c3_new_center_exists`/`c3_new_center_breakout_ok` 是 `XzdEvidence` 恒有字段（`econ_positive.rs:1124-1127`），`build_gate_certificate` 对所有 level 都计算。
- lvl>=2 分支读取（`econ_positive.rs:3851-3852`）无条件可得，与 level==1 分支（3828-3829）同源。
- 探针只计数、`eprintln!` 报数（4128-4130），**不参门、零行为影响**（gate_pass 在 level!=1 不读这两字段）。

**结论**：探针逻辑静态正确。lvl>=2 的 C3 命中数字（codex 裁定的量化输入）待 #166 修复全库编译后补测——命令：
```
ECON_L2_MAX_BARS=300000 cargo test --release --lib acc_classification_level_hole_dx -- --ignored --nocapture 2>&1 | grep c3-breakout-dx
```
预期输出 `[c3-breakout-dx] lge2 routed=217 new_center_exists=? breakout_ok=?`（routed 已由死门重封锁定 217；exists/breakout_ok 是本次待测量）。

---

## 五、口径选择空间（→ codex 提裁，不自决）

**议题**：C3 脱 0 后，XZD 分级口径是否应从「level==1 硬门 / 其余 C2-only」翻转为「全 level C3 硬门」？

**为何是「选择」而非「定理」**：
- PDF 原文（§三）未规定 XZD 内部 C2/C3 的 level 分级 ⟹ 无法从 PDF 定义推导。
- 现行分级源自 codex #41/#44 裁定，其前提「level>=2 旧 C3 判据结构性死门」在 #148 升级重切后**在 level==1 已实证失效**（C3 脱 0）；level>=2 是否同失效，由 lvl>=2 探针数字（§4.2 待测）决定。
- `gate_pass()` 注释（`econ_positive.rs:1131-1134`）与死门断言注释（`econ_positive.rs:4225-4226`）均已声明「下游口径待裁（上浮 Lead，非本层自决）」——本工位继承此声明。

**codex 裁定所需证据包**（本工位已备齐结构证据，量化证据待 §4.2）：
1. PDF 原文摘录：`一类买卖点.pdf` p.7 Q4（XZD 是承接通道，未定义内部分级）；`区间套.pdf` p.3-8（XZD ∉ 区间套 rung 语义）。
2. 现行分级判据源：codex #41 + #44（异质裁决，非 PDF）。
3. level==1 实证：C3 脱 0，exists=13/breakout_ok=3（#148 默认窗基线，已锁死门断言 `econ_positive.rs:4229-4235`）。
4. level>=2 实证：**待 #166 修复后补测**（探针已就位，routed=217）。
5. 现行 dx 探针已内置分级翻转的裁断逻辑（`econ_positive.rs:4106-4116`）：`n_l1_c3==0` ⟹ 全域退化处置成立；`>0` ⟹ 应翻转为分级。此逻辑「只报数，是否翻转由后续工位/编排者裁定」。

**本工位不裁断**：翻转与否需 codex 权衡「level>=2 C3 命中率」（量化，待补）与「第43课语义在高级别的适用性」（概念）。这是价值判断，非逻辑必然。

---

## 六、结果包六要素

1. **结论**：三 XZD 消费点全走 `gate_pass()` 单一来源，无 C2-only 硬编码残留；C3 脱 0 后行为由 `gate_pass()` 内部 level 分级正确承载；代码改动 0 行；口径翻转议题上浮 codex。
2. **定义依据**：`一类买卖点.pdf` p.7 Q4（XZD 承接通道，`PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}`）↔ `pan_div_gate_pass:1229-1240`；`区间套.pdf` p.3-8（XZD ∉ rung 语义）↔ XZD 通道 `nest_depth=None`。XZD 内部 C2/C3 分级 PDF 未定义 ⟹ codex #41/#44 裁定域。
3. **边界条件**：若 lvl>=2 C3 命中率（§4.2 待测）显著 >0，则 codex 可能翻转分级为「全 level C3 硬门」，届时 `gate_pass()` 的 `self.level != 1` 条件需改。若命中率为 0，则现行 level==1-only 硬门维持。
4. **下游推论**：`gate_pass()` 是 XZD 分级唯一决策点 ⟹ 任何未来分级口径变更只需改此一函数，三消费点自动跟随（单一来源的架构收益）。#176（bsp_cand_type provenance）不影响本结论（provenance 是分桶归因，不改 gate 分派）。
5. **谱系引用**：codex #41（level>=2 C2-only 死门）、#44 终局裁定(c)（level==1 C2∧C3 硬门）、#55/#56（C3 恒 0=市场几何事实的翻转条件）、#148（升级重切触发 C3 脱 0）、#137（死门锁默认窗基线）、知识库 L410（小转大补充定位）、275 局部依赖（#166 阻塞判定）。
6. **影响声明**：本复审改动代码 0 行；确认现状严格正确；产出复审报告 1 份；上浮 codex 裁定议题 1 项（XZD 全 level C3 硬门口径）；标注 lvl>=2 探针数字待 #166 修复后补测（真实跨工位数据依赖阻塞，非本工位可自解）。

---

## 附：#166 阻塞诚实声明

本工位的 lvl>=2 C3 命中量化探针（§4.2）与全绿 `cargo test --release --lib`（编排者验收条件之一）**当前无法达成**，根因是并发工位 #166 的在飞半成品破坏全库编译。这是真实的跨工位数据依赖阻塞（我的探针运行依赖「全库编译通过」，该编译被 #166 未完成的 ActiveLeg 消费点改动破坏）。按 no-patch-mentality + 混批防护，**不触碰 #166 的文件**。#166 完成后：
1. 重跑 `cargo test --release --lib`（应全绿——本工位零代码改动，不引入新失败）。
2. 补测 §4.2 探针，填入 lvl>=2 C3 命中数字。
3. 连同 §五证据包一并交 codex 裁定。
