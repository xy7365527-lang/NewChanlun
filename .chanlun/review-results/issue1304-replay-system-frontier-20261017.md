# #1304 回放验收系统 frontier scan（TB-02 工作窗口外并行线）

> 日期：2026-10-17  
> 状态：frontier 技术盘点，零代码改动  
> 范围：rust/src/trading/replay.rs、rust/src/bin/p123_fast_replay.rs、既有 replay 工具链

---

## 目标确认

按 TB-02 全图审计（2026-10-17），**#1304 不依赖 TB-02 完成**，可作为 TB-02 停滞期的并行推进线。本 scan 识别：

1. **已有回放能力现状**（D4 实现 vs p123 稀疏化）
2. **生产级验收要求缺口**
3. **确定性障碍与回避路径**

---

## 一、已有回放实现（两档）

### 1.1 D4 基础回放引擎（`rust/src/trading/replay.rs`）

**职责**：逐 bar 回放历史数据，完整记录每一重的状态演化。

**核心能力**：
- 单 bar 回放：`replay_bar(&mut self, bar: BarInput)`
- 批量回放：`replay_bars(&mut self, bars: Vec<BarInput>)`
- 状态快照：`StateSnapshot { timestamp_ms, output, layer_states }`
- 查询接口：
  - `get_snapshot(bar_index) -> Option<&StateSnapshot>`
  - `query_layer_state(bar_index, layer_id) -> Option<LayerState>`
  - `export_layer_timeline(layer_id) -> Vec<(i64, LayerState)>`

**限制**：
- **零验收机制**：无对拍、无确定性检查、无账目记录
- **接口松散**：依赖 `Box<dyn ClassificationEngine>`，未强制稳定序列化
- **无生产级日志**：缺失分类决策痕迹、无 Δ 计算、无完整性校验

**判定**：**研发原型级**，不能直接用于生产验收（缺关票门第 1/2/3 子句要求的证据）。

---

### 1.2 p123 快速回放（`rust/src/bin/p123_fast_replay.rs`）

**职责**：快速端到端重放（sparse dirty-driven），含完整账本与验收面。

**核心能力**（已在 M2/M3 双跑协议下验收）：
- **稀疏化架构**：per-(level, run) 评估缓存 + 四类 dirty 判据
- **完整账本**：`YieldBook`（candidates/divergences/terminal_confirmed/cert_a/cert_b）
- **验收面**：
  - stdout 门行全套（INPUT/RULE/PROBE/YIELD/CERT/D3/BASELINE/PROVIDER/SNAPSHOT/BIT_EXACT/R7/MISSED）
  - dump 侧信道（CERT/DIV/TERM/TURN/TURN_CLASS/FALLBACK，含行序）
  - M3 全量双跑协议（慢版对照 + CKPT 对账 + 判负即停线）
- **侧信道**（诊断 only，不进验收面）：
  - `P421_LIFECYCLE_DUMP`（活假设 feed/revision/异常审计）
  - `P123_EVENT_DUMP`（候选事件流逐条）
  - `P123_CHAIN_DUMP`（级别链证书 Delta 逐条）
  - `P757_OBSERVABILITY_DUMP`（批量丢弃观测面）

**覆盖范围**：
- **分类引擎**：theta_v0 classifier（M1 在册）
- **数据源**：`btc_1m_full.json`（250k bars 实测锚）
- **验收标准**：逐位对拍（白名单仅 `views=` 字段）

**限制**：
- **bin 形态**：非库接口，难以集成进持续验收
- **固定口径**：M1 theta_v0 专用，未抽象通用回放协议
- **账本隔离**：账本在 `YieldBook` 内部，无标准导出接口
- **无恢复机制**：中断即从头，缺 checkpoint 恢复能力

**判定**：**生产级验收工具**（已有 M2/M3 双跑背书），但**形态不适合作为持续验收框架**。

---

## 二、生产级验收要求缺口（按 TB-02 交付纪律）

### 2.1 关票门要求（`delivery-discipline.md` 全子句）

