# 自生长回路集成设计（#9 前置调研）

**日期**: 2026-02-20
**作者**: hook-validator 工位
**依据**: 043号谱系（自生长回路）、044号谱系（相位切换 runtime 强制）

## 回路总览（043号）

```
Session 操作序列
  → post-session hook 检测模式 → pattern-buffer
    → pattern settled → skill-crystallizer 结晶
      → 新 skill 注册到 manifest
        → ceremony 加载 → 新 Session
```

## 三个连接点实现状态

### 连接点 1：post-session hook → pattern-buffer

**状态: 已实现（#3 完成）**

| 组件 | 文件 | 状态 |
|------|------|------|
| post-session-pattern-detect.sh | `.claude/hooks/post-session-pattern-detect.sh` | 已实现，已注册 |
| settings.json Stop hook 注册 | `.claude/settings.json:50-58` | 已注册 |
| pattern-buffer.yaml | `.chanlun/pattern-buffer.yaml` | 已创建（空，patterns: []） |

**输出格式验证**:

hook 输出 pattern 条目格式：
```yaml
- id: "pat-{hash}"
  signature: "{trigram}"        # 如 "Read→Edit→Bash"
  frequency: {count}
  first_seen: "{ISO timestamp}"
  last_seen: "{ISO timestamp}"
  sources: ["{session-id}"]
  status: candidate             # 或 observed（无重复序列时）
```

**与 dispatch-spec 一致性**:
- dispatch-spec `automation.pattern_detection.buffer_path`: `.chanlun/pattern-buffer.yaml` -- 一致
- dispatch-spec `automation.pattern_detection.promotion_threshold`: 3 -- hook 中 trigram 检测阈值为 2（candidate），crystallization-guard 中阈值为 3（block）-- 一致

**问题**: pattern-buffer.yaml 中 status 字段值为 `candidate`/`observed`，但 crystallization-guard.sh 检查的是 `status: pending`。需要统一。dispatch-spec 未明确定义 status 枚举。

### 连接点 2：pattern settled → skill-crystallizer

**状态: 未实现**

043号谱系定义："谱系节点 settled 且 type=strategy → 自动触发 skill-crystallizer"。当前没有任何 skill-crystallizer 组件。

**缺失组件**:

1. **skill-crystallizer agent**（`.claude/agents/skill-crystallizer.md`）
   - 触发条件：pattern-buffer 中某 pattern 的 frequency >= promotion_threshold（3）
   - 输入：pattern 的 signature + sources（session IDs）
   - 动作：
     a. 读取 sources 中的 session 文件，提取该 pattern 的具体操作上下文
     b. 生成 skill 文件（`.claude/commands/{name}.md`）
     c. 更新 pattern-buffer 中该 pattern 的 status 为 `promoted`
     d. 更新 manifest.yaml 注册新 skill
   - 输出：新 skill 文件 + manifest 更新

2. **触发机制**（两种方案）:

   **方案 A: Hook 触发（推荐）**
   - 在 crystallization-guard.sh 中，当检测到 frequency >= 3 的 pending pattern 时，不仅阻断 commit，还注入 systemMessage 指示 Lead spawn skill-crystallizer agent
   - 优点：复用现有 hook 基础设施（042号模式）
   - 缺点：只在 git commit 时触发，不够及时

   **方案 B: ceremony 步骤触发**
   - 在 ceremony.md 中增加"结晶检查"步骤：扫描 pattern-buffer，对达标 pattern 自动 spawn skill-crystallizer
   - 优点：每次 session 开始时检查，更及时
   - 缺点：增加 ceremony 复杂度

   **建议**: 两者结合。ceremony 中增加结晶检查步骤（主路径），crystallization-guard 作为兜底（防止遗漏）。

3. **pattern status 状态机**:
   ```
   candidate → pending → promoted（结晶成功）
                       → rejected（不值得结晶）
   observed → (累积到 candidate 阈值后) → candidate
   ```
   当前 post-session hook 输出 `candidate`/`observed`，需要一个升级机制将跨 session 的同签名 pattern 合并并升级 frequency。

