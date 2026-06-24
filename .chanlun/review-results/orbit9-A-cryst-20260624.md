# 结晶：A 工位 H⁰ 方向骨架（orbit9 task#39 / #43 / #47 / #51）

结晶工位：orbit9-A-cryst（task#51）｜日期：2026-06-24｜分类：依附 orbit9 9 轨道分类（**非新谱系**）

## 结晶内容（component 已落盘 + 验证状态）

A 工位 H⁰ 方向骨架 component 已落盘于 `rust/src/recursive_t/rec_engine.rs`（分支 `orbit9-A-h0skeleton`，commit `0190470722`）。core：`route_bsp` 新增 `flip_confirmed: bool` 参数 —— `enable_h0_skeleton`（env `T_ORBIT9_H0`，缺省 OFF）门控核心级反向 flip，仅 type1（走势完成 φ=0 seam，`view.t1buy/t1sell`）触发的反向才准 flip；非 type1 反向 ⇒ no-op + `n_h0_flip_blocked++`（骑走势，背驰段不翻核心）。τ 手性对称：同一 flip 分支无方向条件分支，t1buy↔t1sell 镜像（exhaustive #39：9=9 全对称）。

**验证状态**：
- **bit-exact = L1 已实证**（不是仅凭声明）：`cargo test --lib recursive_t::rec_engine` → 9 passed / 0 failed，含 H⁰ 四测试（① confirmed_type1反向_准翻、② 背驰段非type1反向_noop、③ off_非type1反向_flip_bit_exact、④ tau镜像_空头核心_confirmed与背驰段对称）。OFF 路径逐字不变。
- **A-review#43 = WARNING（可合并）**：概念六要素 PASS（587候选A τ手性骨架 + 541 P2 时序约束派生，非 ad-hoc），⑥要点全 PASS。遗留 **HIGH-1**：生产配置 `enable_trend_done_clear=true` 下 type1 反向走 A'' 清仓路径而非 route_bsp flip，H⁰ confirmed 门触达率存疑（若 A'' 优先则 ON≈全禁 flip，与 587 候选A 意图不符）—— **须在 D 工位 L3 验收前澄清**。MEDIUM-1：route_bsp 签名变更对 B 工位接口约束已在此声明（flip_confirmed=type1 信号，调用点 `view.t1buy/t1sell` 注入，非 route_bsp 内部读全局）。
- **A-hetero#47 = 降级（094号缺口记录）**：593 异质源全死，metadata.heterogeneous=pending-real-heterogeneous，不阻塞结晶但记缺口 —— 真异质审计（flip confirmed 门是否禁背驰段武装 / τ 手性群作用层良定义 / judge_divergence Some 门是否引 look-ahead）**未执行**。

## 认识论等级

- 代码结构 + bit-exact = **L1**（管线正确性，OFF=base 逐位，信息增量零但确认无回归）。
- **操作有效性 = L2/L3 待验**：H⁰ ON 是否改善 short_pnl（587候选A 意图）须 D 工位八标的（含 bear）L3 验收，当前**未验证**。HIGH-1 触达率疑点直接影响 L3 是否能观测到 ON≠OFF 的有效差异。

## 边界条件（结论翻转）

若生产路径 `enable_trend_done_clear=true` 且 A'' 优先于 flip 分支 ⇒ H⁰ ON 退化为"禁所有核心 flip"，confirmed 门从未实际触发，587 候选A 有效域坍缩 ⇒ 本 component 在生产配置下为死门。须 D 工位补 `enable_trend_done_clear=true` 集成测试或路径优先关系说明来证否此退化。

## 影响声明

`EngineConfig` / `TRoot` 各 +1 字段（`enable_h0_skeleton`、`n_h0_flip_blocked`）；`route_bsp` 签名 +`flip_confirmed`；on_bar 调用点注入 `view.t1buy/t1sell`。OFF 缺省 ⇒ 主路径零行为变更。**代码改动仅在 `orbit9-A-h0skeleton` 分支，未合入 B/main**。