| 子句 | 要求 | D4 现状 | p123 现状 | 缺口 |
|-----|-----|--------|----------|------|
| ① 改动 | 文件清单 + 行数 | ✅ 模块头 | ✅ 模块头 | 无 |
| ② 验证 | 建源 + 测试 + 反例 | ❌ 零测试 | ✅ M3 双跑 | D4 缺 |
| ③ 签字 | 判据/算法/数学 | ❌ 零审计 | ✅ C5 构造性签字 | D4 缺 |
| ④ 追溯 | 引用标注 | ✅ 模块头 | ✅ 逐票 | 无 |
| ⑤ 架构 | ADR 同步 | ❌ 无 ADR | ❌ 无 ADR | **两档皆缺** |
| ⑥ 入仓 | 名分四态 | ✅ 常驻 | ✅ 常驻 | 无 |

**⟹ 关键缺口**：
1. **D4 缺完整验收**（子句 ②③）
2. **两档皆缺 ADR**（子句 ⑤）：回放引擎的架构决策未落 `docs/adr/`

---

### 2.2 完整历史回放（Destination ④）要求

**定义**（按 Destination ④）：逐 bar 回放历史数据 + 完整记录每一重状态演化 + **账目可查** + **双跑确定**。

| 维度 | 要求 | D4 现状 | p123 现状 | 实施路径 |
|-----|-----|--------|----------|---------|
| **逐 bar 回放** | ✅ | ✅ | ✅ | 已满足 |
| **状态快照** | ✅ | ✅ | ✅（dump 行） | 已满足 |
| **账目可查** | 分类决策痕迹 | ❌ | ✅（YieldBook） | **D4 缺** |
| **双跑确定** | 慢版对拍 | ❌ | ✅（M3 协议） | **D4 缺** |
| **恢复能力** | checkpoint 恢复 | ❌ | ⚠️（只写不判） | **两档皆弱** |

**⟹ D4 升级到生产级需补**：
1. 账本导出接口（`YieldBook` 风格）
2. 双跑对拍机制（M3 协议）
3. checkpoint 恢复（超出 M1 范围）

---

## 三、确定性障碍盘点（按既有审计）

### 3.1 已知确定性问题（来源：`.chanlun/review-results/issue1304-replay-feasibility-20260829.md`）

| 维度 | 问题 | 影响 | 绕行 |
|-----|-----|-----|-----|
| **赋格族 11 引擎** | 实验引擎未冻结 | 回放结果可能漂移 | **首批只带 M1 在册 V2r** |
| **变体注册表** | ~100 变体，V0/V2/V2r/V2of/VL/VR 全由少数基底派生 | 变体组合爆炸 | **M1 只验 enginefix 在册期望** |
| **tape v3** | trading 层第一批需 tape v3 | 数据依赖 | **已在 tape v3 实测** |

**判定**：**已有绕行路径**，M1 范围内确定性可控。

---

### 3.2 未覆盖风险（需补探针）

| 风险项 | 描述 | 探针需求 |
|-------|-----|---------|
| **L0 段账本完整性** | `seg_ledger_short` 计数（#559 C2） | 已有探针（p123 内置） |
| **活窗定位失败** | `l1_live_outcomes` 分布（#527/#601） | 已有探针（p123 内置） |
| **批量丢弃** | `p757_batch_drop_instances` 计数（#757） | 已有探针（p123 内置） |
| **L1 身份级 Live 缺失** | `p757_live_miss` 三类成因（#757） | 已有探针（p123 内置） |
| **分类器接口稳定性** | `ClassificationEngine` trait 变更 | **缺探针** |
| **序列化格式漂移** | `StateSnapshot` 序列化一致性 | **缺探针** |

**⟹ 需补探针**：
1. 分类器接口版本校验（trait 签名 hash）
2. StateSnapshot 序列化往返对拍

---

## 四、推荐实施路径（三阶段）

### Phase 0：D4 生产级升级（不依赖 TB-02）

**目标**：让 `replay.rs` 满足关票门全子句。

**任务清单**（3 个 task 票）：
1. **回放账本导出**（LOW）：
   - 为 `ReplayEngine` 添加 `export_book() -> YieldBook` 接口
   - 单测：replay 3 bars，验证 book 非空
   - 估算：0.5 天

