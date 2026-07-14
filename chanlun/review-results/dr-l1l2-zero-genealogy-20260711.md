# L1->L2 归零定义谱系审查（2026-07-11）

- 任务：深研任务 B，定义谱系审查。
- 范围：只读审查为主；本文件为新增报告。
- 证据边界：当前 worktree 缺 `chanlun/review-results/nest-lag-deep-research-20260709.md`，但同机 `/Users/silencehan/Projects/NewChanlun/chanlun/review-results/nest-lag-deep-research-20260709.md` 与 `/Users/silencehan/Projects/NewChanLun/...` 均存在，SHA-256 均为冻结文档登记的 `43b56ab127d40639c8c9c740d90a9de6608c20a898852e0e8063c05234677b36`，故按冻结权威副本读取。

## 0. 结论

1. `J_parent := D_parent` 这一裁决语义没有错：父级对象必须是父级背驰段，不是父级确认窗，也不是确认时刻。
2. 但冻结落地把 `D_parent` 左端钉成“当前 C 离开 episode 首同向 Segment 起点”，并把 `D_child/I(A_child)` 钉成 `child.a_interval`。这两处至少存在一个符号对象错配：原文区间套嵌套的是“低级别背驰段/背驰定位区间”落在“高级别背驰段”内部，不是把低级别比较用的 A 腿单独塞进高级别的 C 离开 episode。
3. 原文不支持把父级区间左端扩到父级 A 段起点或整个 `a+A+b+B+c` 起点。父级区间应从父级最后背驰段 `c` 的结构起点开始；更精确说，是包含最后一个中枢 B 的第三类离开并最终产生该级别转折的 `c` 走势类型起点，而不是该 `c` 内部某次重新离开 episode 的局部起点。
4. L1->L2 的最近三例中，子级 `I(A)` 早于父级冻结左端 11504/19853/34445 bar，且子级确认也早于父级 D 起点。这不是“确认滞后”的指纹，而是“拿了父级背驰段之前的子级证据来套父级背驰段”的指纹。若强行把父级扩到 A 段起点，可能恢复产量，但会背离第 27/29/37 课的区间套对象。
5. 预测：按本文推荐定义重放，L1->L2 不应因“父级改到 A 起点/整段趋势”而恢复；若只把父级从局部 episode 放宽到完整 `c` 走势类型，当前这组三个近失样本仍不会通过。L1->L2 产出大概率仍为 0 或极低；若非零，应来自 `c` 内部完整背驰段的真包含样本，而不是这些早于父级 `c` 的样本。

## 1. 原文对象：区间套套的是背驰段，不是确认窗

第 27 课直接定义背驰段：“某级别的某类型走势，如果构成背驰或盘整背驰，就把这段走势类型称为某级别的背驰段”（`docs/chanlun/text/blog/027-第27课.md:22`）。同课又说，区间套是“从大级别往下”找大级别买点（`:36`），并用“不同级别背驰段的逐级收缩范围”定位转折（`:42-46`）。这里的父对象是走势区间，不是确认事件。

第 24 课给出 A/B/C 语境：A、B、C 在一个大趋势里，C 完成且力度弱于 A 时构成标准背驰（`docs/chanlun/text/blog/024-第24课.md:22-24`）。这说明背驰判定需要 A 与 C 的比较，但不等于父级区间套的父区间要从 A 起算。

第 29 课更关键：对 5 分钟趋势背驰，原文称“最后的背驰段，跌破该中枢后……”（`docs/chanlun/text/blog/029-第29课.md:30`）。这把父级背驰段定位到最后一个中枢之后的最后下跌/上涨段，即 `c`，不是整个趋势。

第 37 课把这个关系递归化：`c` 至少包含对 B 的第三类买卖点（`:18`），且“对 c 的内部进行分析”，在 c 内部继续套用 `a+A+b+B+c`，形成类似区间套状态（`docs/chanlun/text/blog/037-第37课.md:22`）。所以递归入口是父级 `c` 内部，而不是父级 A 或整段父趋势。

