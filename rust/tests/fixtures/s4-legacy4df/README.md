# S4 旧规则只读历史夹具

本目录为 #1419 融合回归输入。来自冻结提交 4df2d10f92d1075f5fb761a1b112c9c949da5408 的真实写者输出。候选不重写旧规则。

GitHub CI 41044609842 的 legacy-inputs 作业执行 .github/scripts/export-1419-legacy33z.py，在临时旧工作树运行两个 export33z_ 测试。原七代写者与独立全读 oracle 的回复、根表、可达字节、累计数组及 reopen 断言通过后，逐代 VACUUM INTO 导出数据库。source.json 来自同一旧提交的 source_audit 原夹具。原输入顺序为 append1、append2、append3、revision_mid2、insert_mid25、append4、revision_tail4。

cut-N.db 保存每代真实 SQLite 原件。cut-N.json 保存冻结写者回复、完整非节点表和五类累计数组摘要。source.json 保存真实原始账、根声明和节点原件。所有成功证明仅在测试现场重新计算，不从该目录导入。

ADR 0037 第六节将 Sorted layout/1 定为历史只读域。旧七代写锁迁为完整只读历史锁，现役 W 七代仍由真实 writer 对独立全读 oracle 逐项对拍。两者不互相替代。

再生成只允许在指定冻结提交的 CI 临时工作树运行导出脚本，不得用新候选的观测值更新期望。
