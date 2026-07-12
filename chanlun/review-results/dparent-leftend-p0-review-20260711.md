# P0 定义复议裁决（#43）：D_parent 左端口径（2026-07-11）

- 状态：**RULING / SUPERSEDING-IN-PART**
- 范围：仅复议 `D_parent` 左端、相关字段映射、`D_child` 是否并案，以及下一次重放验收标准。
- 写入边界：本裁决不改代码、不执行重放、不回写既有冻结文档。
- 上游材料：
  - `chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md`（任务 B，定义谱系审查）；
  - `chanlun/review-results/dr-l1l2-zero-code-audit-20260711.md`（任务 A，代码实现层审计）；
  - `chanlun/review-results/dparent-freeze-20260710.md`（被复议冻结）；
  - `chanlun/review-results/p0-replay-dparent-20260710.md`；
  - `chanlun/review-results/dparent-funnel-20260710.md`。

## 0. 总裁决

1. **Q1：改。** `D_parent` 左端从“当前 C 离开 episode 的局部起点”改为“完整父级 `c` 走势类型的结构起点”。这里的完整 `c` 是父级 `a+A+b+B+c` 中最后一个同级别中枢 `B` 之后、最终构成该级别背驰段的 `c`；**不是**父级 `A` 起点，也不是整个 `a+A+b+B+c` 的起点。
2. **Q2：拆分对象。** 当前 `departure_move_c_start(...)` 的窗口及 last-reentry 语义只能无条件映射为 `c_episode_start`；它不得继续无证明地充当 `c_start_full`。完整 `c` 的左端必须由递归走势类型的结构归属给出。reentry 只切分局部 episode，不自动切断完整 `c`。
3. **Q3：单独立项。** `D_child := child.a_interval` 不与本轮父级左端并案改定义；它作为待复议映射暂时保留，以便隔离验证本裁决的单一变量效应，但本裁决不对其原文正确性重新背书。
4. **Q4：登记可证伪预测。** 只把父级左端从局部 episode 改到完整 `c` 后，任务 B 所列三个 L1→L2 近失样本预计仍不通过。若重放出现非零 L1→L2 产出，合法来源只能是子级被检对象确实完整落在父级 `c` 内部的真包含样本；不得来自把父窗扩到父级 `A` 或整个父趋势。

## 1. 裁决表

| 问题 | 裁定 | 主要依据 | 反方论证 | 对反方的裁断 |
|---|---|---|---|---|
| Q1：是否将左端改为完整父级 `c` 起点 | **是，立即取代局部 episode 口径** | 第 29 课把最后中枢后的“最后的背驰段”定位为父级末段；第 37 课规定 `c` 至少包含对 `B` 的第三类买卖点，并要求“对 `c` 的内部”递归套用 `a+A+b+B+c`。任务 B 据此把父对象闭合为完整 `c`，而非 `c` 内一次重新离开 episode（任务 B §1、§2、§5.1）。 | last-reentry 定界能避免把“失败离开→回中枢→重新离开”桥接成一个 episode，且已有测试和重放均按此语义稳定工作。 | 该论证证明了 **episode 定界**的正确性，不证明 episode 与父级背驰段 `c` 同一。把局部面积/离开 episode 的边界直接提升为区间套父对象，是对象层级错配。应保留 episode 对象，但不得让它覆盖完整 `c` 对象。 |
| Q2：窗口/reentry 如何映射 | **完整 `c` 与局部 episode 分字段；见 §3** | 当前函数窗口为 `start_index ∈ [B.end_index, until_start]`，`episode_start_in` 以窗口内最后一个回中枢反向段为边界，再取其后首个同向锚段；任务 A 明确确认该实现无 off-by-one，但其返回对象就是“当前 episode 起点”（任务 A §1.1）。 | 继续复用 `enter_src/interval` 最简单，并可保持既有不变量 `enter_src == interval.0`。 | 简单性不能替代语义同一性。若 `c_episode_start != c_start_full`，复用字段会让声明与能力不一致。只有逐事件证明二者相等时才允许物理复用；语义上仍须分名。 |
| Q3：`D_child := child.a_interval` 是否同轮复议 | **不同轮；单独设 P0 定义项** | 任务 B 指出 `child.a_interval` 可能是比较用 A 腿而非低级别背驰段，并给出 `span(A,C)`/`C_child` 两个待裁候选（任务 B §5.2、§7）；任务 A 也把它明确归为定义问题而非实现 bug（任务 A §4）。 | 父、子是同一个包含谓词两端；只改父端会留下一个可能仍错误的整体判据，似乎应一次改完。 | 正因两端都可能变化，必须隔离变量。并案会使产量变化无法归因于父左端还是子对象。下一次重放先固定现有 `D_child` 只验证 #43；随后独立裁决 `D_child`，再进行第二轮重放。暂时保留不等于确认其正确。 |
| Q4：L1→L2 如何验收 | **以“可证伪预测 + 逐样本结构证明”验收** | 代码审计排除了实现误杀；三例唯一失败原子均为 `child.a_interval.0 >= parent.enter_src`，左界缺口分别为 11504/19853/34445 bar（任务 A §2；漏斗 §④）。任务 B 判断这些子证据位于父 `c` 之前，不应因合法修正而被纳入（任务 B §3、§6）。 | 还没有计算三个父事件的 `c_start_full`，因此“仍不通过”不是已证明事实；完整 `c` 左移后也可能碰巧覆盖其中某例。 | 同意其不是事实，故明确登记为**预测**而非定义前提。任何一例通过即证伪预测，必须报告其 `c_start_full` 的递归结构证据；不能事后改定义保住预测。预测被证伪不自动推翻 Q1，但会触发对样本归属或 Q3 的再审。 |

