# #1394 有限 S16 正式 Q21：独立事实复核

**PASS_BOUNDED。9 个检查组通过，新增事实异常 0。** 仅覆盖实际16页Snapshot/472行、content[0,7)、AsKnown0及合法retention Gap；不表示完整Client、生产或9011完成。

独立实读当前MANIFEST全部265项，bytes/SHA/mtime/inode一致。current root ed1/Rust5a2、release binary b4c8…c7f5和原Q21 50ea…d4eb1绑定一致；21个资源数字未改。两组各20份original BEFORE/AFTER/live完整身份一致，包含所需六份当前Controller/socket/Q源码，未重扫574/846文件。

原release1/A源库g16的DB/WAL/SHM在无holder的两次稳定观察中相同，三文件逐字copy，WAL仍7,370,712字节。copy链只有原件stat/read/copy，无原source SQLite-open、checkpoint、chmod或immutable绕WAL。owned copy为自然0644；primary与continuation的baseline/afterQ全24表、48,403行/schema/BLOB一致，且等于已验证release源快照。

物理边界照实保留：owned copy各closed observer的前后，以及各Q上下文前后，均有**SHM mtime变化，bytes/SHA不变**；具体纳秒值见JSON。DB/WAL没有这些差异，原source三文件完整身份未变。只读业务结论由query_only/READ_ONLY正本行、两个Q主DB fd5r和完整语义无写入支撑；不宣称Q或observer所有physical metadata零变化，不单独把协调mtime归因给Q。

两次实际进程均为正式Controller schema3/start_q：primary PID55893/port51472与continuation PID60820/port53838，UID501、完整argv、不同nonce、private S_CONTROL_RESOURCE_SHA256未串入Q，Ready均绑定g16/cut16/native frontier。第二次仍用同owned DB和其合法ROOT/state；原q.process与q.log已先冻结到continuation1/old-controller-state，旧PID已reap。第二次只有必要Ready、AsKnown0、Watch三个请求，没有重复全Snapshot。

实际HTTP公共信封独立核exact头、payload hash、cause/reply ID、session/Q producer/epochs；已执行请求均HTTP200。16页offset按31递增，末页7行，next token逐字转交，472个family/key唯一且计数一致。全部row descriptors重构的projection root为 `sc1-460b659a1fd449f701c5c6f30e20a1a6b07343661d54443dd01f03298918b314`，qcut3 digest也独立对上。fixed cut/index_frontier对当前S publication，objects/observations/witnesses对持久native引用，不跨clock上下文比较旧根。

content实际返回63个path/header proof节点，逐项framing hash一致；前7个item ref按row metadata重算，record作为Edge，472行array target根也对上。源DB观察的47,671个持久节点另核framing hash。这些不是HTTP完整递归body/sourceclosure，也不是Client安装或actual allocation。

原RAW-HARNESS-RESULT仍FAIL：观察器错把content.items的ContentRefs直接同row对象比较产生AssertionError；实际content HTTP200/ok及总数472成立，未观察capture/verify资源失败。离线根关系修正没有重发该请求。缺失的AsKnown0与Watch由授权的新Q上下文接回：AsKnown/as_of0返回cut0空prefix；cursor0、retain4、first13/head16返回HTTP200真实Gap，missing1..12、无delta，rebuild仍指当前cut16。原preflight表示/format错误与continuation ownership失败原件保留，未冒作业务成功。

两个Q都是SIGINT graceful exit0，同Popen wait/reaped；当前两个PID不存在，原件记录终止后两端口重绑51472/53838，本轴未重新bind。exec94568/94704终态0来自父任务运行时，原始进程与本次现场PID检查独立核过。

有限负载下原Q21的正式Ready/capture/verify与查询路径可用已获本射程证据。未证明throughput、生产容量、完整9011、全部资源、Client完整body/安装/物理分配、Browser、FIFO、整图或Goal；未测量值保留UNMEASURED，不填0。通过不关闭#1394/#1323。

固定证据：

- HEAD：`ed1f184bf7513d960d87b6687c279a657c8fa481`
- Rust tree：`5a2acd1e72c3846be38cc29689a82aeb57c68dc6`
- binary SHA：`b4c81c9468be1d1911eecbd48deb1be9e9a2f3a0d082fd8f7474cf9e30efc7f5`
- Q21 SHA：`50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`
- MANIFEST SHA：`7febd5197c74979d31ffeefbe8a23d59d1aeb5bfcdecee6e443a433901922422`
- RESULT SHA：`026b115f647ac6870f9790eac224c6b1b46d73873e3ce877822836ee2d024b03`

本轴仅新增本报告与JSON，未改原件、代码、服务、main、push或tracker；未build/unit/新probe/新矩阵，未打开SQLite。
