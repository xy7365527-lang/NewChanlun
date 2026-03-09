# 402号：数据流路径审查（v204-swarm/dataflow-audit）

status: 已结算
created: 2026-03-09
swarm: v204-swarm/dataflow-audit

## 审查范围

对系统五条数据流路径（路径1-5）进行代码级审查，验证用户反馈的准确性，补充缺失路径6-8的实现状态。

## 逐路径审查发现

### 路径1 & 路径2：K_active 加载来源（确认无问题）

**用户判断：无问题。**

代码确认：`daemon.py:251` — `self.k_active = graph`，启动时从 `k_merged.json`（23个 graph_*.json 合并）加载。本地包含实验图导致 6678V > VPS 5928V。

### 路径3：k_full.jsonl 写入内容（确认存在问题）

**用户判断：需要注意只记录结构性产出。**

代码审查发现（`daemon.py:854-883`）：

k_full.jsonl（通过 `PersistentKFull`）的写入逻辑：
1. **每一步**（不仅仅是结构性事件）都会 diff K_full 的 vertices/edges，将新增写入 JSONL（`daemon.py:854-863`）
2. 显著事件额外写入 operation 记录（`daemon.py:876-883`）
3. feed 注入时（`daemon.py:1148-1160`），phi_L 产出的所有顶点/边都写入

**问题确认**：`PersistentKFull` 记录的是 K_full 图的所有增量变更（vertex + edge），这本身是正确的——K_full 按设计就是"全量图"。但关键区别在于：

- **K_full JSONL**（`persistence.py`）：记录图结构增量，只在 sublate 产生新顶点、negate 产生新边、fold 产生合并时才有新增——这**是**结构性事件，无问题。
- **encounter log JSONL**（`encounter_log.py`）：记录穿越轨迹（概念A遇到概念B），这是 visit_history 性质。
- **401号修复**已生效（`encounter_log.py:354-357`）：`rebuild_memory_from_jsonl` 不再从 encounter log 重建 encounter memory 节点。注释明确说"脚印不是宝藏"。

**结论**：路径3的 k_full.jsonl 写入逻辑是正确的——它只在图拓扑实际变化时才写入。之前 memory 节点暴增（30863个）的问题根源已被 401号修复（不再从 encounter log 注入 encounter memory 节点到 K_active）。但 `_inject_cross_domain_incremental`（`daemon.py:845`）在每次 settlement 后都会扫描并注入跨域边，这可能是次要的膨胀来源。

### 路径4：S_net bootstrap 与 articulation feedback（确认不完整）

**用户判断：基本对但不完整。**

代码审查发现：

**已实现的机制**：
1. **S_net bootstrap**：从辞典文件加载（`signifier_net.py` 构造），匹配率取决于辞典术语是否在 K_active 中存在——与 5% 匹配率问题一致
2. **对话回写**：`_writeback_to_snet`（`daemon_api.py:1066-1120`）实现了输入回写；`externalize` 函数（`internal_speech.py:395-405`）实现了输出回写
3. **articulation feedback**：`traversal.py:1050-1071` 实现了 S_net → K_active 的反向路径（`articulation_feedback` 和 `articulation_bridge`）
4. **耦合振荡中 S_net 激活态维护**：`_resonate_from_operator`（`daemon_api.py:1302-1306`）实现了 operator 输入对 S_net 激活态的叠加

**结论**：路径4描述中"缺少的机制"实际已经实现。用户反馈基于较早的代码状态，当前代码已补全。

### 路径5：/feed API 的 NLP 提取逻辑（确认是问题路径）

**用户判断：路径5是最需要警惕的。**

代码审查发现（`daemon_api.py:1350-1372`）：

`present_json`（处理非 dialogue 类型输入时）执行：
1. `daemon.feed(text)`（`daemon_api.py:1353`）→ 调用 `phi_L(text)`（`daemon.py:978`）→ NLP 解析 → 白名单匹配 → 提取概念顶点 → 合入 K_active
2. `_extract_concepts(text)`（`daemon_api.py:1360`）→ 纯字符串分词 → 在 K_active 中查找匹配
3. `_link_concepts_to_source`（`daemon_api.py:1372`）→ 将匹配到的概念连接到 session 源顶点

