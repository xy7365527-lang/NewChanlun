# p92 nest_replay_postruling 全量回归（Trend C forward-find 修复后）

日期：2026-07-17　数据：btc_1m_full.json（4,613,599 bar）　运行：prefix 遍 ~2200s + replay 遍，views=84,613。

## 门检（全部 PASS）

- **P92_BIT_EXACT**：old_path / tower / moves_centers_bsp_pan / lifecycle_cp_ownership / classification_total 全部 diff=0 —— 新路径对旧路径位级一致。
- **P92_R7**：provider_complete=true，definition_faithful=true，snapshot_no_forward=true，B_zero=false。
- **P92_PROVIDER**：snapshots=3,684，views=84,613，too_short / invalid_seed / missing_carried_center / other / unresolved_targets 全 0，complete=true。
- **P92_SNAPSHOT**：future_violations=0，created_at_reads=0，no_forward=true。
- **P92_MISSED**：terminal_confirmed=24，covered_b=25，intake_fallback_events=2，missed=0。
- **P92_D3**：edges=1，violations=0（可测边仍仅 1 条，D3 可测性恢复归 #103）。

## 产出与修复效果

- **P92_YIELD**：candidates=3,683（trend 521 / pan 3,162），divergence_confirmed=1,235（**trend 429** / pan 806），terminal_confirmed=24。
  - 对照 #104 修复前 trend 侧背驰确认仅 1/807：forward-find 修复后 trend 侧确认恢复至 429 —— 修复在全量数据上生效。
- **P92_CERT**：A=41（trend 3 / pan 29 / mixed 9），B=25（trend 3 / pan 22 / mixed 0）。B_zero=false 维持。
- **P92_BASELINE**：旧路径 483/483/483 vs 新路径 candidates=3,683、terminal_confirmed=24、B_certs=25 —— 新路径候选面扩大 ~7.6×，终段确认按新裁定收紧，符合预期语义（候选≠确认）。

## 结论

Trend C 终段 forward-find 修复后的全量回归通过：位级一致、无前视、无缺失、provider 完备。trend 侧背驰确认从 1 → 429，#104/#105 修复闭环。遗留：D3 可测边仍为 1（#103），B 口径 Long 侧偏少（#102）。
