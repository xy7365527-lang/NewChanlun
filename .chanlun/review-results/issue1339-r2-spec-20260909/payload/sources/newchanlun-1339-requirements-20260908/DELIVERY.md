# #1339 第一阶段交付：需求与验收范围

需求草案已完成独立文档复核，**用户已确认范围并授权进入完整架构重评**。本交付不表示程序已实现，也不是#1323整图完成。

范围确认后，用户进一步强调 **“必须完全分类”**。对应[硬要求](/tmp/newchanlun-1339-architecture-20260908/COMPLETE-CLASSIFICATION-REQUIREMENT.md)已进入第二阶段：补完整叶分支、合法组合和转换的目录，再据此重评架构。第一阶段80行是聚合需求目录，不能当作分类完备性的证明；本轮正在继续这一工作。

从[需求入口](/tmp/newchanlun-1339-requirements-20260908/author/REQUIREMENTS.md)阅读六部分范围，再按需展开[80项贯通表](/tmp/newchanlun-1339-requirements-20260908/author/COVERAGE.md)、[20项观察合同](/tmp/newchanlun-1339-requirements-20260908/author/OBSERVATION-CONTRACT.md)和[15类验收](/tmp/newchanlun-1339-requirements-20260908/author/ACCEPTANCE-PLAN.md)。条目数不是程序通过数。

| 内容 | 当前复核依据 | 结论射程 |
|---|---|---|
| 44项结构需求 | [结构独评](/tmp/newchanlun-1339-requirements-20260908/review/REVIEW-STRUCTURE.md) | 当前两件STRUCTURE文件，初审6M/3L均闭合 |
| 35项完整多重赋格需求 | [赋格独评](/tmp/newchanlun-1339-requirements-20260908/review/REVIEW-FG.md) | 当前两件FUGUE文件，初审3M均闭合 |
| 14组已有证据导航 | [观察与证据专项独评](/tmp/newchanlun-1339-requirements-20260908/review/REVIEW-OBS.md) | 当前两件EVIDENCE-REUSE仍匹配原签署；旧OB签署不覆盖现稿 |
| 需求入口、80行贯通、RF-01及当前观察/验收 | [汇合独评](/tmp/newchanlun-1339-requirements-20260908/review/REVIEW-INTEGRATION.md) | 当前9件，初审2M已闭合，0H/0M/0L |
| 编号、关联、复制字段及文件绑定 | [最终文档机械检查](/tmp/newchanlun-1339-requirements-20260908/evidence/FINAL-DOCUMENT-CHECK.json) | 文档机械一致性；不代替语义或程序验证 |

用户的范围确认单独保存在[确认记录](/tmp/newchanlun-1339-requirements-20260908/SCOPE-CONFIRMATION.md)，不改已签文件的历史状态句。机器清单见[DELIVERY.json](/tmp/newchanlun-1339-requirements-20260908/DELIVERY.json)，各复核仅对其明列SHA生效。

当前没有项目、Lean、UI、回放、恢复、paper/live、性能或经济指标的执行验收。既有三链和全链报告仍只按原静态射程复用，没有把本轮文档复核算成当前源码全量复验。

本轮输出在专属外置草稿目录；仓内代码、封存原稿和GitHub票面未改。AUTHORIZATION与LANGUAGE-ALIGNMENT是补充说明，不冒称属于9件独评；history、生成脚本和已排除K4算术稿不属于当前交付。后续继续#1339候选重评与用户架构选择，再依整图流程进入SPEC及实施。
