# #1340 完整 SPEC 发布与实施分解

已批准的 R2/S2-R1 SPEC 已发布为 #1340，11 个 TB 验收汇总下挂 77 张内部实施票；17 张具名残留任务直接挂 #1323。该包保存计划、各次独立复核、原生依赖及旧票承接的发布收据，是绑定开放票的工作草稿。尚未合入 main，发布与复核均不代表程序运行验收。

- [已批准规范及批准记录](../issue1339-r2-spec-20260909/README.md)。
- [TB01 三片](view/inputs/TB01-EXECUTION-SLICES.md)与[完整父义务贡献账](view/inputs/TB01-PARENT-CONTRIBUTION.md)。
- [TB02–06 五十四片](view/inputs/TB02-06-EXECUTION-SLICES.md)及[最终差异独评](view/inputs/TB02-06-FINAL-SLICE-REVIEW.md)。
- [TB07–11 二十片](view/inputs/TB07-11-EXECUTION-SLICES.md)及[父票与分解复核](view/inputs/ROOT-SLICES-PUBLICATION-REVIEW.md)。
- [全部叶票发布回读](view/inputs/FULL-LEAF-PUBLISHED-VERIFY.md)、[第一阶段回读](view/inputs/PUBLISHED-GRAPH-VERIFY.md)和[真实票号映射](payload/evidence/EXECUTION-SLICES-PUBLISHED.json)。
- [17 张残留任务](payload/evidence/RESIDUAL-PUBLISHED.json)、[原残留计划](view/inputs/RESIDUAL-TICKET-PLAN.md)、[完整历史模型补充](view/inputs/RD-17-MODEL-PLAN.md)及[发布独评](view/inputs/RESIDUAL-PUBLICATION-REVIEW.md)。
- [旧票未销义务及去向](view/inputs/LEGACY-TICKET-DISPOSITION.md)、[当前主图正文](view/tickets/1323-UPDATED.md)和[图更新复核](view/inputs/MAP-UPDATE-REVIEW.md)。
- [归档独立复核](ARCHIVE-REVIEW.md)。
- [发布静态验证](payload/evidence/EXECUTION-SLICES-VALIDATION.json)与[逐文件原路径和 SHA](ARCHIVE-INDEX.json)。

`payload/` 保留各输入原字节；其中当时的待审/未发布状态及本地路径保留为历史来源，不能据此否认后续批准或假称当前运行完成。`view/` 仅转换阅读链接。JSON 内路径通过 ARCHIVE-INDEX 与上一规范包的同名索引解析；未归档引用明确单列，不能替代其原件。

原六 Destination、97 故事、345 条目和 496 验收点保持完整量词；每叶只承担具名子义务。跨 TB 支撑引用和共同销项证据不自动新增调度边。TB01 的局部分类实例不代表全部 CC 已实现或完整经营已验证。

旧 #1327–#1333 继续暂停；#1320 的七标的十年分钟数据及冻结 7/7 重型测量、#1304 的报告器/回归锁、关联票自身义务和 #1338 单独归档门均继续保留。原生旧边未被提前移除。

首票 #1370 的定向实施从精确归档基线开始，实施与独立评审另以真实提交和检查报告验收；本包不保存会话/工具日志，不变更常驻 Prime 服务。main、生产切换、真实外效与未定语义/资金实例仍按具名批准门处理。