## 2. Q1 的规范定义

### 2.1 父级对象

对任一父级趋势背驰事件 `p`，令 `B_p` 为该父级趋势最后一个同级别中枢，令 `c_p` 为 `a+A+b+B+c` 中位于 `B_p` 之后、最终产生该级别背驰转折的完整 `c` 走势类型：

```text
c_start_full(p) := start(c_p)
c_end_full(p)   := end(c_p)

D_parent(p) := [c_start_full(p), c_end_full(p)]
```

`start(c_p)` 必须由递归走势类型的结构分解确定，并满足：

1. `c_p` 在结构上属于父级最后背驰段，而非父级确认窗；
2. `c_p` 包含对 `B_p` 的第三类离开/买卖点结构；
3. 下一级递归分析发生在 `c_p` 内部；
4. 左端不得早到父级比较段 `A`，也不得早到整个父级 `a+A+b+B+c` 的起点；
5. 左端不得因产量、近失排名或 `epsilon` 容忍带事后选择。

### 2.2 来源标签与边界

- `D_parent` 作为“父级背驰段 `c`”的对象口径：**[旧缠论]**（第 29/37 课）。
- 从原文对象到代码字段的离散映射：**[旧缠论:选择]**，必须接受本裁决的结构证明和重放证伪。
- `c_end_full(p)` 本轮继续取父级背驰对应转折段的结构端点。若既有 `parent.interval.1 = seg.end_index` 不能证明等于 `end(c_p)`，则实现前必须先报告能力冲突，不得用确认时点回填。

## 3. Q2 字段映射规格（不改代码）

### 3.1 对象分离

| 语义对象 | 规范字段名 | 左端定义 | 右端定义 | reentry 的作用 |
|---|---|---|---|---|
| 完整父级 `c` 走势类型 | `c_interval_full` | `c_start_full = start(c_p)`，由递归走势分解确定 | `c_end_full = end(c_p)` | reentry 不是充分的截断条件；只有结构分解证明某前缀仍属 `B_p` 而不属 `c_p` 时，才可排除该前缀 |
| 当前 C 离开 episode | `c_episode_interval` | `c_episode_start`，即当前窗口内 last-reentry 之后首个同向锚段起点 | 当前 episode 的结构末端 | last-reentry 正是该对象的局部边界 |
| 父级区间套对象 | `D_parent` | `c_start_full` | `c_end_full` | 不直接读取 last-reentry 边界 |
| 算法确认点 | `confirm_src` | 点坐标 | 点坐标 | 与两种区间均无相等不变量，仅作诊断 |

### 3.2 `departure_move_c_start` 的映射规则

现函数语义不作代码层修改，但字段契约按下列规则冻结：

```text
departure_move_c_start(..., B_p, dir, until_start)
    -> c_episode_start
    != c_start_full                 // 一般情形，不设相等公理
```

