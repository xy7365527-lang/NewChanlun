# A3 阶段2实施审计（codex-challenger, ws-a3iaudit, task #39）

审查对象：工作树 WIP（未 commit）——`rust/src/theta_v0/classifier/mod.rs`（954-1290 区域）、
`rust/src/theta_v0/classifier/recursive_tower.rs`（compose_level_resume 396-437）、
`rust/src/theta_v0/backtest/incremental.rs`（新增 always-run oracle 两 fixture + A3 profile 测试）。

证据来源：本机 codex CLI `codex exec --skip-git-repo-check --sandbox read-only -c
'model_reasoning_effort="high"'`（read-only sandbox，codex 自主用 `rg`/`nl -ba` 核对实际代码文件，
非仅读 diff 摘录）+ 本工位对代码库的直接 diff 提取核对 + 本工位独立重跑测试/profile（非转述
ws-a3impl 报告数字）。

**总裁决：通过。焦点1（truncate 保留 vs 删除，编排者授权 codex 全权裁定）= 保留；焦点2（实施
↔v2规格一致性）= confirmed（1条非阻塞注记）；焦点3（05=H-clone 定性充分性）= confirmed，方向
成立，改写泳道 A 优先级。**

---

## 独立验证（本工位实测，非转述）

在调用 codex 前先本机重跑关键证据，确保喂给 codex 的数据是一手的：

1. 测试全绿：`cargo test --lib a3_oracle`（2 passed + 1 ignored 需CL）、`cargo test --lib
   bit_exact`（54 passed）、`cargo test --lib theta_v0::classifier::`（232 passed）、
   `cascade_reset_on_frontier_interior_rewrite`（ok）。
2. Profile 重跑（`--release`，CL 真实数据）：
   - 200K bar：`05c_tail_upper_build`=2377.4ms/`05_compose_resume`=2521.3ms（94.3%）；
     `05_span` avg=4.64；`03`=14.18ms、`04`=14.42ms。
   - 1M bar：`05c_tail_upper_build`=28792.9ms/`05_compose_resume`=29726.5ms（**96.86%**，与
     ws-a3impl 报告的 96.8% 吻合）；`05_span` avg=**5.34**（与报告吻合）；`03`=90.35ms（较 A0
     基线 2446.0ms 降 96.3%）、`04`=88.55ms（较 A0 基线 4010.9ms 降 97.8%）。
   - 关键观察：`05_span` avg 在 200K→1M（5倍规模）仅 4.64→5.34（+15%），非线性增长——不支持
     H-detect（O(n²)续扫应随窗口规模近似线性增长到远大于 5 的量级）。

## 焦点1：truncate 保留 vs 删除（codex 全权裁定）

**裁决：保留。**

推导链（codex 原话）：两处 truncate 位置代码核对正确——`min_parts` 早停发生在 `LevelCache`
push 前，截到 `level_idx`；`units.is_empty` 早停发生在本级处理完后，截到 `level_idx + 1`
（mod.rs:1065/1278 一带）。removal 子例（截断实际删掉已建高层）目前只有经验不可达证据（600K
真实CL + 9K×2 synthetic，reentry_minparts=reentry_empty=0），**不是类型/代数层面的证明**。
删除后的风险是 `dirty_from` 证书复用陈旧高层缓存 → `frontier_mutated` 假阴 → cascade 该触发
未触发 → 下游静默复用陈旧数据（非 panic，可能很久才被发现）。保留成本 = O(l_max)（当前配置
常数级）且仅早停路径执行（非热路径）。**成本-风险不对称本身决定裁决方向：保留。**

## 焦点2：实施↔v2规格七项一致性

**裁决：confirmed，1条非阻塞注记。**

逐项代码核对（codex 独立核对 + 本工位 diff 核对交叉一致）：
- dirty_from 公式：L0 来自 `l0_dirty_from`（reuse），L1+ 来自上一层 pop 后/extend 前的
  `prefix_count`，驱动 `stable = dirty_from.min(scanned)` —— 与 §2.2/§2.3/§3.3 逐字段一致。
- 04 三重 min：`keep = dirty_from.min(cached_units.len()).min(units.len())` —— 与 §3.2 一致。
- 09 投影加固：`projected_units.truncate(prefix_count)` 后 resume，配 debug_assert 全量投影
  护栏 —— 与 §2.5 一致。
- oracle：两条非阻塞精修建议均已采纳——两个独立 fixture 未合并（`a3_oracle_minparts_reentry`
  / `a3_oracle_pop_rescan_empty`）；`T = tail_upper.len()` 在探针与调用点均有精确定义注释，
  非笼统字面量 —— 与 §5 item3 精修建议一致。
- **超规格新增**（`oracle_probe` 模块）：判定为合规实现细节，非规格偏离——`#[cfg(test)]`
  release/非test零编译，且正好补上"always-run但覆盖为零"的可观测性缺口，是复审精修建议的
  自然延伸而非另立机制。