### 连接点 3：新 skill → manifest 注册 → ceremony 加载

**状态: 部分实现（#4 完成 manifest，加载机制待 #7）**

| 组件 | 文件 | 状态 |
|------|------|------|
| manifest.yaml | `.chanlun/manifest.yaml` | 已实现（#4），包含 skills/agents/hooks 三类 |
| manifest 自动生成脚本 | 未知 | 需确认是否有重新生成脚本 |
| ceremony 加载 manifest | `.claude/commands/ceremony.md` | 需确认是否已读取 manifest |
| 进化形态学架构 | #7 infra-builder 进行中 | 待完成 |

**dispatch-spec 路径不一致**:
- dispatch-spec `automation.manifest.path`: `.chanlun/manifest.yaml` (已修正)
- 实际位置: `.chanlun/manifest.yaml`
- 已统一。

**缺失组件**:

1. **manifest 自动注册接口**: skill-crystallizer 结晶新 skill 后，需要自动追加到 manifest.yaml。当前 manifest 头部注释写"自动生成，勿手动编辑"，暗示有生成脚本，但需要一个追加接口（而非全量重新生成）。

2. **ceremony 加载机制**: ceremony 启动时应读取 manifest.yaml，确认所有 active skill 可用。当前 ceremony.md 是否已引用 manifest 需确认。

3. **进化形态学（#7）**: infra-builder 正在设计的架构将定义 skill 的生命周期管理，包括版本控制、废弃、替换等。连接点 3 的完整实现依赖 #7。

## 缺失连接汇总

| # | 缺失连接 | 依赖 | 实现方案 |
|---|---------|------|---------|
| M1 | pattern status 枚举统一 | 无 | 统一 post-session hook 和 crystallization-guard 的 status 值 |
| M2 | 跨 session pattern 合并 | M1 | post-session hook 增加合并逻辑：同 signature 的 pattern 累加 frequency |
| M3 | skill-crystallizer agent | M1, M2 | 新建 `.claude/agents/skill-crystallizer.md` |
| M4 | 结晶触发机制 | M3 | ceremony 步骤 + crystallization-guard 兜底 |
| M5 | manifest 追加接口 | M3 | skill-crystallizer 直接追加 YAML 条目到 manifest |
| M6 | dispatch-spec manifest 路径修正 | 无 | 更新 dispatch-spec `automation.manifest.path` |
| M7 | ceremony manifest 加载 | M5, #7 | ceremony 启动时读取 manifest 验证 skill 可用性 |
| M8 | 效用背驰检测 | M3 | 制动力机制：skill 数量/复杂度超过收益时停止生长 |

## 依赖关系图

```
M1 (status 枚举统一)
 ├→ M2 (跨 session pattern 合并)
 │   └→ M3 (skill-crystallizer agent)
 │       ├→ M4 (结晶触发机制)
 │       └→ M5 (manifest 追加接口)
 │           └→ M7 (ceremony manifest 加载) ← #7 (进化形态学)
 └→ M6 (dispatch-spec 路径修正) [独立]

M8 (效用背驰检测) [独立，可后置]
```

## 实施优先级建议

1. **立即可做**（无外部依赖）: M1, M6
2. **M1 完成后**: M2（增强 post-session hook）
3. **M2 完成后**: M3（skill-crystallizer agent 定义）
4. **M3 完成后**: M4, M5（触发 + 注册）
5. **#7 完成后**: M7（ceremony 加载）
6. **系统运行一段时间后**: M8（效用背驰，需要实际数据）

## 边界条件

- 如果 #7（进化形态学）的架构与本设计冲突 → M5/M7 需要根据 #7 的产出调整
- 如果 pattern-buffer 的 trigram 检测粒度不够（误报/漏报）→ 需要更复杂的签名算法（如 n-gram 或序列对齐）
- 如果 skill-crystallizer 产出的 skill 质量不足 → 需要 quality-guard 在结晶路径上增加检查点
- 如果效用背驰信号缺失 → skill 数量无限膨胀（020号"纯扩张=窒息"）