1. **窗口映射**：现有 `[B_p.end_index, until_start]` 是“截至当前判破段的 episode 搜索窗”，映射到 `c_episode_search_window`。完整 `c` 的搜索域必须改由递归结构给出的 `c_p` 归属域承载，概念上为 `[start(c_p), end(c_p)]`；不得把一个随 `until_start` 移动的局部搜索窗直接定义为完整走势类型。
2. **reentry 映射**：`episode_start_in` 的 last-reentry 边界只更新 `c_episode_start`。一次价格/几何意义上的回中枢，不能单独证明此前缀不属于完整 `c_p`；若要从 `c_interval_full` 排除前缀，必须有走势分解把该前缀归还给 `B_p` 的独立证书。
3. **字段承载**：若保留 `enter_src` 作为 `D_parent` 左端，则新语义必须为 `enter_src := c_start_full`；当前函数返回值应另存为 `c_episode_start`。若仍让 `interval` 承载 `D_parent`，则必须满足 `interval = c_interval_full`，而不是 `c_episode_interval`。
4. **允许复用的唯一条件**：逐事件有结构证书证明 `c_episode_start == c_start_full` 时，两字段可共享数值；该相等是事件事实，不是全局定义。
5. **禁止映射**：不得把 `parent.a_interval.0`、父级整个趋势起点、`confirm_src`、`until_start` 或任意产量驱动锚点映射为 `c_start_full`。

本节只是字段映射规格。它不指定函数名、数据结构或迁移步骤，不构成代码变更授权。

## 4. Q3 单独立项边界

另立定义项：**“D_child / I(A_child) 区间套对象消歧”**。该项至少必须在以下候选间作原文回溯和机器字段裁决：

```text
候选 1：D_child = child.a_interval
候选 2：D_child = child.c_interval_full
候选 3：D_child = span_ac(child) = [start(A_child), end(C_child)]
```

本裁决对该后续项只规定三条边界：

1. 在 #43 的隔离重放中，继续使用 `D_child := child.a_interval`，仅用于比较父左端变化；
2. 报告中必须标注它是“provisional / pending separate ruling”，不得写成已由 #43 再确认；
3. 后续 `D_child` 裁决不得回改 #43 的重放结果；应另跑一轮，并分别归因父对象与子对象的产量变化。

## 5. 对 `dparent-freeze-20260710.md` 的 supersede 关系

本裁决不删除、不改写历史冻结文件。以下以“条目级取代”生效：

| `dparent-freeze-20260710.md` 条目 | 处理 | #43 生效口径 |
|---|---|---|
| §2 `lambda_enter := enter_src := interval.0` 被定义为当前 C episode 起点 | **部分取代** | 拆为 `c_start_full` 与 `c_episode_start`；若 `enter_src` 继续作为父窗左端，则映射到前者 |
| §3 `D_parent := [p.enter_src, p.interval.1] = p.interval` | **取代其左端及等同前提** | `D_parent := [c_start_full, c_end_full]`；`p.interval` 只有映射为 `c_interval_full` 后才可继续等同 |
| §3 左端唯一来源为 `departure_move_c_start(...)` | **完全取代** | 该函数现语义只无条件提供 `c_episode_start`；`c_start_full` 来自递归走势分解 |
| §3 右端为父级背驰对应转折段结构端点，且不由 `confirm_src` 派生 | **保留** | 继续有效；但要求证明现字段确为 `end(c_p)` |
| §3 排除确认窗、`confirm_src`、临时破中枢段起点、产量回看锚点及 `epsilon` 外推 | **保留并扩充** | 另排除父级 `A` 起点和整个父趋势起点 |
| §4 `D_child := child.a_interval` | **暂时保留，待单独复议** | 仅用于 #43 隔离重放；不构成原文正确性再确认 |
| §4 闭包含、同方向、父子 `cand_delta=true`、`pan_div_diag` 不入链 | **保留** | 闭包含中的父区间替换为 `c_interval_full` |
| §5 `confirm_src`、`lag_conf`、`epsilon_conf` 仅诊断不作闸门 | **完整保留** | 延迟右端使用 `c_end_full` |
| §6 验收表中 `J_parent = parent.interval` 与“D_parent 左端=当前 episode 起点”两行 | **取代** | `J_parent = D_parent = c_interval_full`；左端为 `c_start_full` |
| §6 其余验收行 | **保留或按上表机械替换字段** | 不改变确认时序、方向和事件过滤纪律 |

因此，`dparent-freeze-20260710.md` 仍是 2026-07-10 重放的历史输入；从本裁决生效后，它不再是 `D_parent` 左端的现行定义。

## 6. Q4 可证伪判据

登记预测编号：**P0-43-L1L2-CFULL**。

### 6.1 预测

只改变 `D_parent` 左端映射、保持 `D_child := child.a_interval` 及其余门不变时：

1. 下列三个既有近失对预计仍不通过闭包含：

