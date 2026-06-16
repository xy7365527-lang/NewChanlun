# 矛盾上浮：D 对称化修复暴露 `confirmed_root` 的 raw 残余——"E 用 nf"裁决的适用范围未定

> 上浮人：本 session（2026-06-16）
> 触发：用户任务 1（修 D 回补 raw→nf）+ 任务 2（扫 located/raw 混用）。修 D 后跑 BTC L3
> 对比，发现修复让 BTC **更差**（+340.9%→+136.8%，强平 0→216，子空头净亏 −54k→−195k），
> 根因不是 D 修错，而是 D 修复**只对齐了 E 的一条路径**，暴露了 E 另一条路径（`confirmed_root`）
> 的 raw 残余。
> 分类（四分法）：**语法记录**（§9"根的其他卖点走 E"中"买卖点"= raw 还是 helix，引擎从未
> 显式裁决）∧ **真矛盾**（编排者"E 用 nf"裁决 vs `confirmed_root` raw vs `n7` 测试编码，
> 三者不可同时成立）。
> 等级：L0（结构对称性）+ L3（BTC 4.625M bar 实测，否定性结果）。

---

## 1. 停下来：为什么这不是可自决的定理或行动

用户任务 1 明确"D 用 nf（和 E 的 nf_sell 对称）"，我已实装（`prove_d_symmetric` 守卫，
build+test 绿，375 测试通过）。但任务 2 要求扫"E/D 不对称、located/raw 混用"，扫出 E 自身
**内部不对称**：

- E 非根 spawn 用 `nf`（helix 确认）；
- E **根** spawn（`confirmed_root`）用 `raw sig.sell_any`。

修 D（raw→nf）前，D(raw) 与 `confirmed_root`(raw) 是**对称的**（都 raw）。修复后 D=nf 而
`confirmed_root` 仍 raw ⇒ 把系统从"全 raw 对称"推到"spawn-raw / recover-nf 不对称"。

裁决 `confirmed_root` 该用 raw 还是 nf = 裁决"§9 根的其他卖点走 E"里"买卖点"的存在论（raw
任意信号 vs helix 向心确认），且直接划定编排者"E 用向心确认 nf"裁决的**适用范围**（是否含
根降成本路径）。这超出工位自决——既是定义问题（§9 原文语义），又是已有裁决的范围澄清，且
有测试编码了 raw 行为。故上浮。

---

## 2. 精确描述矛盾

### 2.1 三命题不可同时成立

- **P1（编排者 2026-06-16 裁决，已实装）**：买卖点严格形式 = helix 向心确认（nf），非 raw。
  E 用 nf 开子空头，D 必须用同等 nf 回补。
- **P2（`confirmed_root` 现状，本文件 1446 行）**：根多头降成本经 `is_root ∧ sig.sell_any.get(ladder)`
  = **raw 任意卖点信号** 触发 spawn（无 helix 确认）。
- **P3（`n7_sublevel_sell_spawns_cost_reduction` 测试编码）**：根自层 raw 卖点（sell_any@4，
  无内层 type1 卖历史 ⇒ nf_sell 不 fire）**应** spawn 子空@3。测试断言 `spawns[3]==1`。

P1 要求"E 全路径用 nf" ⊥ P2"根路径用 raw" ⊥ P3"测试要求 raw 路径产出 spawn"。
若 P1 适用于根路径 ⇒ `confirmed_root` 须删/改 nf ⇒ P3 测试失败（须重写）。
若 P1 不适用于根路径 ⇒ `confirmed_root` raw 合法 ⇒ 但 D 修复（P1 应用于子路径）就制造了
spawn-raw/recover-nf 不对称（§3 L3 证据），D 是否也该退回 raw？

### 2.2 接受 P1-全路径（confirmed_root→nf）则：

- E 全部 spawn（根 + 非根）只在 helix 确认卖点触发 ⇒ spawn 变稀少（与 D recover 稀少同频）
  ⇒ 重新对称（spawn-nf / recover-nf）。
- `n7` 测试须重写（其 raw 假设是被否定的行为）。
- BTC：根降成本大幅减少（强牛中 helix 卖点罕见）⇒ 可能更接近"满仓恒持"（踏空 regime 下
  反而可能改善——待 L3 复测）。

### 2.3 接受 P2-根例外（confirmed_root 保持 raw）则：

