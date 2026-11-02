# #1491 登记草稿（S5 0075 起草，未发布；外部写入交主控决定）

核实（2026-10-21，`gh issue view 1491 --json title,body,comments,state`）：#1491 OPEN、0 条评论；
正文只在 CC-007 段写「现有单元与序列 memo 原样复用」，**没有登记 S86 运行库里的任何不可达残留族**，
包括 D5 指定转 #1491 的 coords memo。下面是建议追加的评论正文。

---

## #1488 S5 实测：终面 S86 不可达节点残留（转本票登记）

终面 3ee05a2713 S86 生产构建（两跑相同）：存储 560,069 节点，可达 344,419，不可达 215,650（38.50%）。
main 6badaa6049 r8a 为 784,774 / 440,355（56.11%）。差值 224,705 正好是 #1488 修掉的 fractal per-bar append
作废中间根（70,051 个裸序列顶点及其下全部节点），终面这一族为 0。testonly native 构建少 32 个 memo 节点
（560,037 / 215,618，native 路径不产生这 16 数组 + 16 序列），其余相同。剩余不可达节点逐个属于下列族，
各族计数在 main r8a 与终面生产构建完全相同（主线既有，非 #1488 引入）：

| 族 | 顶点 | 节点 | 结构特征 / 推断的发射处 | 建议归属 |
|---|---:|---:|---|---|
| coords 数组 memo 子数组 | 1,714（native 1,698） | 2,911（native 2,879） | 无父数组，元素序列是某可达数组的连续子段；`content_producer::array_from_node`（coords_array_memo） | #1491（D5 已裁） |
| typed record 重盖章（generated） | 13,336 | 90,729 | 对象当前行同 revision、不同 published_generation 的 generated record；生产者重算既有对象时以当代 generation 重写 record（content_producer.rs:1047 / CC-006 :2335），发布保留旧行 | 待主控：#1491 或另票 |
| typed record 重盖章（internal） | 13,336 | 90,552 | 同上的 internal record（batch_id/lifecycle） | 同上 |
| 同代快照中间版本 | 1,285 | 15,124 | 与该代 snapshot_root 同形（generation 相同，history_mode=RecomputedWithRevision），每代 9–87 个，随该代对象变动数增长 | 同上 |
| 同代增量中间版本 | 1,285 | 13,410 | 与该代 delta_root 同形（base_cut→next_cut 相同） | 同上 |
| 见证 id 配方对象 | 501 | 1,678 | `{mi, object_id, slot, raw_bars}`，`aux_index::verify_witness_id` 为求 `wit-` 哈希写入存储 | 同上 |
| 每代 generated 体 | 86 | 1,032 | 每代一个，含 scope/rule_revision/raw_events/effective_events | 同上 |
| 其余（mixed 200 + 7 个非子段数组 14） | 7 | 214 | 被多族共享的子节点；末代 7 个数组无可达宿主 | 随所属族 |

读数与脚本：`.chanlun/review-results/1488-acceptance/s5/`（unreach_s5.py、unreach-*.json）。
这些族都不是 #1489 两个布局来源（中段插入、CC-007 整对象修订）的直接产物，#1491 的上界门只计**可达**新增
节点（Δ_R），所以不会拦住它们；9011 规模下它们会照比例占存储（S86 已占全库 38%）。是否在 #1491 内一并处理，
或由主控另开票，请裁定。
