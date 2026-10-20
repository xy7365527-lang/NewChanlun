# #1394 本机 same-material 只读审计收益验收

状态：PASS；完整 A DB/WAL/SHM 的两份独立 APFS 克隆，各执行两次 state::audit。
候选 c6255d308fc69d94178916a1dc5534546fb77591 / Rust tree 7196a77fef56cf87cbe91dc4b135326a142e2395；诊断 patch SHA256 e18ffbaefceb9c577f00c92bfed9edb8821e498afce481f9a90011f0f8d13ff3；testbin SHA256 5f9ef56e7cd0ee27dd00094faaf5c73821d6501fe38b4783c238c67d80cb286f。
24 表/48,403 行/96,397 BLOB、AuditedState 四成员、每 cut 五成员、578 marker、SQL found 3,082,463 / unique 35,272 均逐项对拍；原 source 与 589 个原 target 保持 SHA/inode/mtime。

第 1 次：同机旧版 4.117123s，候选 3.803846s；F 4,406,786 → 1,386,860（减少 3,019,926）；reuse hit/miss 3,019,926/62,537；retained peak 35,272 entries / 2,025,582 B。
第 2 次：同机旧版 4.112855s，候选 3.769925s；F 4,406,786 → 1,386,860（减少 3,019,926）；reuse hit/miss 3,019,926/62,537；retained peak 35,272 entries / 2,025,582 B。

RSS 0.1s 采样：旧版 66043904 B、候选 75497472 B；heap peak UNMEASURED。
owned -shm 的 mtime 逐段记在 JSON；各 clone 的 DB/WAL/SHM 字节、SHA、设备与 inode 均保持一致，原 source 与原 target 的 mtime 仍严格不变。
walltime 为本机该次独立审计，不能并入 Q 正文 fixture 采集；full9011 与整图未由本 pilot 证明。
