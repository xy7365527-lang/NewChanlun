# /ceremony — 创世仪式（冷启动 / L2 热启动）

蜂群启动时执行。根据是否存在 session 文件自动选择冷启动或热启动模式。

## 用法

```
/ceremony
```

在新的蜂群会话开始时调用一次。命令自动检测模式，无需手动指定。

---

## 模式判定

执行以下检测序列，确定启动模式：

1. 扫描 `.chanlun/sessions/` 目录
2. 查找 `*-session.md` 文件（统一格式，不再区分手动/precompact）
3. 按修改时间倒序取最新的一个

| 条件 | 模式 | 说明 |
|------|------|------|
| 无 session 文件，或 `.chanlun/sessions/` 不存在 | **冷启动** | 首次使用或历史记录清空 |
| 存在 session 文件 | **热启动（L2）** | 跨会话恢复 |

**兼容**：旧格式的 `*-precompact.md` 和无后缀的手动 session 仍可被识别，但新产出的 session 统一为 `*-session.md`。

---

## 冷启动（Cold Start）

无 session 文件时执行。

### 执行步骤

1. **加载定义** — 扫描 `.chanlun/definitions/`，读取每个定义的版本和状态
2. **加载谱系** — 扫描 `.chanlun/genealogy/`，区分 `pending/` 和 `settled/`
3. **加载目标** — 从 CLAUDE.md 读取当前阶段目标和核心原则
4. **状态报告** — 输出完整仪式报告（见下方输出格式）
5. **确认** — 列出关键概念的当前理解，**等待编排者确认或校正**

### 输出格式

```markdown
## 创世仪式完成（冷启动）

### 定义基底
- 已结算定义：[N] 条
- 生成态定义：[M] 条
- [列出每条定义的名称、版本和状态]

### 谱系状态
- 生成态矛盾：[N] 个
- 已结算记录：[M] 个
- [列出每个生成态矛盾的ID和简述]

### 当前目标
[从 CLAUDE.md 读取]

### 待确认
以上理解是否正确？如有偏差请指出。
```

---

## 热启动（Warm Start / L2 跨会话恢复）

检测到 session 文件时执行。目标：最小化重复加载，最大化恢复速度。

### 执行步骤

1. **定位 session** — 取 `.chanlun/sessions/` 中最新的 session 文件
2. **版本对比** — 扫描 `.chanlun/definitions/` 当前版本，与 session 中"定义基底"表格对比：
   - **未变更**：标记为 `=`，跳过重新验证
   - **版本升级**：标记为 `↑`，读取变更摘要
   - **新增定义**：标记为 `+`
   - **定义消失**：标记为 `-`（异常，需警告）
3. **谱系差异** — 对比当前 `.chanlun/genealogy/` 与 session 记录：
   - 新增的 pending 谱系
   - 从 pending 移入 settled 的谱系
4. **加载中断点** — 从 session 文件的"中断点"章节读取
5. **输出差异报告**
6. **直接进入蜂群循环** — **不等待编排者确认**

### 输出格式

```markdown
## 热启动完成（L2 跨会话恢复）

**恢复自**: [session 文件名]
**session 时间**: [时间]

### 定义基底差异
| 定义 | session版本 | 当前版本 | 状态 | 变化 |
|------|-----------|---------|------|------|
| [name] | [v_old] | [v_new] | [status] | [=/↑/+/-] |

### 谱系差异
| ID | session状态 | 当前状态 | 变化 |
|----|-----------|---------|------|

### 中断点恢复
[从 session 文件读取]

### 蜂群评估
[评估可并行工位，列出本轮计划]

→ 进入蜂群循环
```

---

## 边界情况处理

### session 文件过旧
如果最新 session 文件的日期距今超过 7 天：
- 仍然执行热启动（session 文件再旧也比冷启动有信息增益）
- 在报告头部追加警告：`⚠ session 距今 [N] 天`

### 定义目录或谱系目录不存在
- 如果 `.chanlun/definitions/` 不存在：降级为冷启动
- 如果 `.chanlun/genealogy/` 不存在：创世仪式负责创建

### 兼容旧格式 session
如果只找到旧格式文件（`*-precompact.md` 或无后缀），仍可读取其定义基底表格和谱系状态。中断点章节可能缺失，此时从 CLAUDE.md 和当前 git 状态推断优先级。

---

## 075号架构变更

自 075号谱系起，结构能力（genealogist/quality-guard/meta-observer/code-verifier）不再作为 teammate spawn，
而是由 `dispatch-dag.yaml` 的 `event_skill_map` 定义为事件驱动 skill。

- ceremony 只 spawn 业务工位，不再 spawn 结构工位
- skill 在对应事件发生时自动触发（task_complete、file_write、session_end 等）
- 子蜂群自动继承所有 skill，无需 spawn 结构 teammates

## 注意

- 冷启动：编排者确认后质询循环才正式启动
- 热启动：**不等待确认**，差异报告输出后直接进入蜂群循环
- session 文件是指针，不是叙事。50行封顶。叙事走 git commit messages 和 genealogy
- 已结算定义（状态=已结算 且 版本未变更）不需要重新验证