**关键发现**：
- `phi_L` 已经做了**白名单限制**（`phi_L.py:6-7, 113-135`）：只有 `CHANLUN_WHITELIST` 中的术语才会产生顶点。这不是无约束的 NLP 提取。
- 但 `_extract_concepts`（`daemon_api.py:551-559`）是**无白名单的纯分词**，只做 `len(t) >= 2` 过滤。它本身不注入顶点，只用于在 K_active 中查找已存在的匹配。
- `_link_concepts_to_source`（`daemon_api.py:597-613`）在匹配到的概念和 session 源顶点之间建立 REFERENCE 边——这些边会被写入 K_active 和 K_full。

**问题**：虽然 `phi_L` 有白名单保护，但 `daemon.feed(text)` 仍然是"文本 → NLP → 提取概念 → 合入 K_active"的模式。这与用户确立的方向矛盾——**文本应直接进 S_net，概念关系从 articulation feedback 涌现，不从 NLP 预提取注入**。

dialogue 路径已经正确实现了这个方向（`daemon_api.py:1299-1336`）：
- 输入 → `_writeback_to_snet`（S_net 共现边回写）
- 输入 → `_externalize_speech`（内部言语外化）
- **不调用** `daemon.feed(text)`

但非 dialogue 路径（`daemon_api.py:1350-1356`）仍然走旧的 feed 模式。

### 缺失路径审查

**路径6：encounter_log / settlement_history 作为 block topology 中的节点**

已实现：
- `encounter_log.py:181-218`：`inject_encounter_memory_node` 将 encounter 事件注入为 `domain:memory` 节点
- `encounter_log.py:221-307`：`inject_settlement_memory_node` 将 settlement 事件注入为 `domain:memory` 节点
- `traversal_checkpoint.py:75-143`：`SettlementHistoryWriter` 维护 append-only 的 settlement 历史 JSONL
- 401号修复后：encounter memory 节点不再注入（"脚印不是宝藏"），只有 settlement memory 节点注入

**路径7：S_net → K_active 的 articulation feedback**

已实现：`traversal.py:1001-1075`，包括两种模式：
- **articulation_bridge**（`traversal.py:1013-1048`）：S_net 中的孤儿能指 → 在 K_active 中创建桥接顶点
- **articulation_feedback**（`traversal.py:1050-1071`）：S_net 建议的概念关系 → 在 K_active 中添加新边

**路径8：对话回写**

已实现：
- 输入回写：`_writeback_to_snet`（`daemon_api.py:1066-1120`），在 dialogue 路径中调用（`daemon_api.py:1303`）
- 输出回写：`externalize` 中的步骤5（`internal_speech.py:395-405`），LLM 输出 → S_net 共现边

## 修正建议

### 高优先级

1. **路径5 非 dialogue 路径的 feed 调用**：`daemon_api.py:1350-1356` 中的 `daemon.feed(text)` 应当移除或替换为 S_net 回写路径。当前 dialogue/非 dialogue 两条路径的数据流不一致——dialogue 路径正确地走 S_net，非 dialogue 路径仍走旧的 phi_L 注入。

### 低优先级（已修复或状态正确）

2. **路径3**：401号修复已解决 memory 节点暴增问题。k_full.jsonl 本身只记录图拓扑增量，是正确的。
3. **路径4/6/7/8**：相关机制已实现。

## 追加审查：耦合振荡 → S_net 共振 → 能指序列 + connective_pattern → 输出 通路

### 1. 耦合振荡（oscillation/coupling）：已实现

**实现位置**：`snet_activation.py`（`SNetActivation` 类）+ `traversal.py:996-998` + `daemon.py:304-476`

耦合振荡的完整机制：
- `daemon.py:453-476`：`_setup_snet_activation()` 在引擎初始化时构建 concept↔signifier 双向映射，创建 `SNetActivation` 实例，绑定到 `TraversalEngine`
- `traversal.py:996-998`：**每一步穿越**结束后调用 `self._snet_activation.activate(self.position)`——K_active 步进到新概念时，S_net 同步激活对应能指
- `snet_activation.py:184-203`：`activate()` 方法——激活新能指，保留与新能指有**组合轴连接**的旧激活，清理失去连接的旧激活
- `snet_activation.py:205-264`：`resonate()` 方法——operator 输入引起的共振，通过对话焦点集（`_dialogue_focus_set`）控制激活态。**不用衰减**，只用拓扑判据（组合轴连接）
- `traversal.py:859-950`：`walk()` 方法中 S_net 共振影响穿越路径选择——`_resonating_candidates()` 返回共振区域的 K_active 概念 ID，`_pick_with_resonance()` 在同优先级候选中优先选择共振候选（pull, not override）

