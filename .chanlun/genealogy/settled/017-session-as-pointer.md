# 017 — Session 是指针，不是叙事

- **状态**: 已结算（定理，016的推论）
- **类型**: meta-rule
- **日期**: 2026-02-17
- **结算依据**: 016（规则无代码强制不执行）+ 014/015 的同构应用

## 命题

Session 文件应该是指向文件系统中已有状态的指针（~50行），不是全量叙事（1035行）。任何规则容器都会膨胀，解法是从全量加载变为按需索引。

## 发现过程

### 触发事件

编排者指出 session 文件在重演 SKILL.md 的膨胀模式。手动 session（1035行）把本该活在别处的内容全部内联：代码修复细节（= git commit messages）、哲学讨论（= genealogy entries）、定义版本表（= definitions/*.md 本身）。

### 关键观察

这是同一个架构操作的第三次应用：

| 实例 | 膨胀形式 | 解膨胀操作 | 结果 |
|------|---------|-----------|------|
| SKILL.md | 633行单体 | 核心卡(99行) + 按需加载agent卡 | 014 |
| Session | 1035行叙事 | 指针(~50行) + 内容活在 definitions/genealogy/git | 017（本条） |
| Hook | （潜在：全量扫描所有规则） | 按 staged files 索引命中规则 | 016已预见 |

统一原则：**任何容器膨胀的成本从 O(n) 变 O(1)，方法是按需索引**。不是消灭膨胀，是让膨胀的位置正确——内容活在专门的文件系统位置，索引/指针活在需要快速恢复的位置。

### 具体变更

1. 消灭"手动 session"概念——不再区分 precompact/手动，统一为 `*-session.md` 格式
2. Session 只存：git 状态 + 定义基底表（指针） + 谱系计数（指针） + 中断点（session 独有价值）
3. Precompact hook 产出统一格式 session，继承上一个 session 的中断点
4. Ceremony 不再偏好"非 precompact"文件，取最新 session 即可
5. 旧 session 文件移入 archive/

## 推导链

1. 016 证明：规则无代码强制就不执行
2. Session 文件的"50行封顶"是规则。如果不强制，CC 自然退化为"塞更多以防万一"
3. 但 session 50行封顶是**结构性强制**：hook 产出的 session 格式就是50行左右，CC 没有渠道往里塞叙事
4. 这比 016 的 pre-commit hook 更强：不是"做了检查"，是"结构上不可能违反"
5. ∴ 从"规则 + 检查"升级到"结构不允许"是更好的强制方式

## 谱系链接

- **前置**: 014（SKILL.md 卡组分解）— 同构：全量内容 → 核心 + 按需索引
- **前置**: 016（runtime 强制层）— 前置发现：任何规则需要强制机制
- **关联**: 015（方法=内容）— session 的形式（全量叙事）被 session 应存的内容（指针）否定

## 影响

- `.claude/hooks/precompact-save.sh`: 产出统一格式 `*-session.md`
- `.claude/commands/ceremony.md`: 不再区分手动/precompact，187行→137行
- `references/session-template.md`: 新增，定义50行封顶格式
- 旧 session 文件归档到 `.chanlun/sessions/archive/`

## 来源

- [新缠论] — 编排者诊断："session不应该是全量叙事，应该是指针"
- 统一原则来自编排者："任何规则容器都会膨胀，解法永远是同一个——从全量加载变为按需索引"