2. **双跑对拍机制**（MED）：
   - 实现 `ReplayComparator`（对拍 `ReplayEngine` vs `p123` dump）
   - 白名单：`views=` 字段（同 p123）
   - 测试：100 bars 截段，diff=0
   - 估算：1.5 天

3. **回放 ADR 落盘**（LOW）：
   - 起草 `docs/adr/00XX-replay-engine-architecture.md`
   - 内容：D4 vs p123 形态选择、接口设计、验收标准
   - 估算：0.5 天

**总估算**：2.5 天（纯技术债，零决策）

---

### Phase 1：p123 框架化（依赖 Phase 0）

**目标**：将 p123 bin 形态抽象为可复用库。

**任务清单**（2 个 grilling 票 + 2 个 task 票）：
1. **grilling：账本接口设计**（1 轮）：
   - 问题：`YieldBook` vs `ReplayEngine::StateSnapshot` 如何统一？
   - 决策：统一接口 vs 双接口并存
   - 估算：1 会话

2. **task：账本抽象层**（MED）：
   - 实现 `trait ReplayBook`（统一 D4/p123 账本）
   - 为 `YieldBook` 实现该 trait
   - 估算：1 天

3. **grilling：侧信道可选性**（1 轮）：
   - 问题：P421/P123/P757 侧信道是否应可插拔？
   - 决策：编译期可选 vs 运行时可选
   - 估算：1 会话

4. **task：侧信道插件化**（HIGH）：
   - 设计 `trait ReplaySidecar`
   - 迁移 P421/P123/P757 dump
   - 估算：2 天

**总估算**：3 天 + 2 轮 grilling（约 4 天）

---

### Phase 2：持续验收集成（依赖 Phase 0+1）

**目标**：将回放验收集成进 CI。

**任务清单**（1 个 grilling 票 + 2 个 task 票）：
1. **grilling：CI 回放策略**（1 轮）：
   - 问题：全量 vs 截段 vs 采样？
   - 决策：PR CI 跑 10k bars，nightly 跑全量
   - 估算：1 会话

2. **task：CI 回放脚本**（MED）：
   - 脚本：`scripts/replay_ci.sh`
   - 输入：btc_1m_full.json（10k 截段）
   - 输出：对拍报告 + 失败即红
   - 估算：1 天

3. **task：nightly 全量回放**（LOW）：
   - GitHub Actions workflow
   - cron：每日凌晨 2:00
   - 产物：全量 dump 归档到 Releases
   - 估算：0.5 天

**总估算**：1.5 天 + 1 轮 grilling（约 2.5 天）

---

## 五、与 TB-02 关系明细

### 5.1 不依赖 TB-02 的部分（可立即并行）

- ✅ Phase 0 全部（D4 升级）
- ✅ Phase 1 账本接口设计
- ✅ Phase 2 CI 策略 grilling

**理由**：这些任务只涉及 `replay.rs` 与 p123，不触及 TB-02 的六大 Destination。

---

### 5.2 弱依赖 TB-02 的部分（可提前准备）

- ⚠️ Phase 1 侧信道插件化：**P421/P123/P757 是 p123 内部机制**，插件化设计不依赖 TB-02，但**验收需要 TB-02 完成后的稳定基线**。

**建议**：Phase 1 可提前设计，但**验收推迟到 TB-02 完成**。

---

### 5.3 强依赖 TB-02 的部分（必须等 TB-02）

- ❌ **生产级 checkpoint 恢复**：需要 Destination ④ 的"成交持仓来源唯一"完成后才能定义 checkpoint 内容。
- ❌ **#1304 接通验收**：按 Destination ⑥"接通 #1304，完整验收前不关闭 #1279"，**#1279 是 TB-02 阻塞器之一**。

**判定**：**checkpoint 恢复超出 #1304 当前范围**（属 M2 或更晚），Phase 0-2 可不含此功能。

---

## 六、推荐优先级（按 TB-02 停滞窗口最大化并行）

### 立即可启动（Week 1-2）

1. **Phase 0 全部**（2.5 天）：
   - 回放账本导出
   - 双跑对拍机制
   - 回放 ADR 落盘

2. **Phase 1 账本接口设计**（1 会话）

**理由**：零决策风险，纯技术债，可立即派 AFK 工蜂。

---

### 等 TB-02 关键票解锁后（Week 3-4）