| 样本 | 旧父窗 | 子级暂定对象 | 旧左界缺口 |
|---:|---|---|---:|
| 1 | `[352003,354036]` | `[340499,341236]` | 11504 bar |
| 2 | `[2180264,2182560]` | `[2160411,2161133]` | 19853 bar |
| 3 | `[2194856,2197213]` | `[2160411,2161133]` | 34445 bar |

2. L1→L2 产出预计仍为 0 或极低；若为非零，每条新增边都必须是暂定子对象在完整父级 `c` 内的真闭包含样本。
3. 不把“产量必须为 0”设为硬闸门；产量不是定义真假的替代物。

### 6.2 何时判预测被证伪

满足以下任一条件，即将 `P0-43-L1L2-CFULL` 标为 **FALSIFIED**，不得事后修改样本或左端定义：

1. 上述三例任一在合法 `c_start_full` 下通过；
2. L1→L2 出现非零边，但结构证据显示其子对象并不完整属于父级 `c_p`；
3. 非零只在把父左端推进到父级 `A` 或整个 `a+A+b+B+c` 起点后出现。

第 1 项证伪的是任务 B 对这三个样本的预测，不自动证伪 Q1；第 2、3 项同时构成重放验收失败，因为它们违反本裁决的对象定义。

## 7. 重放验收清单（本轮不执行）

### A. 重放前定义/字段检查

- [ ] 唯一变量变更为 `D_parent.left: c_episode_start -> c_start_full`；`D_child`、右端、方向、`cand_delta`、确认诊断门均保持不变。
- [ ] 每个父事件同时输出 `c_start_full`、`c_episode_start`、`c_end_full`，不得只改字段名。
- [ ] 每个 `c_start_full` 都附递归结构归属证据：父级最后中枢 `B_p`、完整 `c_p` 的首尾、其内第三类离开结构。
- [ ] 统计并报告 `c_start_full == c_episode_start` 与 `<`、`>` 三类数量；若出现 `c_start_full > c_episode_start`，须先解释对象分解，不能静默接受。
- [ ] 证明 `D_parent` 未扩到父级 `A` 起点或整个父趋势起点。
- [ ] `confirm_src`、`lag_conf`、`epsilon_conf` 仍不参与过滤、排序和否决。

### B. L1→L2 漏斗

- [ ] 保留旧基线：L1 可达 partial chain = 3，L2 `cand_delta=true` = 13，同向可达候选对 = 22；若输入或上游产出变化，先停止横比并报告基线漂移。
- [ ] 输出新旧父左端下每一级的候选数、相邻边数、完整证书数及差分。
- [ ] 对所有新增 L1→L2 边逐条打印：`parent B`、`c_interval_full`、`c_episode_interval`、`child.a_interval`、方向、闭包含两端布尔值。
- [ ] 对每条新增边独立证明子对象属于父级 `c` 内部；“数值落在扩大的区间内”本身不足以替代结构归属证明。

### C. 三个预注册近失样本

- [ ] 按同一父/子事件身份重新定位三例，不得用新的近失排序替换它们。
- [ ] 分别报告新 `c_start_full`、与 `child.a_interval.0` 的有符号差、最终 Sub 左/右界结果。
- [ ] 任一通过时，将 `P0-43-L1L2-CFULL` 标为 `FALSIFIED`，并附 `c_p` 递归分解；不得以“产量恢复”直接判成功。
- [ ] 三例全不通过时，将预测该子项标为 `SUPPORTED-ON-THIS-REPLAY`，不得外推为所有数据、参数或品种的定理。

### D. 最终判定

- [ ] `PASS`：字段能力与本裁决一致，所有接受边均为父 `c` 内真包含，且预测结果被诚实登记（可被支持，也可被证伪）。
- [ ] `FAIL-DEFINITION-MAPPING`：`departure_move_c_start` 的 episode 值仍被无证明地当作 `c_start_full`，或父窗被扩到 `A`/整趋势。
- [ ] `FAIL-EVIDENCE`：出现非零边但没有逐条结构归属证据，或三例事件身份未保持。
- [ ] `BLOCKED-CAPABILITY`：现有字段无法表达完整 `c`；此时只报告冲突，不得用局部 episode、确认点或产量锚点代填。

## 8. 本裁决的边界结论

任务 A 已证明当前 0/0 不是已知实现 bug；任务 B 已证明冻结左端把“完整父级背驰段 `c`”收窄成了“当前离开 episode”。本裁决只纠正这个父对象错配。它不承诺恢复产量，也不借产量决定定义；相反，下一次重放的职责是让数据证伪或支持预注册预测，并把任何非零结果约束为 `c` 内部的真包含证据。