- **非阻塞注记**：05 子计时在 `THETA_PROFILE_STAGES` 未启用时仍有 `stage_profile::enabled()`
  分支判断，不是字面零指令开销（有一次 thread_local 读 + 分支预测）。这不构成行为偏离，也不是
  当前 blocker——与既有 `stage_profile::time` 机制（mod.rs 主循环 03/04/09 等全部 stage 计时）
  同一开销模型，A3 未引入新的开销范式。

## 焦点3：05=H-clone 定性证据充分性

**裁决：confirmed，方向成立，改写泳道 A 优先级。**

理由（codex 原话）：1M 下 `05a_detect_windowed`≈198ms，而 `05c_tail_upper_build`≈28.8s，占
05 的 96.86%；同时 `05_span` avg 从 200K 的 4.64 到 1M 的 5.34 仅涨 15%，不支持续扫跨度随
窗口规模线性增长（H-detect 应表现为跨度随 n 明显增长）。插桩位置正确分离了 detect/center
collect/tail build 三阶段（recursive_tower.rs:413 一带）。

**结论**：这组数据构成 L2+ 判定（真实数据、可能否证假设、非合成确认偏差）——当前 05 的
60%+ 耗时属于 H-clone（tail_upper build 内 clone/compose 分配可压），非 H-detect（O(n²)
续扫，A4 窗口封存域）。**泳道 A 优先级应改写**：A4（窗口封存/resume 锚前移）不是当前最大靶；
`05c` 内 `subs_moves[win[*]].clone()` / `LeveledMove::compose` 分配复用是下一最大靶（task #40
方向正确，已 pending）。

**codex 对 #40 实施方向的补充建议**（非阻塞，供 #40 工位参考）：不要先上重抽象，先对 05c
内部再切一刀（clone vs compose vs allocation 分项计时），哪个大就动哪个。`span max=8375`
值得记录留证，但 05a（detect）总耗时占比很小的前提下不构成阻塞缺口。

---

## 结果包（result-package.md 六要素）

1. **结论**：A3 阶段2实施三焦点全部通过。焦点1（编排者授权全权裁定）codex 裁决保留两处
   truncate（成本-风险不对称：保留成本 O(常数)+仅早停路径，删除风险=静默数据错误）。焦点2
   实施与 v2 规格七项逐字段一致，1条非阻塞开销模型注记。焦点3 codex 独立确认 05c 是 H-clone
   （非 H-detect），泳道 A 优先级改写生效，A4 非当前最大靶，task #40（05c compose 优化）为
   新最大靶，方向正确。
2. **定义依据**：不可变前缀 = anc.pdf §16；A3 v2 设计稿 §2.2/§2.3/§2.5/§2.6/§3.2/§3.3/§5
   （`.chanlun/review-results/a3-design-20260702.md`）；v2 复审七项裁决
   （`.chanlun/review-results/a3-v2-reaudit-20260702.md`）；231号有效域（L0-L3 认识论等级，本
   审计 profile 数据为 L2+：真实 CL 数据、可能否证 H-clone/H-detect 二选一假设）。
3. **边界条件**：焦点1 裁决在"中枢计数单调非降"仍为经验观测（非形式证明）的前提下成立——若
   未来出现该单调性失效的真实数据（removal 路径实际触达），truncate 的正确性论证（re-audit
   LevelCache::default 重建等价论证）仍应继续覆盖，不需改动；若单调性被证明为代数不变量，
   truncate 可能降级为可选（但保留仍零成本，无需删除动机）。焦点3 裁决在两档 profile 规模
   （200K/1M）范围内成立——若未来在更大规模（如10M+）观测到 span 显著增长趋势，需重新评估
   是否转为 H-detect 混合态。
4. **下游推论**：Lead 可合树 commit A3 批（mod.rs/recursive_tower.rs/incremental.rs 三文件
   WIP），解锁 #23（07b extract_second frontier 门控）/ #40（05c compose alloc 优化）。#40
   工位落地时应先对 05c 内部再拆分项计时（clone/compose/alloc），而非直接假设某一项主导。
5. **谱系引用**：`.chanlun/review-results/a3-design-20260702.md`（v2设计稿）、
   `.chanlun/review-results/a3-v2-reaudit-20260702.md`（v2复审七项裁决）、
   `.chanlun/review-results/codex-a3-design-ruling-20260702.md`（v1裁决，refuted 第4/6条
   在 v2 §2.6/§5 根修）；231号形式化有效域（L0-L3 认识论等级标注）；090号严格性语法规则
   （成本-风险不对称裁决依据）。
6. **影响声明**：本审计零代码改动、零 git 操作。新增本文件 + 本机 codex CLI 一次调用
   （101,304 tokens）+ 本工位独立重跑测试（`cargo test --lib`，多组）与 profile
   （`--release`，200K + 1M CL bar，累计约70秒机器时间）。未修改任何生产代码/测试代码，
   仅验证工作树既有 WIP。