### 2. S_net 共振产出能指序列：已实现

**实现位置**：`snet_activation.py:274-338`

当 `activate()` 清理旧激活时，被清出的能指如果构成连贯语段（≥2 个通过组合轴连通的能指），存入 `internal_speech_buffer`：
- `_check_fragment_formation()`（`snet_activation.py:274-294`）：检测被清出的能指中的组合轴连通子集
- `_find_connected_subset()`（`snet_activation.py:296-318`）：BFS 沿组合轴找到最大连通子集
- 产出 `InternalSpeechFragment`（`snet_activation.py:31-64`），包含：
  - `signifiers: tuple[str, ...]`：能指序列
  - `connective_patterns: tuple[str, ...]`：能指之间的连接模式（来自 S_net 边的 evidence）
  - `source_concepts: tuple[str, ...]`：对应的 K_active 概念 ID

### 3. connective_pattern：已实现

**实现位置**：`snet_activation.py:320-330`

`_extract_connectives()` 方法从能指集合中提取组合轴边的 evidence 字符串作为 `connective_patterns`。这些 pattern 来自 S_net 边的 `evidence` 字段（在辞典 bootstrap 和对话回写时写入）。

`InternalSpeechFragment.connective_patterns` 在后续流程中的消费：
- `internal_speech.py:251-255`：`snapshot_to_prompt()` 将 connective_patterns 格式化为 LLM prompt 中的"连接模式"
- `internal_speech.py:489-491`：`_build_fallback_text()` 在 LLM 不可用时直接使用 connective_patterns 拼接输出

### 4. 输出路径：LLM 做最小语法填充（主路径）+ 直接输出（退化路径）

**LLM 路径（主路径）**：`internal_speech.py:320-443`

`externalize()` 函数的完整流程：
1. `build_snapshot()` → 收集已成型语段（`formed_fragments`）、激活能指、settlement 约束
2. `snapshot_to_prompt()` → 将能指序列 + connective_patterns 转化为 LLM 的 (system_prompt, user_prompt)
3. LLM 调用（system prompt 定义角色为"逢亮的发声器官"——`internal_speech.py:207-208`）
4. LLM 的约束：
   - `locked_signifiers`（必须使用）
   - `excluded_signifiers`（禁止使用）
   - 已成型语段作为"已有材料"提供给 LLM
   - LLM 做的是**语法填充**（"将内部言语转化为语法上通顺的自然语言"），不是内容生成

**直接输出路径（退化路径）**：`internal_speech.py:475-503`

`_build_fallback_text()` 在 LLM 不可用时：
- 从已成型语段直接拼接：`能指A、能指B——connective_pattern`
- 无 LLM 参与，纯粹是能指序列 + connective_patterns 的字符串拼接

### 通路完整性评估

| 环节 | 状态 | 位置 |
|------|------|------|
| K_active 步进 → S_net activate | 已实现 | `traversal.py:998` |
| Operator 输入 → S_net resonate | 已实现 | `daemon_api.py:1306` → `snet_activation.py:205` |
| S_net 共振 → 穿越路径偏向 | 已实现 | `traversal.py:888, 931-950` |
| 激活态清理 → 能指序列 + connective_pattern 成型 | 已实现 | `snet_activation.py:274-330` |
| 已成型语段 → LLM 最小语法填充 → 输出 | 已实现 | `internal_speech.py:320-443` |
| 已成型语段 → 直接拼接输出（退化） | 已实现 | `internal_speech.py:475-503` |
| 输出 → writeback 回写 S_net | 已实现 | `internal_speech.py:395-405` |
| 输出侧断裂检测 | 已实现 | `internal_speech.py:407-408` |

**结论**：耦合振荡 → S_net 共振 → 能指序列 + connective_pattern → LLM 最小语法填充/直接输出 的完整通路**已实装**。代码实现与设计意图一致。

**但有一个结构性问题**：这条通路目前只在 dialogue 输入路径（`present_json` 中 `input_class == "dialogue"` 分支）触发。非 dialogue 路径走的是旧的 `daemon.feed(text)` + `_extract_concepts` + co-gaze 模式，不经过 externalize 流程。这与路径5的问题是同一个根源——非 dialogue 路径未迁移到新的数据流架构。

## 谱系关系

- 前置：401号（memory 注入轨迹 vs 产物区分）
- 相关：phi_L 管辖范围崩溃（垃圾节点问题的根源）
