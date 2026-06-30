# 情况说明（给外部 AI 咨询用，自包含）

这是一个 Rust 量化回测引擎项目（缠论技术分析 + 拓扑方法）。当前有**两个待决议题**需要外部视角。两个议题相互独立，可分开回答。

---

## 议题一：增量回测的 O(n²) ceiling — 能否做到 per-bar O(1) 摊还？

### 背景
回测引擎逐 bar 推进。每个 bar 要维护一棵"多级别走势结构树"（tree），树随行情增长。我们要求**增量维护**：第 n 个 bar 的处理代价应与"本 bar 的变动量"成正比，而非与"整棵树的大小 n"成正比。验收指标：在 16000 bar 规模下，per-bar 处理时间对 n 的经验指数（exp）应 ≈1.0（即总时间 ~O(n)，单 bar ~O(1) 摊还）。

### 已完成的部分
处理分两阶段：
- **merge 阶段**（把本 bar 新结构合并进持久状态）：已增量化成功。merge_s@16K 从 0.059s 降到 0.012s（降 5×），且通过 bit-exact 验证（增量结果 == 全量重算，逐 bit 相同）。
- **extract 阶段**（从树提取"元素视图" + 构建 3 个索引）：**仍是瓶颈**。

### 卡住的点：extract 阶段的 "TreeKey-miss ceiling"
树有一个缓存键 TreeKey。大多数 bar 命中缓存，O(1) 复用。但当**出现更高级别的新结构时**（CL 品种 16K bar 中约 26 次），缓存 miss，触发**全量重建**：
- `extract_elements(tower)` 遍历整棵树 O(tree)
- 3 个 `build_*_index` 各 O(tree)

miss 发生在大 n 端时树已经很大，累积成本 O(Σ tree_at_miss)，这是一个小系数的超线性项。实测 @16K：x_exp（extract）=2.09，m_exp（merge）=2.07，都还在 ≈2.0，没到目标 1.0。

代码注释里已标注这是已知 ceiling：「更高级别新出现时根结构重构，前缀不可简单 append 复用」。

### 核心问题（要问的）
1. 当"树的根/最高层级会随数据增长而改变结构"时，是否存在一种**增量维护 + 索引重映射**方案，让 miss 时也只付 O(变动) 而非 O(整树)？还是说这种"根结构重构"本质上无法增量化，O(n log n) 或更高是理论下界？
2. 一个具体方向：miss 时检测"最高级别变化的边界"，只重建变化的部分，重用未变前缀的索引（做索引重映射而非重建）。这个方向的**正确性风险**在哪？（我们有 bit-exact 验证作为安全网，但想先知道理论上的陷阱。）
3. 有没有这类"持久化 + 偶发根重构"数据结构的成熟范式可借鉴（如 finger tree、persistent segment tree、版本化 B-tree 的某种变体）？

---

## 议题二：事件溯源（event-sourcing）系统的 reader/writer 契约分裂

### 背景
有一个"目标进度追踪"系统，用 append-only 事件日志（events.jsonl）记录进度。一个纯函数 reducer 读所有事件 + git 当前状态，重算出"当前目标状态"（哪些验收项通过了）。

事件类型（writer 端 SCHEMA 定义 7 种）：GOAL_SET（设目标，带 base_head = 设定时刻的 git commit 锚点）、EVIDENCE（叙事证据，自由文本）、CHECK_PASS（验收项通过）、BLOCKED、DECOMPOSE、SUPERSEDE、CLOSED。

### 矛盾
发现 **reader（reducer）依赖一个 writer 从不产生、SCHEMA 未定义的事件类型 GOAL_RESUME**：
- reducer 读 GOAL_RESUME 来"在 session 热启动时把 base_head 重锚到新的 git HEAD"。
- 但 writer 的合法事件里没有 GOAL_RESUME。历史上 2 条 GOAL_RESUME 全是**绕过 writer 裸 append** 进日志的。

同时还有第二个分裂：**验收项闭合只认 CHECK_PASS，不认 EVIDENCE**。结果是——3 个验收项机制上已达成（有测试通过、有 EVIDENCE 记录），但 reducer 显示全部"未通过"，因为没有 CHECK_PASS 事件。而正常 /goal 协议循环只写 EVIDENCE，从不自动写 CHECK_PASS。goal 因此永远无法正常闭合。

### 三个候选立场
- **A**：reducer 是对的，补 writer/SCHEMA 让 GOAL_RESUME 合法化（带 base_head）。但后果是每次 resume 都重锚 base_head，导致"base_head 过时"的预警信号永远为 False。
- **A2**（第三方案）：补 writer 让 GOAL_RESUME 合法，**但它不带 base_head**——base_head 永远等于最初 GOAL_SET 时刻的锚，"过时"信号保留为有价值的降级预警。GOAL_RESUME 降为纯恢复记录（只记 goal_id + note）。
- **B**：reducer 越界了，删掉它的 RESUME 重锚逻辑。承认"base_head 过时"是正确的降级信号，不该被事件抹平。

### 附带的前置缺口
若要让"验收闭合认 EVIDENCE→CHECK_PASS 提升路径"（即定义"谁有权把 EVIDENCE 的验证结论封为 CHECK_PASS"），会撞到：EVIDENCE 当前没有稳定 id，所以 CHECK_PASS 无法用 evidence_ids 机器溯源到具体 EVIDENCE。需先给 EVIDENCE 加稳定 id。

### 核心问题（要问的）
1. 在事件溯源架构里，reader 依赖 writer 不产生的事件类型，这是不是一定是 bug？还是说"reader 宽容读历史 ⊋ writer 严格守新写"可以是**有意的有效域分层**？分层的合法边界在哪？
2. base_head 这种"锚点"语义：它应该是**不可变的历史锚**（GOAL_SET 时刻固定，A2/B），还是**可随 session 推进的活动指针**（A）？"过时预警"信号的价值有多大？
3. 三个立场（A / A2 / B）哪个在事件溯源的设计原则下最自洽？A2 是真正的扬弃还是把矛盾藏起来的折中？

---

## 当前事实快照（供参考）
- 5 个验收项：[1]ElementId bit-exact ✅机制达成 / [2]ΔSharpe≠0 ✅机制达成 / [3]L2/L3全窗8品种 ⏳进行中(1/8坐实) / [4]O(n)exp≈1.0 ⚠议题一卡住 / [5]Nautilus集成 ✅机制达成
- reducer 显示 passed=[]（0/5），正是议题二的后果——机制达成≠CHECK_PASS闭合。
- 两个议题都是"停下来不绕过"的真实矛盾，不是赶工能解决的。
