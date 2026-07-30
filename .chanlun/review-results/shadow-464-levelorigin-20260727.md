# #464 影子评审：#455 删除 `BspPoint.level_origin`

- 评审对象：`2d1abf9786218432d3971310118dad7a28a122b2`
- 父提交：`f185f921d1197138ab9153b592af95942d832844`
- 规格源：GitHub #455、#434 grilling、影子评审票 #464
- 方法：新上下文、禁自评；两轴独立并行；只读审查，不改代码、不做 Git 写操作
- 总判定：**不通过；⚠ 存在 HIGH。是否回票由编排 session 决定。**

## 执行证据

- 完整检查 `git show 2d1abf9786` / `git diff 2d1abf9786^ 2d1abf9786`：18 文件，`+71/-102`；`git diff --check` 通过。
- 父提交 `rust/src` 精确计数：62 个 `level_origin: 0` 构造字面量；唯一直接字段读取是 `PartialEq`，塔侧 `classifier/mod.rs` 无 stamping/赋值。
- 目标提交：`rust/src` 零命中，`formal/` 零命中；全仓仍有 12 个文件命中，其中 `UBIQUITOUS_LANGUAGE.md` 是活跃冲突，`CONTEXT.md` 是按 D4 留存的删除墓碑，其余为 dated roster 或历史评审/研究档。
- 断言差异：仅 `const GOLDEN` 从 `0xe6a2_63e3_43e4_3845` 改为 `0xe371_3897_d9bf_978c`；旧值已进入历史列表，无第二处断言值变化或断言弱化。
- 独立 `git archive` 快照实跑：目标提交 `cargo build --release` 退出 0；目标及父提交 `cargo test --release --lib` 均退出 0，均为 **2093 passed / 0 failed / 141 ignored**；digest guard 通过。
- GitHub 收尾核对：#109 已留“只订正、不裁去留”评论；#110 已留 AC2 正式撤回评论。

## Standards

**一句话结论：不通过；删除本体和断言充分，但活跃术语文档及“无消费者”措辞违反 090 的声明一致性。**

- **HIGH｜硬违规——活词汇表与实装冲突。** `UBIQUITOUS_LANGUAGE.md:11,44` 仍把 `level_origin` 定义为买卖点现行级别身份，并称“当前实现”仍设置它；与 `CONTEXT.md:31-33` 及字段已删除的实装正面冲突。违反 `AGENTS.md` 090 和 `.claude/rules/no-patch-mentality.md` 的“概念清晰、声明与实际一致、完整解决”。dated `review-results/`、`agent-roster` 属历史档案，可保留；该顶层术语表无退役标记，不能按历史档处理。
- **MED｜硬违规——声明范围不精确。** `CONTEXT.md:33`、`bsp.rs:193-196`、`projection.rs:72-74` 使用未限定的“无消费者”。删除前字段被 `PartialEq` 读取，并经派生 `Debug` 进入 FNV digest；准确口径应为“无下游级别语义消费者，仅有恒真 equality 分量及 Debug/digest 机械消费”。`bsp.rs` 的“同级别结构相等”也无法由已无级别输入的 `eq` 自证，宜写成“级别参照系外置后的结构相等”或明确调用前提。
- **LOW｜Fowler judgement call——Shotgun Surgery。** 一个字段删除迫使 17 个 Rust 文件修改公开结构体字面量，暴露构造耦合；不阻断本票的直接删除。
- 断言强度：固定 GOLDEN 加逐 case 剩余结构字段比较，未弱化；独立 release build/test 已真执行并通过。

Fowler 12 条逐项：① Mysterious Name—不命中，删除误导字段；② Duplicated Code—不命中，无新增重复逻辑；③ Feature Envy—不适用；④ Data Clumps—不命中；⑤ Primitive Obsession—不命中，反而删除裸 `u32` 副本；⑥ Repeated Switches—不适用；⑦ Shotgun Surgery—命中，LOW judgement call；⑧ Divergent Change—不命中，均服务删除；⑨ Speculative Generality—不命中，改动正是清除它；⑩ Message Chains—不适用；⑪ Middle Man—不适用；⑫ Refused Bequest—不适用。

**Standards 计数：HIGH 1 / MED 1 / LOW 1。**

## Spec

**一句话结论：不通过；代码删除及验证本身达标，但活跃术语契约残留构成 HIGH，且“无消费者”事实声明需限缩。**

### (a) 缺失/部分

- **HIGH——活契约未清。** 原文要求“删掉这个字段，让级别身份只有一个来源”，且评审题意加严为全仓清理。目标提交 `rust/src`、`formal/` 均零命中；但 `UBIQUITOUS_LANGUAGE.md:11,44` 仍把它定义为买卖点携带的查询起点及“待修正实现口径”，与代码及 `CONTEXT.md` 正面冲突。其余残留可归为 `CONTEXT.md` 的 D4 删除墓碑、dated roster、历史评审/研究记录。

### (b) 越界

- 无。18 文件改动均属字段、构造/fixture、`PartialEq`、Debug GOLDEN 和契约说明清理；未改 `LevelProjectionLayer` 行为、`source_index`、ADR 或 backtest 行为。#109 订正评论与 #110 AC2 撤回评论均存在。

### (c) 看似实现但错误

- **MED——“无消费者”声明过宽。** 原文称“恒为 0、无消费者”；父提交确有 62 个硬编码 0 的构造字面量，且两条塔终装路径无赋值点，但字段仍被 `PartialEq` 直接读取，并经派生 `Debug` 被 digest guard 观察。准确表述应是“无下游级别语义消费者”；D2/D3 本身也承认这两个消费/可观测面。
- 删除代码面完整：目标提交所有构造/fixture、比较分量已清；无序列化键残留；`signal.rs` 注释块无该字面量。断言值仅 GOLDEN 改动，旧值入历史，digest guard 实跑通过。
- 独立快照实跑：目标 `cargo build --release` 退出 0；目标及父提交 `cargo test --release --lib` 均为 **2093 passed / 0 failed / 141 ignored**。

**Spec 计数：HIGH 1 / MED 1 / LOW 0。**

## 分轴汇总

- Standards：**HIGH 1 / MED 1 / LOW 1**
- Spec：**HIGH 1 / MED 1 / LOW 0**
- 分轴机械合计（不跨轴去重、不重排）：**HIGH 2 / MED 2 / LOW 1**