## 2. 对冻结左端的审查

冻结文件定义：

- `D_parent(p) := [p.enter_src, p.interval.1] = p.interval`。
- `p.enter_src` 是“当前 C 离开 episode 中第一个与破中枢方向同向的 Segment 的 start_index”。
- `D_child(c) := I(A_child) := c.a_interval`。

见 `chanlun/review-results/dparent-freeze-20260710.md:29-41` 与 `:45-61`。

这份冻结把两个不同层次压在一起：

1. 父级 `D_parent`：原文应是父级最后背驰段 `c` 的区间。
2. 子级 `D_child`：原文应是低一级“相应背驰段/背驰定位区间”的区间。

若 `departure_move_c_start(...)` 返回的是父级完整 `c` 走势类型的起点，则父级左端可接受；若它返回的是 `c` 内部“失败离开 -> 回中枢 -> 重新离开”之后的局部 episode 起点，则它比原文 `c` 起点偏右。代码注释显示当前函数明确是“当前 episode 首同向段起点”，并按 reentry 定界（`rust/src/theta_v0/classifier/divergence.rs:738-755`），因此它不是无条件等价于第 37 课的完整 `c`。

但这不推出父级应扩到 A 段起点。第 37 课说的是在父级 `c` 内部继续套用下一级 `a+A+b+B+c`；父级 A 是比较参照，不是区间套父窗。

## 3. L1->L2 左界失败的含义

复跑漏斗记录首个归零在 L1->L2：L1 有 3 个可达 partial chain，L2 有 13 个 `cand_delta=true`，但 `I(A_child) subset D_parent` 成功边为 0（`chanlun/review-results/dparent-funnel-20260710.md:39`）。

三个最近样本全部是 Sub 左界失败：

| 样本 | 父级 D_parent | 子级 I(A) | 左界缺口 | 子级 confirm |
|---:|---|---|---:|---:|
| 1 | `[352003,354036]` | `[340499,341236]` | 11504 | 345518 |
| 2 | `[2180264,2182560]` | `[2160411,2161133]` | 19853 | 2168349 |
| 3 | `[2194856,2197213]` | `[2160411,2161133]` | 34445 | 2168349 |

来源：`chanlun/review-results/dparent-funnel-20260710.md:45-49`。

这组数据不是 7 月 9 日深研讨论的“子级确认晚于父窗右缘 60/87/183 bar”问题。这里子级 `I(A)` 与子级确认本身都在父级 D 左端之前很远。若把父级区间扩到足以包含这些 A 段，得到的是“父级 `c` 之前的低级别背驰证据也算父级转折定位”，这与第 37 课“对 c 的内部进行分析”相反。

因此，子级 `I(A)` 早于父级离开段起点，不能直接证明父级应取父级 A 段起点。它更直接说明两种可能：

1. `D_child := child.a_interval` 把“子级背驰段”误落成了“子级比较 A 腿”；
2. `D_parent := 当前离开 episode` 比父级完整 `c` 偏右，但只应修到完整 `c` 起点，不应修到父级 A 起点。

## 4. 对照 7 月 9 日深研与 7 月 10 日裁决

7 月 9 日深研的核心结论是：区间套解决的是高级别背驰确认滞后；嵌套对象是“背驰段区间”的包含，不是确认时刻先后；推荐 `J_parent := 父级背驰段 D_parent`，`lambda_C` 不参与否决（权威副本 `nest-lag-deep-research-20260709.md:12-16`, `:115-123`）。

7 月 10 日裁决登记：

- `J_parent := 父级背驰段 D_parent（进入背驰段起点 -> 该级别背驰对应转折端点）`；
- `I(A) ⊆ D_parent`；
- `lambda_C` 仅登记延迟；
- `D_parent 左端判定与 D_child 字段映射必须在重放前冻结`。

