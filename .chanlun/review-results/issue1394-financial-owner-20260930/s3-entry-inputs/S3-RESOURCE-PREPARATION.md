# #1394 首批 S3 具名 TestOnly 资源准备

状态：**仅准备**。未开 serve、Browser 或 9011，未完成资源验收或 wholegraph。实际运行的是已绑定当前 release CLI；不是旧 runtime-plan 数字。

基线 `7d6dd63aeb6bb6861a9b91d794225d93818ad317`，release SHA `5884a388e435132558f4bb33489a82e22c5948fbe71612849ca472978c3f1762`。INPUTS.json 冻结输入、clock、profile、catalog、rule、schedule及 binary/source绑定 SHA。原 one-bar TEST.OHLC 的 input-1/clock逐字复制，业务身份和值未更改。ready/receipt只是同会话合法typed查询，receipt逐字指回实际原消息与payload hash。自有新DB `owned.sqlite`。

init→ready-before→ingest→receipt→ready-after 全部exit=0，stderr=0；ingest与receipt均Committed且收据相同，generation=1。每步deadline30秒、PID及lsof原样采样已落盘并reaped。recover因冻结clock没有监督器事件为NOT_RUN，没有补clock。

七字段只位于 resources.json，外层exact `schema_revision`、`s_resources`，版本 `s-structure-resources/1`，七值全部显式规范正整数文本，无extra/default/新量纲。

| 字段 | 值 | 本夹具依据与范围 |
|---|---:|---|
| max_frame_bytes | 4096 | 最大具名帧1317 B ×2后向上2幂；实CLI payload和按当前源码派生服务回复（未实收socket） |
| read_timeout_ms | 1000 | 调用方完整帧到达预算；真实socket延迟未测 |
| write_timeout_ms | 1000 | 调用方回复排空预算；真实socket延迟未测 |
| response_timeout_ms | 6000 | 最大整进程观测向上562ms×10再向上1000ms；预算选择，不是handle峰值 |
| queue_capacity | 1 | 具名串行域最多一个等待handler请求；尚未观测真实队列 |
| max_connections | 1 | 后续一个Unix连接串行发送；当前没有service连接 |
| audit_cache_source_bytes | 1 | 已证明审计临时生命周期无跨request源keeper，保留字节0≤最小正cap1，不是未测填0 |

FRAMES.json逐项记录实CLI canonical+LF帧、请求帧及**源码派生**预期服务回复帧；不是冒充socket观测。Ready额外绑定具名 `S_CONTROL_INSTANCE_ID=issue1394-s3-onebar-20261001`（28字符），同时按Controller UUID36字符上界预留8字节，最大预算帧采用该36字符上界。UUID只是长度界，没有伪造实际服务身份。若Controller使用不同实例字符串、错误诊断、其他catalog/profile/clock/数据/并发域，必须重新核帧尺寸和资源预算。framecap不是生产cap或全部错误帧上界。

整进程最大观测包括init、冷进程审计、开库及lsof采样成本；没有孤立的handle或冷审计时长。每个CLI新进程均走现有state::audit，材料行数见OWNED-MATERIAL-COUNTS.json，审计射程限此一bar源。最大数字FD**采样**7；ready-before未捕获活numeric FD（不能声称0 FD），均不作FD峰值。永久Generator缓存、临时heap峰值和RSS均未测，另账，不属第七门。

ORIGINALS-BEFORE/AFTER完全相同（SHA/bytes/mtime_ns），包括旧源DB及wal/shm/lock；从未SQLite-open原源。全部自有子进程退出并reaped，未启动listener。

后续正式Controller验收清单及边界计划见RESOURCE-PREPARATION.json的next_acceptance。先核新产品binary/source绑定与资格，再用自有DB和具名实例实际运行单连接ready/ingest/receipt；保留canonical+LF实收帧、持久前沿与收据、transport与handle分开计时。边界域包括fragment/no-LF/cap及cap+1/不读回复/第二连接，分别有界、串行清理，明确队列资格。超时handle只poll到终态不重启。每个场景保留失败、检查原件不变并reap；不得由这些准备物声称服务已验收。
