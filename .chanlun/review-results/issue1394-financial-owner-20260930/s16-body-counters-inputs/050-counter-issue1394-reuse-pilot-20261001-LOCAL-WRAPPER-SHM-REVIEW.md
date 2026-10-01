# #1394 LOCAL wrapper 的 owned SHM mtime 增量复审

结论：**PASS / Approve（静态恢复与运行门）**。审查对象 `LOCAL-audit.py` SHA256 `1222ffcd766d91a599ad2c7ecc199d57cb8f3f2edd72d265cc95d16f7f34009c`。旧 `LOCAL-WRAPPER-REVIEW.md` FAIL 与 `LOCAL-WRAPPER-FINAL-REVIEW.md` 对 3dc51d... 的 PASS 均不覆盖。本结论不代表同材料审计已运行或收益已证实。

上次失败证据 `LOCAL-output/LOCAL-STATIC-FAILURE.json` SHA256 `6768816525aa033ed6b159af95aa39002ec72c33007273dddae34744c7eaf9d1` 记载：尚无 audit child；将该记录中相对与绝对路径指向的同一 clone 文件对齐后，baseline owned clone 仅 `s.sqlite-shm` mtime 改变，DB/WAL/SHM 的 SHA、字节数、设备和 inode 均未变。当前 `LOCAL-output` 有成功的 `BUILD.json` 与旧 `REBOUND.json`，没有 baseline/candidate PLAN 或 PROBE；已构 testbin SHA `5f9ef56e7cd0ee27dd00094faaf5c73821d6501fe38b4783c238c67d80cb286f` 取自 BUILD 记录，**本次未重哈希该二进制**。

审查结果：

- `owned_source_equal()`（第 91–100 行）仅在三件文件的路径、SHA、字节、设备、inode 逐件相同时放宽 `s.sqlite-shm` 的 mtime；DB/WAL mtime 仍逐件严格。纯内存反例证实只有 SHM mtime 漂移被接受，DB/WAL mtime 与 SHM SHA、字节、设备、inode 漂移均被拒绝。
- `rebind-after-build`（第 244–261 行）要求旧 REBOUND 和 BUILD、要求尚无运行 PLAN/PROBE，先走未改的 `original_gate()`，再验证两份 owned clone 与已构二进制，写 `REBOUND-2.json` 绑定旧 REBOUND、PREPARE、BUILD、新 wrapper、source binding 和 diagnostic patch。`bound_gate()` 对 REBOUND-2 的这些哈希链重验。旧 BUILD 不被重跑或覆盖。
- `run_one()`（第 396–466 行）分别记录 `source_before`、`source_after_static`、`source_after`；静态 DB 读取和 audit child 后均检查 owned clone 严格字段。`report_evidence()`（第 537–605 行）再次对拍这些阶段记录、当前 source、完整 DB、plan、原始日志、PROBE 与 fresh comparison；报告 JSON 还记录 prepared/before_static/after_static/after_audit/at_report 的 SHM mtime。原 A source 和 589 个原 target 仍由 `original_gate()` 使用 SHA/inode/mtime 严格校验，不使用 SHM 例外。
- 优化模式的非 assert 入口拒绝（第 22–23 行）、严格计数类型门、PASS JSON 最后原子发布仍在，未见本次修订使旧门退化。

放行顺序：由 Root 决定时机后运行 **`rebind-after-build → run → report`**；不要重跑 build。每一步核对对应成功产物，最终仍需独立检查实际计数、状态对拍、F 降幅和 RSS 采样声明。本次仅做源码、现存小型 JSON 与纯内存比较器检查；未运行 rebind、build、真实 audit 或 report，也未重新哈希 589 target、原 A 或 fullDB。
