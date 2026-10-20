# #1394 完整数组线性审计增补

本文件为 open #1394 工作草稿，归总图 #1323、已批准 C 增补。完整 root[] 数组现在可在明确的 BudgetNone 审计中按序 DFS 枚举，减少每项重新从根寻找路径。候选六文件已按冻结 SHA 纳入本地隔离集成，代码 commit `f57015506656bb32eebd5987a9996ded7a95280b`、Rust tree `c60452f606202ed6b4754ce29bd17698d115ed32`；未合 main、未推送。

新 helper 不保存 body，不跨读取/cut/事务保留证明。每个真实节点读取仍核当次 F，每个逻辑位置仍枚举，重复子树不丢位置。根 ContentRef、count/k 规范分解、叶/子值头和缺失/坏摘要仍核；header 不充当完整闭包，原所有闭包/类型/来源/clock/独立索引与三次完整审计保留。NodeSource 默认关闭；普通 lookup、fragment、计费 Some 原路径与逐项业务交错序不变。原 4MiB/65,536 资格、S7/Q21 和 30s 请求门没变。

代码的概念、Spec、Standards 独评通过。58 个靶向测试报告通过；其中新数组4、旧path11，计费路径另逐项对拍 Ref/错误/SQL/F/预算门序并核期限先于 SQL。n16 的原读取256次先使48上界反例变红，修后48；n65 为1607→195，n257为7943→771。JsonText 探针早期夹具曾错误要求 prefix 空，旧合法 JsonText+Index 的 prefix 本就非空，已按旧行为修夹具；新旧同 fixture 的引用与原完整拒绝码仍保留。旧失败日志没改写。

原配置 Clippy 仍被未改 `state.rs:60` 的 `absurd_extreme_comparisons` deny 阻断，其余既有警告照实；没有加allow、删test/ignore或越权改常量来修绿。cargo check 与变更文件 rustfmt 已执行通过，此增补不声称原Clippy全绿。

实际数据沿用全量首试停机域：raw28/cuts27，message28 AcceptedPending，无第28代发布。只作完整DB/WAL/SHM字节复制，仅在新owned副本 readonly/query_only打开；没有S/Q、恢复、重投、删除pending或新全量replay。工蜂的debug逻辑backup及简化返回SHA只保留为过程证据，不计最终完整等价或原30s门收益。

最终独立诊断副本按同 SHA 的旧 DIAGNOSTIC.patch/observer 构建 release，保留 testbin `e62213072f6c095702064f34ad1a8b388d3d0aeaadbde42e36967a2d0675a97f`。两遍候选完整四成员、每cut全部字段与旧冻结PROBE逐值相同，指纹均 `42444317d86a03e066867bdfccfd0bb9f07010a1a2655483fdc377b7a115b8a0`。963个阶段事件顺序及overlay读/写量相同，Root重新计算完整返回SHA并核对源码六文件和保留二进制。

| 同材料每轮 | 旧冻结基线 | 候选 |
|---|---:|---:|
| 节点 SQL found | 8,792,023 | 8,090,429 |
| F 调用 | 3,402,722 | 3,400,571 |
| audit 秒，第1轮 | 11.468526542 | 11.182987417 |
| audit 秒，第2轮 | 11.495750583 | 11.144998917 |

每轮少701,594次节点SQL，约7.98%。这些是只读audit/cfg(test)观测；旧新运行不同时且OS负载未控制，不能单凭wall变化归因或宣称真实S整个请求/30s门/完整9011已通过。剩余约809万次读取仍是后续实测优化对象，不能把这个局部改动当整图完成。

只清了本轮可重建debug/baseline/release目标与diag archive，Root/pilot旧589 targets、原DB和不可替代原件不动。release testbin已按原SHA搬至owned目录保留；早期ACCEPTANCE中的旧构建路径已不存，应沿RECONSTRUCT及本增补的实际保留路径读取，不能混称二者都存在。

旧349份tracked packet文件原样保留；本轮23份小型报告、补丁、源码及监督收据位于linear-array-inputs。大型DB、PROBE、过程日志与保留二进制由同名JSON列完整路径/大小/SHA，仍为caller-owned；硬换行/patch空白/末尾空行用raw.json保持原字节。新非test CLI构建、后续完整9011正式双跑、crosswalk/Q-E-B/Client实际分配、原TB和六Destination均仍未完成。#1460前端独立。
