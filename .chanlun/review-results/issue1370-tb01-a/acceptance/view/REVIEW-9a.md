> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# TB-01-A 耐久性修后定向复核（9a1bc124）

**结论：CHANGES_REQUIRED，新增 0H / 1M / 1L。原 DUR-H01、DUR-H02、DUR-M01 的真实反例已修复；新增 DUR-M02 仍阻断已签 AC4，不能被这些通过结果抵消。** L 项仅是 CAS 跳过时的诊断用词，不是状态保护失败。

固定提交 `9a1bc124133d84fb3f575e6dd0da2bd322eefb3c`，tree `660ff5dc52c971b36894cbf2ca60e31fee8f3a07`。实际 native SHA-256 `066324ab50e9547777fc5f5870ae98c404ba3de53ce25e6da9855d142ee174c3`；根构建记录与本轮读取的 408 份 Rust 构建源指纹一致，入口文件与固定 commit 的 git blob 逐字相同，源码 SHA `84b7eff5e46d166aeba3d6ff59108bc71aec156232c13891d6de0a5faed13ae7`。二进制复制至本目录 `frozen-final/s_structure_session`，复制前后 SHA 一致，所有探针执行此副本并使用绑定的 macOS Python/lib；完整绑定见 `EXECUTION-BINDING.json`。

## 新增 DUR-M02 / P2：不可变批次没有先持久可读，再发布引用

这是 **#1370 AC4 已明确承接的 A 义务**。冻结 SPEC C09.4（`:325`）规定“先持久写完整对象，再由同一 S 事务发布可达引用”，并明确允许崩溃留下的不可达对象回收；接口合同 `:55` 规定 Commit 对外只在引用的全部批次对象已持久可读后发生。#1370 AC4 又直接写入“完整不可变批次先持久可读，Commit 事务同时发布对象、关系、修订和结构索引可达根”。不能因为完整历史/故障恢复还在 B/TB05，便把 A 的这一条已签顺序推迟。

冻结 Rust 入口 `:1071–1073` 创建 Commit 写事务，`:1102–1106` 在这个事务中首次 INSERT batch；对象/关系/修订及 `:1219–1225` 的 generation/cut/index_frontier 随后仍在该事务内，`:1265` 才执行唯一 commit。因而不存在一个对独立读者可见的“新批次已持久，正式根仍旧”的阶段。当前同事务原子提交确实避免悬空根，本报告**不声称它已经导致数据损坏**；缺陷是批准的先持久后发布协议未实现。旧批次 hash、不可变性、引用闭包检查全部通过也不能补出这个时序状态。

最小修复：完整批次先在受 epoch 约束的有限事务内持久，并对已存在同 ID 核对规范字节/长度；随后以独立短事务核自己的 Begin、epoch、generation、input frontier，再同时发布对象、关系、修订和可达根。第二阶段失败不能抬新根；未知中断仍留 Begin，可留下不可达批次。这里不要求实现崩溃恢复、回收器或完整历史。

本项由冻结源码和已签合同直接成立，没有额外运行 native，也没有假称已观测当前代码中不存在的中间持久状态。

## 原 2H / 1M 与指定边界的实测结果

全部使用独占新库，完成 **12 个场景、79 个场景级断言**，另有两批次内的逐条哈希/闭包断言。机器摘要逐项记录真命令、返回值和持久数据；命令总数 57，故意制造的拒绝返回不计为工具失败。

