# PR1843 集成差异独审

结论：源审 PASS；新 head 必要 CI 仍须通过。

作者 Opus，非作者主控读取 Git 实际对象核验，不依据备份文件代替候选。

候选 55d012bf223ef2cff70abef648996b464966d433；两个父为 b6dd6bfb07ac185b3af4b56c576b41cb28e479d5 和 876fab8fbf2ea5f20611f83690b6d01dcd614e1a。

通过 Python 结构断言核 base/ours/main/result：112/117/115/120 条。四份 schema/issue 元字段一致，路径无重复，base 每条在两侧完整不变。新结果逐条恰为两侧并集，顺序为 main 原序加 PR 独有五条；没有修改、丢失或额外条目。

五条 PR 独有项均为1818的layout审查原件；三条main独有项为1539的archaeology、review40y、review41k。原1818归档与ADR0059在旧候选到新head间零差异，复用45I/45Q原审。

不重复文件摘要计算；CI仍负责真实SARIF与登记绑定的动态检查。仅在新head必要CI全绿、合前再核当前集成后可merge；不自动关闭1818。