3. **Phase 1 侧信道插件化**（3 天）：
   - 等 #1419（source_audit 失败）或 #1488（B2 v4 CI）之一解锁
   - 用稳定基线验收插件化

4. **Phase 2 全部**（2.5 天）：
   - CI 回放脚本
   - nightly 全量回放

---

### 暂缓到 TB-02 完成后

5. **checkpoint 恢复**（未估算）：
   - 等 Destination ⑥ 完成
   - 属 M2 或更晚

---

## 七、风险与代价

### 7.1 已知风险

| 风险项 | 概率 | 影响 | 缓解 |
|-------|-----|-----|-----|
| **D4/p123 账本不兼容** | MED | HIGH | Phase 1 grilling 提前暴露 |
| **p123 侧信道过度耦合** | HIGH | MED | Phase 1 插件化设计 |
| **CI 回放超时** | LOW | LOW | 10k 截段 + nightly 全量 |
| **TB-02 阻塞延长** | MED | HIGH | Phase 0-1 可独立交付 |

---

### 7.2 代价（照实）

- **Phase 0-2 不含 checkpoint 恢复**：中断即从头，适合 CI/nightly 但不适合交互式调试。
- **p123 框架化成本高**：~4 天（含 grilling），而 M1 只需 p123 bin 形态已够。
- **ADR 债务暴露**：回放 ADR 是第一个"架构决策未落 ADR"的显式债务记录。

---

## 八、结论与建议

### 8.1 #1304 可立即并行推进

**已验证**：
- ✅ D4 已有基础回放能力
- ✅ p123 已有生产级验收背书（M2/M3）
- ✅ Phase 0 全部任务不依赖 TB-02
- ✅ 确定性障碍已有绕行路径（M1 范围内可控）

**推荐首批**：
1. Phase 0（2.5 天）：D4 生产级升级
2. Phase 1 账本接口设计（1 会话）

**总工期**：约 3.5 天（可派 2 个 AFK 工蜂并行）

---

### 8.2 与 TB-02 协同策略

- **Week 1-2**：Phase 0 全部（不等 TB-02）
- **Week 3-4**：Phase 1-2（部分等 TB-02 解锁，部分可提前）
- **M2 或更晚**：checkpoint 恢复（强依赖 TB-02 完成）

**关键依赖**：
- Phase 0-1 账本设计：**零依赖 TB-02**
- Phase 1 侧信道插件化：**弱依赖 TB-02**（需稳定基线）
- Phase 2 CI 集成：**弱依赖 TB-02**（策略可提前，验收需基线）
- checkpoint 恢复：**强依赖 TB-02**（M2）

---

### 8.3 下一步动作（立即可执行）

1. **开 #1304 子票**：
   - 子票 A：回放账本导出（task, LOW, AFK 工蜂）
   - 子票 B：双跑对拍机制（task, MED, AFK 工蜂）
   - 子票 C：回放 ADR 落盘（task, LOW, RLM 子代理）

2. **更新 #1304 票面**：
   - 在 #1304 评论添加本 scan 指针
   - 更新 blocking 边：子票 A/B/C → #1304

3. **派发执行**：
   - 子票 A/B 派 sandcastle 工蜂
   - 子票 C 派 RLM 子代理（只读文书）

**预期交付**（Week 1-2）：
- 3 个子票全部关闭
- D4 满足关票门全子句
- #1304 从 TB-02 阻塞集中解除（弱依赖变为可并行）

---

## 附录：关键文件清单

### A.1 回放引擎

- `rust/src/trading/replay.rs`（157 行，D4 基础实现）
- `rust/src/bin/p123_fast_replay.rs`（4,925 行，生产级验收 bin）

### A.2 已有审计

- `.chanlun/review-results/issue1304-replay-feasibility-20260829.md`（2026-08-29）
- `.chanlun/review-results/wayfinder-full-runner-path-2026-09-01.md`（2026-09-01）

### A.3 相关票

- #1304：本票（回测重定位）
- #1279：TB-02 阻塞器之一
- #378/#372/#948/#1294：历史回放相关票

---

**frontier scan 完成**。本报告未改动任何代码，已满足 #1304 步骤 1 要求。
