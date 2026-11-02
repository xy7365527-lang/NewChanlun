# #1790 独立复审 r2

**结论：PASS**

r2 已修复此前的 R8 与 R1 问题：

- `archaeology.json` 为 `parsed=true`，含 8 个 points，schema 与 `decisions/1786/archaeology.json` 一致；`archaeology.md` 已补齐。
- 从 archaeology.json 的 `command` 字段直接抽查并复跑：Q1 `group_sources_coords` 命中 12/12；Q3 object_index 命中 7/7；Q4 ADR 先例命中 9/9（预留号命令 3/3）。
- E3 的 RawRefSpec、Leaves、bind_source 版本换根，E5a–E5e 的严格递增有效域与 seed/establish/new_group/merge 路径，未封口边界和 bounded 不含 `subsequent_development`，均可回查 PR #1780 head `a485194947`。
- `CONTEXT.md:176` 已列入受影响清单，载体选择符合 ADR 0005/0022/0026/0032/0015 修订段先例。

未发现新的阻断项。
