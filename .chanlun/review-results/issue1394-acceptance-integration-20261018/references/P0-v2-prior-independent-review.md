# #1394 P0 v2 控制原语独立审查（2026-10-18）

结论：**修复进展可以接收；P0 定版与执行候选仍 BLOCK。** 原三项 HIGH 的主路径、非有限指数、一次消费、登记与共享读取界已补，普通/-O 原41项均通过；独立反例又确认两处冻结身份/证据归属不变量的矛盾。先在新包修下述 F1/F2；不改原封包，不据此执行 Rust、Long、真实 S/E/B/Q 或关闭任何图票。

## 身份及验证范围

被审 `/Volumes/AgentStorage/issue1394-real-driver-p0-v2-20261018`：
- driver SHA256 `3b29bbbebb7df49bf28a58ff04597a2e93cb5d92f27f21eb79642b7421cea380`。
- MANIFEST SHA256 `21422ddca25cfce0b84999a621d34746da99086e2e2ededfee01ba2d925c51d9`。
- REPORT SHA256 `65389a19bfc98ffc3f4e8e5af22e532e13f35c393f8ed38109bc4f94728ff3d8`。
11 项 MANIFEST 大小/摘要全符，SHA256SUMS 12 项退出 0；oracle 与原包逐字相同。审后再次核 11 项零变化。完整前次输入为 `/tmp/1394-real-driver-independent-review-20261018.md`，此次复用其协议/原件边界，仅审 P0 差异及相关上下文。

本审只向 `/tmp/r1394p0-uqhcupi9` 复制 driver/oracle/test 三文件，再运行轻量测试。普通 `python3 -B -m unittest -v test_driver`：41/41、0.230 秒；`python3 -O -B -m unittest -v test_driver`：41/41、0.223 秒。不是 82 项不同业务验收。日志 `independent-normal.txt`、`independent-optimized.txt` 保留在该目录。

额外探针与原始结果：同目录 `independent_probes.py`、`independent-probes.json`。所有数据库均来自单测临时 SQLite；进程仅短命 Python gate 与 `/tmp` 下只打印固定文本的 toy shell 脚本。未读取真实业务库、未运行 Rust/Long/产品、未连接真实资金或外部服务、未 push。已创建的真实 toy child 均已 wait/reap，临时每案例目录按测试流程回收。

## 概念层质询结果：发现实现与合同不变量矛盾

规范没有被文字改写：固定来源/准入摘要、同一绝对截止、只处理自己登记的进程和证据目录、失败不得冒成功仍是合同。下面两项实现却违反这两个关键不变量；不宣称全面工程层通过。它们不需要新教义或放宽规范，应在现有 #1394 P0 写集中窄修。

### F1 [HIGH] 第二次哈希重新建立基线，可接受偏离冻结合同的程序

位置：`driver.py:565` 核 command 的冻结 SHA 后，`:616` 再哈希并无条件赋 `self.binary_before`；`:646`、finish/cleanup 此后只与这个新值比较。

独立实际 toy 反例：先为脚本冻结 SHA `2a1e4a46a2ffc43bcb4a01b9c90c2c9fdae21411757b1eabcfa4e87ac85ca259`，第一次 bounded_hash 真实读完旧文件后，将同一临时脚本改为只打印 changed 的新文件，随后返回真实旧摘要。没有伪造哈希值，也未触碰任何真实 binary。check_command 因旧摘要正确而通过；构造器第二次哈希取到新 SHA `863ff672f5c8b0e159897027d0c135e0946d1737c86705b094d1dcc966a1b71e` 并把它当基线。实际 child 输出 `changedn`、returncode=0，finish 的 before/after 都为新 SHA，cleanup.reaped=true；冻结摘要与实际执行基线不同却没有拒绝。ACK 只替换为明确 toy 返回，用于隔离本地身份逻辑，不冒充真实监督。

此“二次基线覆盖冻结值”在原 v1 已存在，本次 P0 又新增放行前重哈希但仍比较新基线，故不是声称 v2 新引入；它是此次完整启动失败路径复核发现的未封口缺口。合同要求按冻结产物执行，不能把另一次读到的字节自动升级成新授权。

修复：以 command/准入原件的 binary_sha256 为唯一不可变预期值，每次观察（构造前、ACK/落盘后放行前、结束与清理）都与该值比较；不得以观测值覆盖预期。构造前发现变化应在 Popen 前拒绝；已有 gate 时变化必须保持未放行并回收。加“首次合法核验后真实修改 toy 文件”的锁，断言拒绝、marker 不存在、原阶段已消费，不能只测 bounded_hash 从一开始就返回 wrong。该修复只封已证实的重置漏洞，不把前后哈希说成原子执行或完整外部监督证明。

### F2 [HIGH] 构造时目录未获所有权，cleanup 仍向已有目录写原件