见 `chanlun/escalate/p0-jparent-dparent-ruling-20260710.md:9-13`。

判断：

1. 裁决的 `J_parent := D_parent` 是正确方向，推翻了旧的父级确认窗。
2. 裁决没有充分消歧 `I(A)`：如果 `I(A)` 指“子级背驰定位区间/背驰段证据区间”，可与原文兼容；如果落地为代码里的 `child.a_interval`，即“与子级 C 配对的前一中枢同向离开 episode”，则它不是第 27 课所说的低级别背驰段本身。
3. 所以错误主要在冻结口径的落地与符号消歧，不在 `J_parent := D_parent` 的裁决方向。更严格地说：`D_parent` 的左端落地可能偏右；`D_child` 的字段映射更明显需要复议。

## 5. 应采用的可机器检定义

### 5.1 父级区间

对任一父级趋势背驰事件 `p`：

```text
D_parent(p) = [ start(c_p), end(c_p) ]
```

其中：

- `c_p` 是父级最后一个同级别中枢 `B_p` 之后、最终产生该级别背驰转折的最后走势类型；
- `start(c_p)` 是该 `c_p` 的结构起点：包含对 `B_p` 的第三类离开并进入最终背驰段的首个次级别走势/Segment 起点；
- `end(c_p)` 是该级别背驰对应转折段的结构端点；
- 排除：父级确认窗、`confirm_src`、父级 A 段起点、整个 `a+A+b+B+c` 起点、以及 `c_p` 内部某个未证明等价的局部 reentry episode 起点。

可机器检查：

```text
parent.has_trend_beichi == true
parent.c_start == start(c_p)
parent.c_end == end(c_p)
D_parent = [parent.c_start, parent.c_end]
```

若现有字段只有 `enter_src`，必须新增或重算 `c_start_full`，并断言：

```text
enter_src == c_start_full
```

若断言不成立，`enter_src` 只能作为 `I(C_episode)` 起点，不能作为 `D_parent` 左端。

### 5.2 子级包含对象

原文最小闭包应检验低级别背驰证据是否发生在父级 `c_p` 内部。建议冻结为：

```text
D_child(c) = span_ac(c) = [ start(A_child), end(C_child) ]
Sub(child, parent) := D_child(c) ⊆ D_parent(p)
```

如果后续裁决坚持只套“低级别背驰段 C_child”，则应写成：

```text
D_child(c) = [ start(C_child), end(C_child) ]
```

但不能继续把 `D_child` 命名为“子级背驰段”同时落到 `child.a_interval`，除非另有正式裁决证明 `child.a_interval` 在本项目符号中不是比较 A 腿，而是完整子级背驰段。

## 6. 对产量的预测

按本文定义，L1->L2 不应因为把父级左端扩大到 A 段或整个趋势而恢复；这种恢复属于对象误纳。

若只从“当前离开 episode”修到“完整父级 `c` 走势类型”，理论上可能恢复少量真样本，但当前漏斗给出的三个最近样本中，子级 `I(A)` 与确认都早于父级 D 起点数千到数万 bar，不能作为会恢复的证据。基于现有样本，预测 L1->L2 仍为 0 或极低；若出现非零，必须逐条证明子级完整 `span(A,C)` 位于父级 `c` 内部，而不是只因父级左端被扩到 A 段才通过。

## 7. 后续复议建议

1. 对 `I(A)` 的裁决符号补一次定义广播：它到底是子级背驰段、子级 C 段、还是比较用 A 腿。
2. 在 `CandDeltaEvent` 中分离字段：`c_interval_full`、`c_episode_interval`、`a_interval`、`ac_evidence_interval`，禁止继续用 `interval` 同时承载父级背驰段与局部 C episode。
3. 复跑时输出四列左界对比：`child.a_interval.0`、`child.interval.0`、`child.ac_evidence_interval.0`、`parent.c_start_full`。若非零边只依赖 `parent.a_start`，应判为原文不支持。
