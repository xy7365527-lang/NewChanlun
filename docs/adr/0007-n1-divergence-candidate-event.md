# ADR-0007：N1 载体——背驰段候选事件一等塔对象（形态 / 身份键 / 产出关系）

map #529 N1 缺口：背驰段区间（`seg_c`/`interval_b`）只活在默认关闭的 C2 seam 与 sidecar 驱动的 `CandDeltaEvent`，`classify_impl` 主路径不产生、不保存。#540（grilling，编排者 2026-07-28 裁定）三问一次裁定；本 ADR 是文书落库。前置 = ADR-0006（N0：原生 Cand = 结构宽候选，Sub = C⊆C）与 #535（因果形态封口）。

## 裁定文本

1. **形态 = 新塔对象**：一等「背驰段候选事件」，承载 E2E-D1-D3 结构载荷（kind/event_level/center_ids/pair_id/structural_predicates/extreme_proof）+ 背驰段区间（C 段，`interval_b` 同口径）+ 状态与钟（E2E-D5）。`CandDeltaEvent` 的字段集（`a_interval`/`c_episode_interval`/`c_interval_full`/`b_parent`/`c_structure`）为其零件库，但对象不原样提升。
2. **身份键 = 结构身份键**（`ConfirmKey` 同构口径，#69 5a 既有设计）：键材料 = level + 中枢对身份（B_p 指纹 + 前中枢）+ seg_a + c_start；**c 段右端与 as_of 不入键**——键管身份、修订管生长。与 #206 锚（极值价, 合并组锚）分层：点身份管 BSP↔证书同一拐点（N4 材料），段结构身份管本事件。
3. **产出关系 = `classify_impl` 主路径内联产出 + 每级 append-only 候选事件流 + E2E-O 修订协议**：每级 decompose 完成即产出；修订只追加、旧 revision 保留（禁删除模拟失效）；`observed_at`/`first_provable_at` 一次写入不后移；`Confirmed`/`Invalidated` 终态不复活；同 `as_of` 重跑强制零 Delta；全量/增量两路径产出逐字节一致（套 `classify_with_tower_incremental ≡ 全量` 既有锁，E2E-S1 门验收）。

## Considered Options

- **(b) 扩 `LeveledMove`/`CpStructureIdentity`**：被否——候选是事件（结构合格时出现），非每个走势单元的固有属性，扩字段 = 每个 move 背几乎恒 `None` 的字段，本体混淆；`CpStructureIdentity` 装的是 c_p 完整走势身份，非背驰段区间，两物。
- **(c) 原样提升 `CandDeltaEvent`**：被否——旧力度确认语义（ADR-0006 已排出原生定义）+ sidecar 产 + 兼容字段缠身（`interval`/`confirm_src` 别名）；提升 = 把被否语义焊进原生载体。
- **纯查询视图**（候选当纯函数现算，不落账）：被否——E2E-D5 的钟要求状态（首见钟无处放）；且 N1 定义即「主路径产生**并保存**」，视图 = 仍无载体。

## Consequences

- **N1 实装前置裁定全齐**（N0 语义 / 载体 / 键 / 产出形态 / 因果红线 / Sub 口径），可进 /to-spec；实装走 claude CLI Opus 5（2026-07-26 令），配影子评审票。
- **主要工作量在案**：`classify_impl` 纯函数新增输出通道（`Classification` shape 变更）+ 全量/增量对拍锁扩展。
- **N2 无需再裁**：谓词语义已被 ADR-0006 + #246 钉死（C⊆C 闭区间包含、相切算包含），出 task 票随 N1 实装包规格化；N2 谓词落点 = 本事件的区间字段。
- **N4/N5 材料备**：N4 身份边 = 段结构键（本裁）+ #206 点锚分层；N5 首证钟 = 本事件的 `first_provable_at` 字段（塔对象内，不再登记在 bin）。
- **零生产行为变更**：本裁定为对象建设，不接消费点（N7 另票另裁）；nest 管线三件原样（ADR-0005）。
- **程序性**：本 ADR = #540 三裁定文书；实装中的字段级命名/复用 `ConfirmKey` 与否归实装票，不属本裁。
