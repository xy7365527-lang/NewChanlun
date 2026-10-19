# #1394 同记录局部字段批读增补

本文件为 open #1394 工作草稿，归 #1323 及已批准 C 增补。候选只改 source_audit.rs，三处将同一个part/payload的多个字段在明确BudgetNone审计中展开一次：形态part三字段、组high/low两字段、见证同组high/low/open/close四字段。字段Ref只在当前对象调用栈使用，不跨记录/cut/事务缓存。原数值/连续性、组存在、真实来源成员/差集、窗口/身份、见证组值与完整闭包/索引/clock及三次审计均保留。

局部代码独评通过，11个靶向测试通过。Some仍沿原即时path读取与业务交错顺序，完整夹具reads151351/F170797及原预算/错误轨迹一致，期限仍先SQL。原scalar允许Number/String/Bool/Null，新局部Ref解码保持该域，再做原as_i64/as_str，true不合并成1。原state.rs:60默认Clippy deny继续留红，没有加allow/删test/ignore/越权改常量；rustfmt和diff检查通过。

| 同对象字段数 | 旧节点读取 | 新节点读取 |
|---:|---:|---:|
| 3 | 118 | 50 |
| 2 | 227 | 174 |
| 4 | 400 | 175 |

固定单文件SHA `aa5ee2c0261e7487446c9991559e028d87475a405c741fe5b331b74b0c11e3b5`，patch SHA `f5d9348d991adf0b9b17635596537a9299b27ec0d91504ff599bb7936eed2f31`，已本地隔离集成 commit `d3956b0310c96db4cbbd79d2ad752f5b0803f738`、Rust tree `0eb9bec56e9d99606622ce11b4869eefebae5763`，未合main、未push。

原Root pending28三件只作文件字节复制，SQLite仅打开field新owned副本，readonly/query_only。相同冻结 DIAGNOSTIC.patch/observer release 两遍对拍前轮array release：完整四成员及每cut全部字段逐值相同，SHA均 `42444317d86a03e066867bdfccfd0bb9f07010a1a2655483fdc377b7a115b8a0`，963阶段事件顺序、overlay读写及unique digest相同。Root重新计算确认。

每遍节点SQL 8,090,429→7,753,079，少337,350，约4.17%；F仍3,400,571。候选audit观察10.671285250/10.631571042秒，跨时OS负载未控制，不把wall变化归因或外推工期/30s门。材料仍raw28/cuts27/Committed27+AcceptedPending1，不是input28之前状态；未删除pending、恢复、重投或启S/Q。原件bytes/dev/inode及DB/WALmtime、自有DB/WALSHA+mtime有记录，SHM实际变化不冒称零。

只复用一个本轮自有release target，未重建旧array二进制。诊断源码/可重建target清理后保留源码补丁、observer、两bin、build与targeted日志、PROBE和RECONSTRUCT。candidate observer bin SHA `16f66083fab18b69a1fac5f98847ee84402391703a6c52de68d5ec3ae4f147aa`；它是test观测产物，不是正式CLI。

旧374份tracked packet文件原样保留，本轮23份小型冻结源码/报告/监督/日志位于field-read-inputs。大型三件库、bin、完整PROBE仍caller-owned，按同名JSON完整路径/大小/SHA读取；含硬换行、patch空白或末尾空行的原件封进raw.json保持UTF8字节。此局部读取量改善不完成全9011、30s服务域、C或六Destination。新非testCLI、后续正式full目标及其余原验收继续保持未完成；#1460前端独立。
