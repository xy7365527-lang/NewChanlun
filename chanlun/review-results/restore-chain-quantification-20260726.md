# wayfinder #301 — 恢复链≥2 级生产窗口量化：restore_calls 频率 + p̃/角色差值

- 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`，对拍时点 HEAD=5395a60911）。
- 票：#301（#247 挂账量化缺口；编排者 2026-07-26 定窗定通道）。
- 通道：票面通道①——`#[cfg(test)]` 临时 ignored 探针 `r301_restore_chain_window_probe` 直调 `run_theta_v0_pi_overlay` 生产函数（#267 同款先例）；**未走** p123_fast_replay / backtest_bin。探针用毕已删净（coverage.rs / runner.rs 复原 HEAD，git 面零残留）。
- 对拍（三臂，A vs C 按票面、B 拆分 #247/#267）：
  - A = `b57da4bde4^`（修复前）。A/B 臂无 #247 引入的 `restored: Vec<usize>`，探针埋点以**行为中性纯记录**同名向量嫁接（graft.py，无 parent/attached_dir 修补逻辑——本报告显式声明）。
  - B = `b57da4bde4`（仅 #247）；C = HEAD（#247 + #267）。
- 窗口：OKLO 全窗（343282 bar）+ BTC 前 50 万笔切片（476284 步）。
- 工作 lineage（090 照实）：Kimi agent（中断，/tmp/r301/ 存 after 臂日志）→ claude CLI Opus（600s 上限被杀，/tmp/r301-work/ 存三臂构造件与 B/C 日志）→ 主控 session 核验续跑 A 臂并成稿。三批产物交叉一致。

## 1. 频率：目标场景发生次数（两窗口）

| 窗口 | restore_calls | complete | already_in_raw | registry_lost | chain_ge2 | chain_ge2 且中间 parent≠None | restored_pushes | resolvable |
|---|---|---|---|---|---|---|---|---|
| OKLO 343282 | 263 | 198 | 65 | 0 | **0** | **0** | 0 | 0 |
| BTC 前50万 | 2021 | 365 | 1656 | 0 | **0**（链长1仅 1 例） | **0** | 1 | 0（未解析⟹不伪造，AncOK 剪除） |

**#247 的行为改变场景（恢复链≥2 级且中间节点 parent_id≠None）在两窗口实际发生 0 次。** BTC 唯一 1 次恢复 push 父不可解析，按不伪造语义剪除（boundary_germ=1、germ_patched=0，∂ 保持）。

## 2. 对拍（A vs B = #247 影响；B vs C = #267 影响）

| 窗口 | 量 | A（修复前） | B（#247） | C（HEAD） | A→B Δ | B→C Δ |
|---|---|---|---|---|---|---|
| OKLO | sum_p̃ | 1297926894.188826 | 同左 | 1297392476.805325 | **0** | −534417.383 |
| OKLO | stream_hash | 0x1ee71cdd9d4b1e41 | 同左 | 0xcb700f459111ceb9 | **逐位同** | 变 |
| OKLO | n_orders | 114109 | 同左 | 114978 | 0 | +869 |
| OKLO | 账户 PnL | −785127.105 | 同左 | −783983.105 | **0** | **+1144.000（减亏）** |
| BTC | sum_p̃ | 10690557.716214 | 同左 | 10687156.162596 | **0** | −3401.554 |
| BTC | stream_hash | 0x0c5e7bc41ab583a2 | 同左 | 0xa6257359605f681d | **逐位同** | 变 |
| BTC | n_orders | 30455 | 同左 | 30455 | 0 | 0 |
| BTC | 账户 PnL | 841644.580 | 同左 | 841222.270 | **0** | **−422.310（转差）** |

（reconcile 残差各臂均在 1e-8~1e-9 PASS 量级，run 间微差照实登记，非身份字段。）

## 3. 结论（L1 结构证据，不声明 alpha）

1. **#247 生产影响 = bit-exact 零**：目标场景两窗口零发生，A→B stream_hash 逐位相同、全部指标零差。#247 是把「理论可达但本数据未达」的防御分支输入修对——正确性修复，窗口内无行为差；**不代表更大窗口/其他品种恒零**（BTC 全窗 460 万笔未跑，本切片 50 万笔；若需更宽覆盖走通道三选一另票）。
2. **#267 影响（B→C）**：OKLO +1144.000（−785127.105→−783983.105，减亏 0.146%，与 #267 报告逐位一致，独立复现）；**BTC 切片 −422.310（841644.580→841222.270，−0.05%，本次新测，照实报转差）**；两窗口触发面/∂/声部身份零变化（germ_patched=0、voices 865/1527 不变）。
3. **签字包口径建议**：#247「实装后复核」栏可写「生产窗口差值 bit-exact 零（场景零发生，两窗口三臂对拍在案）」；#248 签字包关注的成交/净值量化归 #307，与本票不重叠。

## 4. 偏离与未做

- BTC 为前 50 万笔切片非全窗（票面允许；OKLO 频率非零（263 次 restore_calls）但目标场景为零，BTC 切片同结论，未扩窗）。
- A/B 臂探针埋点为行为中性嫁接（ graft 无修补逻辑），已在 §0 声明；graft.py 与三臂构造件留 /tmp/r301-work/（不入仓）。
- OKLO/BTC 之外品种未测。
- 探针与注册行已删净，coverage.rs/runner.rs 经 `git show HEAD:` 复原 + cargo test 1834/1 复验。