位置：`driver.py:624` mkdir 与构造异常分支；`:807–808` 只凭 output.exists 且无 CLEANUP.json 即写清理回执，没有“本实例成功创建并仍持有该目录”的检查。

独立反例：先在临时目录建 `preexisting-evidence/SENTINEL`，传该已有目录给 OwnedProcess。mkdir 正确抛 FileExistsError，Popen mock 证实没有启动 child；异常处理随后却写入该目录 `CLEANUP.json`。原目录从 `[SENTINEL]` 变为 `[CLEANUP.json,SENTINEL]`，新增记录 owner={}、reaped=false、errors=[]。虽然未覆写 SENTINEL、也没有假报 reaped，但已经污染不属于本实例的既有证据集合。使用 `xb` 只能防同名覆写，不能授予对别的运行目录的写权限。

修复：记录目录创建成功及其稳定身份/句柄，只有本实例实际拥有且未被替换的目录才允许写该目录内原件。mkdir 失败/已有路径/symlink 均不得在该路径写 CLEANUP；失败信息交给已拥有的根级收据位置或返回调用者。测试既有空目录、含 sentinel 的旧目录和非目录路径；Popen 未启动时保持原路径逐字/目录项不变，同时成功创建后的 ps/ACK/binary 失败仍要留本实例回收记录。

## 原 P0 修复的已核定部分

1. **晚 ACK**：ps 使用剩余 timeout；connect 后/send 前/recv 后/strict 返回后重算同一 min(root,stage) 截止。五个 fake-clock 反例分别耗尽 ps/connect/send/recv/parse 均拒绝。ACK/OWNER 落盘后和 gate 写入前有复查；gate 自己在 exec 前检查原绝对截止。真实 toy 的 ACK 丢失和 ACK 落盘延迟均没有产生 marker，child 被 reap。
2. **正常退出及身份竞态**：poll 已退出、ps 时进程消失均回到本 Popen.wait；allow_exited 不导致向陌生 PID 发信号。独立额外探针在 ps/身份读取前抛错，gate 未释放且被 SIGTERM/reap，三流关闭；身份不符探针信号数为 0、reaped=false、错误保留、三流关闭。TERM 超时转 KILL 的现有锁仍过。cleanup_all 单 owner 异常不阻断其他 owner，错误不包装为成功。
3. **SQLite 生命周期**：closing 包住 connect，finally 清 progress handler 与 rollback，外层始终 close；成功/JSON解析失败/查询失败三路测试均实测 close。未以 with connection 的事务退出冒 close。
4. **SQL/原件同界**：ReadBudget 显式共享 records/bytes/deadline，SQL 每行在 strict 前计费，余量设置 SQLITE_LIMIT_LENGTH，busy timeout 与 progress handler 使用当前剩余截止；跨第二个 DB 调用触顶和昂贵 view 中断均通过。read_original/hash_original 64KiB 分块累计，失败不退还额度，custody 与 STARTUP 消耗传入同一对象。该预算不等于所有 native heap/RSS/磁盘的硬隔离，不冒充完整业务 Job 八维计费。
5. **strict overflow**：parse_float 验有限值，正负指数溢出、嵌套、Infinity/重复键拒绝，1.5 保留；非优化与 -O 均适用。不存在依靠 assert 才守住的新准入条件。
6. **一次消费/登记**：命令/帽匹配后先消费 stage/domain/epoch，再核摘要；失败不退还，同实例第二次拒绝。Popen child 在 ps/ACK 前 register，构造失败进入 cleanup；同域现有 live owner 禁并存。此处只确认当前原语，不证明 P1 的新 epoch、前驱收据、跨进程调度或业务身份规则已完成。

## 工程层与验收边界

发现上述合同不变量矛盾后，不签发工程层总体 PASS。F1/F2 为 >99% 置信的具体可复现问题；不重复列已披露的“尚未完整 P11”作为新增缺陷，也不要求本 P0 包假装实现所有外部 supervisor 功能。

41 项当前测试通过证明既有 toy 域修复有效，不证明已覆盖本审新增两条反例。修复后须保留原包，在新版补 F1/F2 拒绝锁并重跑普通/-O；只复审新尾差，不重新下载或重做无变更的 v2来源/R1–R3审核。

同步文件系统 syscall 只有前后期限检查、无法本线程中断内核阻塞；外部 supervisor 尚未交付完整持续采样/失联处理；当前仅回收直接 child，不是整进程树。作者已明确披露，这些仍是接通真实产品前的独立门，不能由上述 toy 成绩抵销。

P1 scheduler 的全域身份/原件/前驱绑定、独立完整核账与 closure 遍历、资源对象全组合、完整 Long/Voice 冻结、ARTIFACTS、精确单次运行准入仍未完成。execution_authorized=false、PARTIAL_STATIC_ADAPTER_NOT_P11_ACCEPTANCE 保持；#1394/#1361/#1340/#1323/#1279 均不能据此关闭。
