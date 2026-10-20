# #1394 fullclock dev 正式 prefix16：独立失败事实复核

**VERIFIED_BOUNDED_FAILURE。事实复核 PASS，9 个检查组通过、新增事实异常 0；业务结果仍为 FAIL。** runner handle33698 的 terminal exit0 不表示 prefix16 或 9011 成功。

| 检查组 | 独立核验 |
|---|---|
| 包完整性 | 原 manifest 244 项全部 bytes/SHA/mtime 一致。release1 新增文件不在旧 manifest 射程，不据此判旧包多文件异常。 |
| 当前绑定 | root ed1、Rust tree5a2、dev frozen binary5437；BEFORE/AFTER 与当前源、clock、resource、profile 身份一致。本轴未重扫574源码或重过19/7测试。 |
| 旧原件 | 52 项 BEFORE/AFTER/live bytes、SHA、mtime、inode 等身份一致，旧失败记录未改。 |
| 消息来源 | 前16消息逐字等于9011 draft前16，并独立对拍旧fresh1/fresh2共32条原消息。OHLC为整数原值，无舍入；raw_text原volume1.0保留，typed volume文本1未更改。 |
| pending/Begin | A message10、B message9 的原canonical消息/hash、AcceptedPending、attempt1及Begin均持久；first_published_generation为null，publication/delta空，时钟仅accept+begin，无该消息commit或recover记录。 |
| 同域对拍 | 初始化全24表/schema一致；最终不同停点造成16表差异。共同已提交1..8的实际回复、persistent receipt/clock/native publication、delta/BLOB严格相同；未采samecut8 fullDB，不能宣称严格samecut DB一致。 |
| 真进程与wire | A19956/B23439原ps/UID/完整argv、同fullclock及私有资源digest一致。19个完整公共信封独立核canonical+LF、exact公共头、payload hash、causal、reply ID、session、S producer/epochs/Ready nonce。实际request/reply均≤4096字节。 |
| 采样 | A1397/B1127条100ms原样本独立重算RSS、SQL文件logical/allocated、最小free及首尾覆盖；最大间隔114.840834/115.584625ms。最大值都是采样下界，不是严格峰值，CPU/内部阶段未测。 |
| 终态与边界 | A SIGTERM=-15、B SIGINT=-2，init均0，全部同Popen wait/reaped；当前dev PID不存在，自有短socket目录不存在。prepare阶段Python symlink解析导致的false equal断言失败保留，未起服务、未改资源。 |

A 已收到9个 Committed，message10 的整个exchange为 **30.0027035秒** 后 DeliveryUnknown；B已收到8个，message9为 **30.002664625秒**。两个失败reply原件确为0字节，表示调用方未收到回复，不是JS/内存峰值0或消息被拒绝。A已持久accept130/begin131，B为accept127/begin128；各下一阶段commit时钟尚未记录，原消息已接纳而未发布。final advance_state分别仍是begun:…:10:9和begun:…:9:8。没有recover、pending重投、改身份或加预算。

本次冻结输入是完整790,213字节fullclock，原SHA b02…04f01，与有限资源profile b3…54e69、response_timeout_ms=30000一同实核。它只被接受为finite profile，不是默认值、生产容量或全部9011准入。完整clock_plan_json/hash在持久protocol中与原件一致。A/B的19,726/16,568个源内容节点framing hash独立核过，已发布native引用同当前fullclock上下文；旧clock16的根不可跨上下文比较。

RSS采样下界A171,163,648/B167,903,232 bytes；采样SQL文件上界记录A DB3,424,256/WAL logical4,511,432 allocated4,513,792，B DB2,666,496/WAL logical4,284,832 allocated4,288,512，SHM均32,768。这些是观测最大值下界。首样本在child launch观察前，末样本在reap观察后；进程launch→reap是观察界，不是精确完整寿命。整个exchange时间不冒称SQL COMMIT、handle独占或Hash/audit/store阶段时间。

旧internal8f9 release与本次dev的build、code、clock和接口不同；不据其差异做因果归因或线性外推9011。Q、Client、全部资源峰值、生产容量、整图及Goal未获证明。业务FAIL、prefix16/9011未完成与#1394仍open的状态保持。本轴只读，不打开SQLite，未对closed observer的逻辑快照追加物理效应主张。

固定证据：

- ROOT HEAD：`ed1f184bf7513d960d87b6687c279a657c8fa481`
- Rust tree：`5a2acd1e72c3846be38cc29689a82aeb57c68dc6`
- dev binary SHA：`5437fbb5d581d1b48f6a1d3405cae67327113e5a2c7fedd7ce3a1d9ce1e18e91`，dev debug=0 incremental=0
- clock SHA：`b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01`
- resource SHA：`b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69`
- MANIFEST SHA：`8e8ac71c78ab51602221869a4b43a3d8bf1582fcc740519eed1bb7c84580f351`
- RESULT SHA：`3833ff9dc4fcfbe8adfe968849e412a28e13039ef34515387a911366c4ebbc39`

机器检查、实际pending原identity/clock、源节点计数及重算采样见同目录JSON。本轴仅新增本报告与JSON，未改原件、失败记录、其他工位、main、push或tracker。
