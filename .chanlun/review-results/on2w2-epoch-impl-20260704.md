# forest_epoch 架构件实装结果包（H6 O(n²) sound 修复落地）

**日期**：2026-07-04
**工位**：on2w2-epoch-impl（on2-sweep #183 后继）——按 `on2w2-epoch-design-20260704.md`（codex GO）实装
**前置**：设计 commit `d5dbedff92`（仅设计文档，未实装）
**认识论等级**：soundness = L0（代数/定义 + 完备枚举 + debug 全量对拍）；bump 率/加速 = L2（CL 8K/16K 真实数据）

---

## 1. 结论

`forest_epoch`（classifier 域 u64 单调计数器）实装落地。K_i 森林缓存命中判据从每 bar O(全塔) 的
`TreeKey::of_forest` 指纹降为 O(1) epoch 比较。bit-exact 铁律满足（全电池绿 + debug 逐 bar 对拍
0 divergence）。

**关键订正（实装暴露的设计缺陷，已修）**：设计 §3 的 E1/E2 **长度判据**（`reuse<len ∨ tail 非空）
在 CL 实测 bump 率 = **100%**（O(n²) 未消除，x_exp 仍 2.3）——因 frontier resume 每 bar
truncate+repush L0 尾段 / pop+重扫 upper 末窗，但 99.2% 复现**字节相同**的内容。改为**逐值内容判据**
（E1 比 `moves_tower_l0[reuse..]` vs 新 segments 投影；E2/E2a 合并比 `popped_upper` vs `tail_upper`）后，
bump 率降至 **2.17%**（vs of_forest 真变 0.78%，2.8× over-invalidate 来自 E4 空-L0 clear，sound-safe）。
这是设计 §7 明列的失败分支（"bump ≫ 0.8% ⟹ E1 门控失效 ⟹ 复查门控"）的正面落地，非绕过。

## 2. 定义依据

- **K_i / T_i 双视图**（A12 #160，settled）：`extract_carrier_forest`（K_i，读全塔含 L0）是 tower 纯函数
  ⟹ tower 内容不变 ⟹ 森林不变。forest_epoch 当且仅当 tower 任一级 `Vec<LeveledMove>` 字节变更时 ++。
- **递增点枚举**（设计 §3 + 实装订正）：E1（L0 塔重建，mod.rs:1101 逐值判据）/ E2+E2a（upper pop+extend
  合并逐值判据，mod.rs:1360）/ E3（cascade clear，mod.rs:1268）/ E4（`clear()`，mod.rs:617）。
- **命中判据双路**（设计 §5.4）：`Some(e)` epoch O(1)（生产）；`None` of_forest 指纹 fallback（合成/全量，
  无 O(n²) 暴露）。双 key 失效不变量：miss 重建同刷 `key`+`key_epoch`（interp.rs:tree_segment_cached_gen）。

## 3. 边界条件（结论翻转）

- 若 debug 逐 bar 对拍（`gen_fastpath_bit_exact_debug` / `bit_exact_per_bar_cached_vs_nocache`）某 bar
  panic ⟹ E1-E4 枚举不完备（存在未识别写入站点）⟹ 停下补枚举（no-workaround），不加兜底。
- 若 CL 实测 bump 率回升 ≫ 0.78% ⟹ 某 E-site 内容判据退化为长度判据/无条件 ++ ⟹ 复查逐值比对。
- 若 `false_hits > 0`（epoch 相等但 of_forest 指纹变）⟹ soundness 破裂 ⟹ 立即停。实测 false_hits=0。

## 4. 下游推论

- 生产 runner（step + merge 两调用点）K_i 森林段从 O(n²) 降为 O(n)（命中 98.5%，重建仅真变 bar）。
- candidate/gamma 段、merge 段的 O(n²) 残余**不在本工位有效域**——`profile_full_engine_scaling_16k`
  strategy exp 仍 1.85，那是 candidate 每 bar 全量拼接 + merge 全量快照（codex Q4 诚实声明的独立残余），
  非 forest_epoch 目标。本工位只消除 `of_forest` 每-bar 全塔指纹。

## 5. 谱系引用

- #160 A12（648 裁决 D）：K_i/T_i 双视图分离——本件是 K_i 侧 O(1) 命中判据落地。
- H6 NO-SHIP 结果包 + `on2w2-epoch-design-20260704.md`（codex GO）：设计权威。
- 订正条：设计 §3 长度判据 → 逐值内容判据（frontier resume 每-bar 复现相同字节的实证驱动）。
  interp.rs:641-644 的「generation 对 K_i 不 sound」注释语义按设计 §8 订正为「间接覆盖+blunt 兜底
  恰 sound 但过度 bump 无收益；forest_epoch 站点直接覆盖，紧且 sound」（本件已在 forest_epoch 字段文档落地）。

## 6. 影响声明

**改动文件（4，本件全部）**：
- `rust/src/theta_v0/classifier/mod.rs`：TowerCache 加 `forest_epoch` 字段 + `forest_epoch()` accessor；
  E1-E4 逐值判据 bump（E1 L0 塔 / E2+E2a upper pop+extend / E3 cascade / E4 clear）；新增 2 覆盖性测试
  （`forest_epoch_bumps_on_l0_tail_redivision` = A12 bar3020 场景 / `forest_epoch_stable_on_no_change`）；
  oracle_probe 加 forest_dirty 分解计数（cfg(test)）。
- `rust/src/theta_v0/backtest/incremental.rs`：`IncrementalClassifier::forest_epoch()` 透传；diag/test
  调用点加参；新增 `diag_forest_epoch_bump_vs_true_change`（ignored，bump 率 + 森林段 before/after 计时）。
- `rust/src/theta_v0/backtest/runner.rs`：`classify_at` 闭包返 4 元组（+forest_epoch）；`pi_theta_fill_loop`
  签名扩 4 元组；2 生产调用点 + 全部合成测试闭包透传。
- `rust/src/theta_v0/strategy/interp.rs`：TreeCache 加 `key_epoch` 字段；`tree_segment_cached_gen` 加
  `forest_epoch: Option<u64>` 参 + 双路命中判据 + 双 key 失效不变量；2 wrapper 加参透传；订正 A12 遗留
  stale guard（`bit_exact_per_bar_cached_vs_nocache` oracle T_i→K_i）；新增 `tree_cache_dual_key_..._no_stale`。

**不改**：`extract_carrier_forest`（森林构造）、`generation` 语义（candidate 段 + T_i 契约）、
`of_forest`（None fallback 保留）、任何输出计算路径。

---

## 验收数据（L2，CL OOS 2023-01-01..2025-06-30）

| 指标 | 值 | 判定 |
|------|-----|------|
| forest_epoch bump 率（8K） | 2.17%（174/8000） | vs of_forest 真变 0.78%，2.8× over-invalidate（E4 空-L0 clear，sound-safe） |
| false_hits（epoch 相等但森林真变） | **0** | soundness ✓ |
| E-site 分解 | E1=189 E2=0 E3=75（16667 calls） | E1（L0 逐值）主导，E2 归零（pop 复现相同窗口不 bump） |
| epoch 命中率（生产判据，16K） | 98.51% | vs 长度判据 0%（订正前） |
| of_forest 段计时（16K，隔离） | 0.33s → 0.054s | **6.1×**（extract_s，diag_gen_fastpath） |
| forest 段计时（8K，含 candidate 共同基底） | 0.052s → 0.020s | **2.60×** |
| debug 逐 bar 对拍（4K，含 frontier 重划） | 3860 命中 / 0 divergence | bit-exact ✓ |
| `cargo test --release --lib` | 1487 passed / 0 failed | 全绿（+3 新测试） |

**守卫清单落地**：G1（命中 debug_assert，改判据后确认仍执行，判据文本更新为「epoch 漏 bump」）；G2（miss 双视图一致性，不变）；G3-G5（GOLDEN/bit_exact 电池，全绿）；G6（CL 逐 bar 对拍复用为 diag，bump 率实测）；
G7（新增：L0 尾段重划 epoch bump 覆盖性 + Some/None 双 key 交替无陈旧 + 无变更不 bump）。
