# #1394 current release 正式 fullclock prefix16：独立事实复核

**PASS_BOUNDED。8 个检查组通过，新增事实异常 0。** 业务通过只限本次current源码、完整9011 clock、同一有限七预算下的两个fresh prefix16。dev旧业务FAIL/pending原件保持，本报告不关闭#1394或整体Goal。

| 检查组 | 独立核验 |
|---|---|
| 新包与旧失败包 | release manifest846项全部bytes/SHA/mtime一致；旧dev244项仍保持原SHA/mtime，并对release BEFORE身份独立核inode/device。新报告属独立新增，不误判为旧manifest的extra file。 |
| 真构建与binary | build PID35499实际wait exit0、samePopen reaped，39.289039834秒。原build.stderr确有非test bin的rustc -C opt-level=3，--release --locked、incremental0、当前Cargo.lock/Toml绑定相同；7,176,304字节built artifact与冻结binary均为b4c8…c7f5。未重build/unit。 |
| 同源输入与预算 | root ed1/Rust5a2与当前绑定一致；BEFORE/AFTER相同，52项旧原件当前身份一致。prefix16与dev逐字相同，仍为790,213字节fullclock b02…04f01、resource b3…54e69及response30000ms，没有退回16 clock或改七值。 |
| 真服务与回复 | A37407/B38903原ps/UID/完整argv/nonce、私有资源canonical SHA对应当前release binary；34个完整公共信封独立核canonical+LF、exact头、payload SHA、causal/reply ID、session/S producer/epochs。各16实际Received Committed，无超时，request/reply≤4096字节。 |
| strict全域对拍 | 两fresh初始化相同；最终全24表、schema、48,403行/BLOB逐项相同，semantic SHA76d2…9762a。各16消息Committed、attempt1、48条原clock事件、generation16/idle、无pending。16组reply/receipt/clock/nativepublication exact；47,671个持久内容节点framing hash独立核过。 |
| 四域共同前沿 | releaseA/B与devA/B共同1..8的实际回复、persistent receipt/clock/native publication逐字/逐值相同。devA message10、devB message9仍为AcceptedPending，旧业务RESULT仍FAIL；release成功不抹旧30秒失败。 |
| 采样 | A595/B590条100ms样本的RSS、SQL logical/allocated、minfree、gap和首尾覆盖独立重算一致。最大gap121.424250/115.069917ms；最大值仅采样下界，launch→reap仅观察界。 |
| 终态与范围 | A SIGTERM=-15、B SIGINT=-2，init均0，全部samePopen wait/reaped；build及服务当前PID不存在，自有socket目录不存在。父任务exec70359 terminal0与业务prefix16 PASS分开，完整9011/全部资源/整图仍未通过。 |

最大**整个exchange**为A **10.536168292秒**、B **10.500176125秒**。原件明确physical_handle_ns/sql_commit_physical_ns为UNMEASURED；本轴不把它们称为handle独占、SQL COMMIT、Hash/audit/build/store内部耗时。当前dev与release事实各绑定自身binary，clock、预算和原消息不变，观察到的差异不是完整受控CPU负载实验，也不能据固定parse/canonical/clone计数或审计增长式独自给出完全因果及线性9011外推。

最大采样RSS为A177,340,416/B180,207,616 bytes；两条路径采样DB logical/allocated均6,848,512、WAL logical7,370,712/allocated7,372,800、SHM32,768。原始samples首条在child launch观察前、末条在reap观察后，覆盖实际记录的观察范围；不声称严格峰值或真实完整进程寿命。CPU/内部阶段仍未测，没有把未测量填成0。

old8f9 release另有build/code/clock/接口，不能当本次current source release的等价运行或因果基线。本报告复用已经完成的dev消息来源核验，不重放9011或同一Actor路径。Q、Client、生产容量、全部资源峰值、9011、整图与Goal仍未获本包证明；root应统一接回checkpoint，不能从本prefix16通过跳到wholegraph完成。

固定证据：

- ROOT HEAD：`ed1f184bf7513d960d87b6687c279a657c8fa481`
- Rust tree：`5a2acd1e72c3846be38cc29689a82aeb57c68dc6`
- release binary SHA：`b4c81c9468be1d1911eecbd48deb1be9e9a2f3a0d082fd8f7474cf9e30efc7f5`
- profile：release opt-level3/ltofalse/codegenunits16/incremental0，实际non-test rustc已核
- clock SHA：`b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01`
- resource SHA：`b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69`
- MANIFEST SHA：`b5a9cc1c576cc813acb53aceeadd89ddca097ed04d5a0882104647593467f78e`
- RESULT SHA：`6122feb186aa0d35e794e36da394082e9ca5204bef4c15a64f554396a45d32eb`
- fullDB semantic SHA：`76d25b386cee9e11b3cc0e29461a9d3a1d175238ae86550931ede9a3e5a9762a`

逐项机器结果与重算采样见同目录JSON。本轴仅新增本报告和JSON，仅读原件、不打开SQLite、不运行服务/build/unit/新probe，不改main、push或tracker。
