# #1372 Q 编码 memo 增量独立审阅

结论：**PASS_BOUNDED_INCREMENT**，未发现本增量产品阻断。Q 固定 `18ee720a02e4c4bb4fd8447ab99e6ef518957e14`；追加 CI 候选 `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`。本票整体仍 **NOT_VERIFIED**，不以副本检查代替新 AC3 / R5 实际负载。

评审身份为原生 Codex `/root/r7_codex_product_review`，未实现 Q 或 CI；本轮不自评 Rust。只写本目录，没有产品、Git、GitHub、活库或服务动作。读取图元数据没有本候选工作树版本，故采用固定文件与 Git 对象逐字核对，未以主仓图代替候选。边界来自指令，并非 OS 沙盒。

| 核点 | 结论与精确依据 |
|---|---|
| 原摘要全字段/类型/顺序 | `s_query_integrity.py:243–270` 保留模式、按表/行顺序与逐值类型长度标记及所有 BLOB 原字节；仅已核 `delta_json` 的 `canonical(str)` 从压缩原编码恢复。独立重算完整 15 表摘要一致，160 个完整字符串编码及公开 Delta 均逐字恢复。`RESULT.json#/checks/0–1`。 |
| 来源与失效 | `:738–841` 仍逐值比较全部 Delta 列、原 UTF-8 与 batch 完整字节，首次差异后重建后缀；完整 raw/索引/control/epoch 门位于摘要前。实际同代 SQLite 附注提交改变 data_version 和摘要、160 hit；改第159代空白/转义/中文/控制字符与具名扩展后 158 hit、2 miss，完整摘要仍同。`:901–902`、`RESULT.json#/checks/3–4`。 |
| 连续代索引 | `:715–717,800–803` 先核总行数与 `1..G` 连续代，才使用 `generation-1`。159改161而总数不变亦拒绝。`FOLLOWUP-RESULT.json#/checks/1`。 |
| 预算 | `:836–841` 的新增值为 ordinary bytes，包含于 `proof._audit_memo`；`s_query.py:139–153` 原联合计量与删除 memo 路径未改。full **40,771,243 B**、去memo **13,231,048 B**；隔离阈值 **27,001,145 B/worker** 下清掉memo，完整160Delta与最终全字段投影保持。原配置 **134,217,728 B/worker** 未改。这些数值不是RSS上限。`FOLLOWUP-RESULT.json#/checks/0`。 |
| 坏源新鲜拒绝 | 已暖缓存后独立SQLite提交159重复JSON键，拒绝并清cached/memo、释放事务。`FOLLOWUP-RESULT.json#/checks/2`。 |
| 公开兼容 | 5个cut × state/cursor/两种完整projection共20值有无memo一致；所有160公开Delta对SQL原JSON一致。作者816记录的标签全集、逐值canonical比较脚本及所有equal回执已核；没有把作者执行冒称本代理重跑。`RESULT.json#/checks/1–2`、`STATIC-AUDIT.json`。 |

生产 integrity 文件 SHA256 `d69fda491cbb52ead1a714bf942d5c8229ddd9d9f83dd4a17910515d101780bc`；新测试 SHA256 `74d4e9b8d5d8516b8dd2903e291dfb7f769c397f7e6300855ff0e077bc4a82e4`。`s_query.py` SHA256 `ce51b7a72de1bf1ab869da9e1498bc8ef95e3458981b847aaab033453268ad65` 同字节。AST 差异仅 `capture_digest`、`verify`；完整捕获、模式/typed/raw/control、sealed校验与投影函数未变。详细路径/版本/行号和完整 SHA 见 `REVIEW.json`、`INPUTS.json`。

首轮探针 `RESULT.json` 的 **FAIL 原样保留**：它把另一个暖 proof 的对象大小减1作为冷 proof 的排除阈值。诊断复现暖 **40,792,287 B**、冷 **40,791,664 B**、阈值 **40,792,286 B**；冷memo实际合法装得下，摘要完全相同。不是产品偷偷突破预算。后续改为明确位于 bare/full 区间的阈值完成原目的；记录在 `BUDGET-ORACLE-DIAGNOSIS.json` 与 `FOLLOWUP-RESULT.json`，没有回写失败、扩资源或改产品。

作者31项Q测试及v2真实二进制集成exit0已读回执与日志。816报告 SHA256 `002e42d7190a88c37e8eb0b208ba7f8e5fde4769db69824e24dce700934bd941`。其中 data_version **2→3** 是同代 meta 附注 SQLite Commit，不是新的 S 结构代；159坏Delta拒绝也来自隔离副本。当前限域足以审本优化，不能将报告中的模型耗时与113页用时当成AC3已过。

CI 小增量亦 **PASS_BOUNDED_SOURCE_WIRING**。`.github/workflows/ci.yml:263–278` 新增独立 `s-observer`：Python3.11 discover收 **11+7+24+9=51** 项，Node22执行 **32** 项浏览器协议替身测试。删除这16行后与旧CI逐字一致，原Rust/pytest/fixture门保留；旧pytest `testpaths=["tests"]` 本来不收 `s_session/tests`。根同入口本地51+32回执exit0已核；未独立运行远端CI。**31项Q及v2原生/HTTP集成仍未接该job**，继续作为具名本地验证；Node不是Chrome或完整C验收。`CI-REVIEW.json`、`CI.diff`。

本目录三份隔离DB的最终main/WAL/SHM全数保全；原冻结130,269,184 B库前后SHA仍为 `a30aaa8d62b4658f9891ce89b082ca6d9030928be631a5c9fa6f013c321de74a`。main文件同SHA不表示WAL变异未发生，完整状态须组合保全。没有读取正在运行的最终AC3/R5库；其封存证据另审。

追加测试入口尾项 `932de089e989ca9ce482a93d6db825c4d4fda028`：**PASS_BOUNDED_TEST_ENTRY_FIX**。仅 `r9_contract.py` 38增12删，显式 TestOnly Q预算/epoch1与可选受测binary；两业务函数 `exercise` / `exercise_r10` AST逐字未变。Popen自有子进程 TERM→10s→必要KILL→10s，启动失败也有界清理并抛错，R10 finally保全LOG。独核R9/R10完整操作日志的 **45+43** 个子进程argv/DB/参数与-15回收；**324+392** 个HTTP原body SHA全核。原件执行Q版本按作者报告绑定，不冒称后来的compactReady/18ee已经重跑R9/R10；业务载荷未重新全域判定。见 `R9-REVIEW.json`（含两大日志完整SHA）与 `R9.diff`。