- 必须承认 D 修复制造的 spawn-raw/recover-nf 不对称是**真实缺陷**（§3）⇒ D 应退回 raw
  （回到全 raw 对称，BTC +340.9%）⇒ **任务 1 的 D 修复应撤销**。
- 即"E 用 nf"裁决只适用于…哪里？若 E 根用 raw、E 非根用 nf、D 用 raw，则 D 与 E-非根 又不
  对称（回到用户任务 1 的原始抱怨）。三方无一致解 ⇒ 须重新定义对称性的锚点层。

### 2.4 根因：对称性锚点未定

用户任务 1 把对称性锚在"E 的 nf_sell 路径"，但 E 不是单一标准——E 有 nf（非根）+ raw（根）
两个标准。"和 E 对称"本身歧义（对齐哪条 E 路径？）。这是**语法记录缺口**：引擎从未显式声明
"降成本 spawn / 回补的统一买卖点标准是 located/nf 还是 raw，根与非根是否同标准"。

---

## 3. L3 证据（BTC 4.625M bar，2017–2026，否定性结果）

| 指标 | D=raw（修复前=全 raw 对称） | D=nf（修复后=spawn-raw/recover-nf） | Δ |
|------|------|------|---|
| strat% | +340.9 | +136.8 | −204.1pp |
| 强平数 | 0 | 216 | +216 |
| 子空头净亏 | −54,322 | −194,697 | −140,375 |
| 多头腿数 | 43 | 1 | −42 |

修复后 216 强平 = 子空头"raw 频繁开 / nf 稀少平"的堆积签名。这是**否定性 L3 结果**
（formalization-validity-domain.md：缩小有效域，比确认性更有价值）——它否证了"单边对齐 D 到
E-nf 路径即恢复对称"的假设，证明对称性须**全路径统一**（根与非根、spawn 与 recover）。

注：D 修复**通过 8 prove 零 panic**（必然性 L2 仍成立）⇒ 按编排者"回测是有效域读数非验收
标准"裁决，D 修复**未违反任何必然性**，是合法产出；L3 否定性是 regime 有效域边界，不是
panic。本上浮**不主张撤销 D 修复**，而是请求裁决 `confirmed_root` 标准以**完成**对称化。

---

## 4. 请编排者裁决

**Q1（核心，语法记录）**：降成本 spawn / 回补的统一买卖点标准是什么？根降成本（`confirmed_root`）
与非根（nf）、spawn 与 recover，是否必须同一标准？

候选解：
- **(A) 全 nf**：`confirmed_root`→`nf_sell`（删 raw 路径）；`n7` 测试重写；E/D 全路径 nf 对称。
- **(B) 全 raw**：D 退回 raw（撤销任务 1）；E/D 全路径 raw 对称（BTC +340.9%，但违背 P1）。
- **(C) located 链**：spawn/recover 都升级到 located 级联链（最强 N5/N6）——但违 N7（降成本
  不需 pending），且 segment 子无链。
- **(D) 根例外保留**：承认根降成本 raw 是 §9 有意的有效域设计，D 用 nf 是子级别有意的有效域
  设计，二者**本就不该对称**（根 = 主降成本入口须敏感，子 = 回补须严格）——则任务 1 的
  "和 E 对称"前提被否定，D-nf 是独立正确的子级别裁决，confirmed_root-raw 是独立正确的根裁决。

**Q2（依附 Q1）**：若选 (A)/(C)，E-short-spawn（空子 spawn 多孙）被 D shadow 成死分支（§4
of 审计），是删除还是另设触发？

---

## 5. 当前状态（本 session 已做）

- 任务 1 D 修复已实装（`self_level_counter_fire` + `prove_d_symmetric`），build+test 绿。
- BTC L3 对比已跑（本文件 §3）。
- 审计文档：`analysis/unn_de_fc_symmetry_audit.md`。
- **未**改 `confirmed_root`、**未**删 E-short-spawn、**未**撤销 D 修复——等本裁决。

## 6. 谱系引用

- `project_unn_btc_spawn_throwback`（E spawn 强牛过度=踏空根因，333 子空头亏−54322）——
  本 L3 与之吻合（修复前 −54322 复现），且揭示其机制 = confirmed_root raw 高频 spawn。
- `project_t14_t5_root_flip_necessity`（必然性累积不被 L3 否定，L3=有效域读数）——同范式。
- `.chanlun/escalations/2026-06-15-confirm-arming-differance.md`（前向 vs 回溯 confirm 存在论）
  ——同属 confirm/buysellpoint 严格形式的定义层未结算簇。
