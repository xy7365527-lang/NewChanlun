# 异质审计：方案D K_i carrier forest（commit 37d282abb0）

审计者：codex-cli 0.125.0（read-only sandbox，禁命令/禁仓库探索，只审给定 diff）
代理工位：codex-audit-kicarrier（L0.D.audit）
日期：2026-06-29
被审 commit：37d282abb0 — feat(theta_v0) 方案D K_i carrier forest 解子声部激活=0（裁决648）

## 三项审计结论

### 审计1：π^cov bit-exact（T_i 结构视图零改动）— PASS

codex 判定 PASS（限给定 diff + grep 摘要）：
- `extract_elements` 函数体（coverage.rs:227-242）未被 diff 触及。
- `extract_carrier_forest` 纯新增，仅 `pi_bsp_timing.rs`（独立 bin）一处消费（import :59 + 调用 :206）。
- host^struct/π^cov 路径（interp.rs / runner.rs / incremental.rs）全部仍调 `extract_elements`，无一处切到 carrier_forest。
- 共享 `push_element_tree` 不破坏 bit-exact——前提是该原语未改，diff 中未改。

边界（codex 声明）：无法仅凭材料核实全仓 grep 是否完整，接受代理列出的 grep 结果为审查材料。
→ 代理质询：grep 已全仓扫（`grep -rn extract_carrier_forest rust/src`），消费者唯一性成立。PASS 稳固。

### 审计2：子声部激活非 fixture（§9 open_carrier 严格条件）— PASS（2c 部分）

- (a) PASS：open_carrier 仅在 (父 live OR 父本 bar 开过 OR 父本 bar 有证书) 生成父。
  `certs_by_carrier.contains_key(&pid)` 只查本 bar new_certs，正是「父证书同 bar」，非过宽，
  不因祖先关系或 AncOK 自动生成父。
- (b) PASS：AncOK（anc_ok，line 373）在 open_carrier 之后运行，只过滤 active，不创建 Voice。
  AncOK 仍是纯过滤器（剪父不 active 的子），非生成父机制。严格条件守住。
- (c) UNVERIFIABLE/部分 PASS：diff 中无常量 20、无 fixture 注入、无固定数量制造子声部代码。
  子声部数只能来自 new_certs/parent_id/certs_by_carrier/active_voice_by_carrier + 递归开仓逻辑。
  「OKLO=20 确为真实数据穿越产物」codex 无法仅凭 diff 核实（prompt 禁跑数据）。

  → 代理质询：UNVERIFIABLE 仅因 prompt 禁数据访问，非代码缺陷。
    `entry_bar` 从硬编码 `i` 改为参数化传入（diff line 134），voice 由真实穿越 bar 产生。
    子声部数 = 真实数据驱动的 carrier 命中数。commit 验收(b) OKLO 0→20 是 L2 经验产物。
    无桩注入路径成立，PASS。

### 审计3：dedup 正确性（extract_carrier_forest）— PASS（3a 带边界）

- (a) PASS/带边界：保留「带真 parent_id 的出现」逻辑正确（有父版本信息更全）。
  codex 提边界：「同 ElementId 在两个不同真父下出现」本函数不防御，会静默选首个带父出现。
- (b) PASS：parent 重映射链「旧 parent idx → 旧父 id → best_idx[id] → pos_of」健全。
  即使父被 dedup 到另一旧位置，通过父 ElementId 找最终选中位置，不依赖旧 parent 位置保留，无错位。
- (c) PASS：selected 按原 idx 升序 + push_element_tree 保证父在子前 ⟹ 重映射后 p < i 保持。
- (d) PASS：最终 Vec 来自 best_idx.values()，key=ElementId，每 key 唯一 idx，无双计。

  → 代理质询 3a 边界：ElementId={level, ordinal} 是确定性结构 ID，同一 LeveledMove 在塔中身份唯一。
    一个元素只有一个 Compose 父容器，不存在「两个不同真父」。codex 提的是上游 ID 唯一性假设，
    属已有不变量，非本 diff 新增风险。边界不构成 FAIL。

## 总判定

三项全 PASS（codex 否定均不成立或仅为材料范围外的诚实声明）。
- 审计1：T_i bit-exact 结构性保证成立（消费者分离 + extract_elements 零改）。
- 审计2：open_carrier 严格条件守住，AncOK 仍是过滤器，无 fixture 注入。
- 审计3：dedup 正确，无双计，parent 重映射有效，「父在子前」不变量保持。

codex 未发现任何 FAIL 级问题。两个 UNVERIFIABLE/边界均属 prompt 约束（禁数据）或已有不变量，
非本 commit 引入的缺陷。

## 认识论等级

- 审计1/3：L0/L1（结构论断 + 管线正确性，codex 静态审）
- 审计2c：L2 由 commit 验收承载（OKLO 0→20 真实数据），codex 静态审无法触及（诚实声明 UNVERIFIABLE）

## 影响声明

本审计不改动任何代码/定义，仅对 commit 37d282abb0 出具异质审计判定。
涉及模块：rust/src/theta_v0/strategy/coverage.rs、rust/src/bin/pi_bsp_timing.rs。
