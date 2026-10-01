# #1394 最终 S3 实际验收：独立事实复核

**结论：PASS，仅限本次普通 S3 单 bar 与已提交原件恢复域。13 个检查组通过，新增事实异常 0 项。** 两次原 harness FAIL 均保留，不计通过；本轴未重跑服务、build 或测试矩阵。

| 检查组 | 独立核验结果 |
|---|---|
| 包完整性 | 402 个 manifest 条目全部实读，bytes/SHA 一致；RESULT 与 MANIFEST 指定 SHA 一致。 |
| 候选绑定 | 当前 final/Rust tree 与冻结 binary 一致；只核 6 个 changed 当前源码均逐字等于 final commit，不重复扫描 574 份源码。 |
| 旧原件 | 95 项 BEFORE/FINAL 完全一致；当前 live SHA、bytes、mtime、inode、device、UID、mode 再核一致。未打开旧 SQLite。 |
| 实际进程身份 | A/B/C epoch1 与 C epoch2 的原始 ps/lsof、完整 argv、PID/UID、socket 身份、nonce、0600 私有资源原件及规范 digest 四组逐项对拍。15 个 child 原始 launch argv 同终态记录一致，全部显式 `/3`，共 5 init、9 serve、1 recover。 |
| 实际 wire | 19 个完整回复逐字等于独立重算 canonical JSON + LF；其中 16 个公共信封核 exact 11 字段、payload SHA、请求 causal、reply ID、session/generation、S producer/source namespace/epochs；3 个为 transport 错误。Ready nonce 位于已 hash payload，clock/profile hash 与实际所选原件相同。 |
| A/B 普通路径 | 两个新库原始 ingest 请求与冻结 input-1 相同；各 original、receipt、idempotent 的所有 Committed 字段严格等于冻结 CLI ingest.stdout。各 1 次 attempt、10/11/12 clock，幂等及最终逻辑快照不变。 |
| C 同域恢复 | C 原 input 与 A/B 相同，但 lifecycle clock 不同；只同 C 首次 Committed 比较。recover PID 77641 实际 exit 0，clock13、epoch history 1→2；仅 meta、s_clock_events、s_protocol_meta、writer_epoch_history 四表变化，生成/原消息/发布原件不变。 |
| C 旧 epoch 拒绝 | epoch1 serve PID 77656 实际 exit 1；旧 input 明确 StaleWriter，stale start/input/receipt/final 的逻辑 tables 与 recover 后一致。epoch2 receipt 全字段等于 C 原始 Committed，attempt 仍 1。 |
| 私有 SHA 三负例 | PID 72568/72570/72572 各实际 exit 1，分别有摘要不匹配、小写 SHA 格式错误、非 UTF-8 明确 stderr；stdout 为空，DB/WAL/SHM 原始 BEFORE/AFTER 字典完全相同，无 writer lock/socket。当前 pinned 源码保证拒绝位于 clock/profile/lock/db/listen 前。 |
| 实际资源边界 | A 的 4096 字节帧成功 Ready；4097 字节拒绝。真正无 LF 请求约 1005.481667 ms 超时，第二连接在 max_connections=1 时 EOF 拒绝；边界前后逻辑 tables 不变。 |
| 终态与信号 | 15 个 child 均有真实 terminal returncode，当前逐 PID 的 ps 未发现仍在运行或 PID 复用；相关服务 reaped 原件实核。A 72576=-15，B 72645=-2，C 首轮失败善后 72679=-15，C epoch2 77676=-15；没有改写为 0。 |
| 失败链与续接 | FAILED-RUN-1/2 的 status 仍 FAIL，第二轮 A/B 通过事实保留；C 同一已停止 DB 经 resume_c.py 续接，不重新 init/换 input ID。原 b86 PREPARED-HOLD binary 绑定仍为 ab54…；现有封存与旧评审不改。 |
| 清理与快照 | 自有短 socket 路径及目录现场不存在，runner PID 77619 当前不存在；17 份逻辑 snapshot 的整 tables 及逐表 rows SHA 独立重算一致。没有连接 SQLite，避免 observer 再建 sidecar。 |

C 的原 clock 规范 hash 为 `9f73852e…8bc9b9`，A/B 为 `cd65fa6c…d1f31d`；其 index_frontier 分别为 `batch3-8a0f92df…dfd1ee` 和 `batch3-529bec7f…53ca13`。已核源码的 prepared metadata 承载 clock_plan_hash，根不同不构成同域不一致，不能拿新 clock 的 C 根与旧 clock 的 CLI 根硬对拍。C receipt 始终与自身首次 Committed 严格相同。

两次失败的射程保留：首轮 observer 的 mode-ro 打开自行创建 WAL/SHM，随后“sidecar 应不存在”的断言失败；wrongSHA 已真实早拒，但首轮没有完整 sidecar 无变化证据。第二轮是跨 clock 根比较器失败，不能抹成整轮 PASS；其余 C 步骤由原 DB 续接完成。session 99804 的 runner exit 0 来自父任务运行时结果，本轴独立核 STARTED/CLEANUP 原件和当前 PID，不把 session 句柄自行重跑。

未验证 Q、Browser、生产容量、严格 RSS 峰值、queue 实际饱和、write backpressure、response expiry、AcceptedPending 故障矩阵、条件 FIFO、9011/S16 扩域、整图或 main 集成。audit_cache_source_bytes 沿用已评审的跨请求源零保留结构与已有 fresh bad-node 动态锁，本次不据此宣称 RSS 零或全部七门峰值通过。

固定证据：

- final：`46d183a20afe2c68e01e26555531663a7810f065`
- Rust tree：`5a2acd1e72c3846be38cc29689a82aeb57c68dc6`
- binary SHA256：`5437fbb5d581d1b48f6a1d3405cae67327113e5a2c7fedd7ce3a1d9ce1e18e91`，18,609,984 bytes
- RESULT SHA256：`1c8f40e860dbe852af3386df8bdf6d9e2660601cca7e4a0f4d8463a3b08a4256`
- MANIFEST SHA256：`4d4c36ea248d2987212b01f728e6a7f3beee3ec60a0e31fc50c4af06383195c1`
- 私有 canonical resource SHA256：`ce36ec974ebc06089be51ff8d578c7686e28c0e8e9fd9f6634b8e5879b64fb07`

逐项机器结果、当前 6 文件 SHA、全部真实退出码见同目录 INDEPENDENT-FACT-REVIEW.json。本轴仅新增这两个报告文件，未改产品、旧报告、原件、main、push 或 tracker。