| 项目 | 实际证据与结论 |
|---|---|
| DUR-H01 正常交错可继续 | 512 条合法单调输入被实际 S 接纳；从独立只读连接看见持久 Begin 后，只 SIGSTOP 自己创建的 advance PID 并核 WUNTRACED。第二个 advance 拒绝；新的第 513 条由真实 Accept 成功接纳。恢复第一个进程后，它因前沿变化拒绝发布、将自己的 Begin 正常取消为 idle；随后真实重试无需 reset 即成功发布 generation 1。原永久 begun 反例已消失。见 `runs/normal-interleave-r3/interleave-normal/summary.json`。 |
| DUR-H02 过期 epoch 接纳零写 | 仅在该新库构造 writer_epoch=2 前件。真实 Accept 拒绝并报 StaleWriter；所有应用表完整逻辑指纹前后相同，含输入/收据/profile/meta。这里“零写”指应用记录零变化，不声称文件系统没有 open/sync。见 `runs/epoch-r3/stale-epoch/summary.json`。 |
| DUR-M01 profile 重放不偷换来源 | 同 profile 同输入真实重放逐项返回原收据；同 profile_id 仅改变中文 note 的规范字节后，Accept 以 IdentityConflict 拒绝。generation、cut、根和 profile hash 不变，Snapshot profile 仍等于已封存批次来源。见 `runs/profile-r3/profile-and-batches/summary.json`。 |
| 0/1/2 知识不足 | 三个独立真实会话分别成功发布，均无伪对象，持久 raw 数量正确；知识不足的 `detail.raw_events` 分别为 0/1/2，不以域不满足覆盖。见 `runs/boundary-r3/{zero,one,two}/summary.json`。 |
| 三点同价 | `[10000,10000,10000]` 成功发布一条 `adjacent_inclusion` 事实，带原始三价格；无 UNIQUE 错误、无知识不足伪装、无第五 Other 对象。见 `runs/boundary-r3/same-price/summary.json`。 |
| 部分包含与去重 | `[10000,11000,11000,12000,13000]` 的 raw 窗口 0/1 两项域事实、`[10000,11000,11000,12000,12000,13000]` 的 raw 窗口 0/1/2/3 四项域事实全部逐值保留。这里只把合并对象作为诊断读数，**不将含相等输入的合并结果算作本片或未定 G 的成功验收**。见 `runs/partial-inclusion-r3/`。 |
| 旧批次字节、hash 与闭包 | 实际两次发布后第一批完整字节保持不变，两批长度、canonical 字节和 batch ID 一致；两批内 17 份 raw 记录与原收据 hash、13 个对象 ID、input_refs、raw/merged 见证及关系均由独立标准库 checker 完整核对。见 profile 场景的 closure 数组。 |
| token/epoch/generation 防误清 | 真实 advance 在持久 Begin 后暂停；在自有库分别设置另一 token、新 epoch、已发布 generation 标记，恢复后均拒绝且保护字段和旧根原样保留。**这些 native 反例首先触发 Commit fence，不能声称已动态跑到 cancel helper 的所有跳过分支。** helper 源码 `:784–806` 另确认在同一 IMMEDIATE 事务内逐项核 epoch、编码含 token/gen/frontier 的 state、begin_token、published generation==gen−1；只有匹配才写 idle。generation 场景是受控标记，不冒充第二实际写者的完整 Commit。见 `runs/native-commit-guards-r3/`。 |

在当前退化 tick 域，包含等同于相邻价格相等。冻结 inclusion 源码 `:41–42,110–158,423–439,446–477,522–557` 表明：全程无方向时 merged 保留原 raw；出现严格方向后，重复价格经同源包含处理折叠。配合 local_shape `:133–140` 的域判断，本轮没有发现“不同 raw/merged 实际窗口仅因编号相同而被 dedup 抹掉”的本片可达反例。上述两条部分包含输入又实证所有原始域事实仍在。该判断没有外推到非退化 OHLC 或未来其他结构轴；扩域后应重新明确观察坐标域身份。

## DUR-L01 / P3：CAS 未清理时诊断仍称已释放

`cancel_own_begin(:785–806)` 在 epoch/state/token/published generation 不匹配时返回 `Ok(())`，保护状态不动；`:1272–1274` 的调用者无法区分“已清理”和“保护性跳过”，统一输出“本 attempt 已正常取消并释放推进权”。若保护条件在前沿检查结束到 cancel CAS 开始之间变化，文案会错误描述实际结果。

这是**非阻断诊断项**，源级条件关系明确；本轮没有声称真实捕获了这个极窄时序，也没有将它报成越权清理或耐久破坏。最小修法是 helper 返回是否实际取消的 bool/enum，按结果说明已取消或已不再持有该 Begin；不能为让文案成立而放宽保护条件。

## 下一轮最小顺序验收已准备，尚未执行

`publication_order_probe.py` 已完成内存语法编译，明确拒绝本轮 9a commit，并要求下一份根绑定中的 `publication_order_fix=BATCH_DURABLE_BEFORE_ROOT`。它只使用本目录的新库和冻结二进制：先真实发布 cut-1，再接纳 512 条追加；仅在独占库安装记录实际写连接阶段/SQLite/sourceid/FULL/fullfsync 的观测触发器。

探针先从新的只读连接看见真实 Begin，暂停并核验自己的 PID，再继续；随后用新的短只读连接捕获“新 batch 已提交可读、index_frontier 仍指旧根”，再暂停自己 PID。暂停后换两个新连接复核同一中间状态，独立校验新 batch 全字节/hash/引用闭包；最后续跑 Commit，核新根正好指向此前已持久的 batch、字节未变且旧批次保持。若暂停太迟或没有捕获状态，报告调度未形成证据，**不得冒称通过**。不杀进程制造恢复，不增加 TB05 或全历史义务。

新协议允许前沿变化后留下未引用批次；下一轮复用 DUR-H01 探针时，“无陈旧发布”应核旧根/generation/对象未动，不能继续把“batch 表必须零新增”作为拒收条件。本轮 9a 的该断言只属于当时单事务实现的实测，不会跨版本沿用为新存储协议。

## 边界与产物

没有访问根数据库、18770–18773、GUI、E/B/X，也没有修改仓库/GitHub。所有创建的 native 子进程均已退出。源码/二进制指纹、命令记录、场景摘要、已有冻结合同的 SHA 与本报告对应 JSON 可复查；纯旧批次 checker 自检不计作修后 native 验收。本轮只承接上列定向义务，不宣布 #1370 整票、TB-01 或 #1323 关闭。
