# #1394 入口差异独评实体

这是当前 Codex 根任务归档的独立原生评审结果，不是实施者自评。评审均未修改被审产品；实体绑定 open #1394，不替代整个增补联合验收。

| 工位及固定覆盖 | 实际结果 |
|---|---|
| `finance_transport_exact_review`，`30f0f9e6 → 0b59b46b1f` | Standards PASS；Spec FAIL，HIGH：真实严格 JSON 解析越过更早操作期限仍返回 OK。真实 Job/解析器临时调用器两次 60 ms 限下 76/77 ms OK，另 150 ms 限下 227 ms OK |
| `s3_writer_exact_review`，`30f0f9e6 → c27023c205` | Spec FAIL：原持久身份冲突在隐式 pending 提交后才拒绝；Standards FAIL：资源文件 FIFO 打开可阻塞。另指出基线 epoch 被误当 volume_unit |
| `s3_financial_exact_review`，`1a2d852cb4 → 508fbac879` | Spec PASS，限定生产者；Standards FAIL：非普通 DB 路径即完整输入 EOF 后仍阻塞，真实两个 CLI 已复现。basis、完整 wire 承诺、专用 anchor、独立证明及只读 overlay 未发现语义矛盾 |
| `interface_fixes_tail_review`，`508fbac879 → a20bdee2e0` | Spec PASS；三个初评门与 volume 修正闭合；财务静态非法路径闭合，但 stdin 期间替换 FIFO 仍 2/2 阻塞，保留 MEDIUM／Standards WARNING |
| 同一独立尾审，`a20bdee2e0 → 9611a33a9c` | 概念层／Spec／Standards PASS；只读连接在 stdin 前打开，EOF 后身份核验、短事务；FIFO 和普通文件替换各 2/2 拒绝，静态 12/12，语义 3/3，真实 CLI 1/1。合法同库写入不阻塞且读新 cut |
| 同一独立尾审，`9611a33a9c → bdc50d324a` | 概念层／Spec／Standards PASS；新增混合 input profile/schema HIGH 闭合。共享纯 helper 逐字保持旧两个条件、错误文本及成功域；S3 在 raw/message/clock 写入前拒绝，反例与旧入口各 1/1 |

最终差异可组合复用上述覆盖：末尾只变原已声明的补修面，生产者、运输、writer 三职责的初评及修复结果没有被新的未审代码替换。根工位自行核 cherry-pick 后的产品文件与对应隔离工位 blob 相同；共同 `mod.rs` 只合并两条独立分派，已由财务初评覆盖。

未验证范围保留：S3 serve 缺启动资格；E/B v2 消费、全历史引用闭包及经济三阶段未接；generation=0、真实跨 epoch 财务、全 9011、资源峰值／部署、远端 CI／新扫描未成立。现有业务拒绝仍可在独立合法旧 pending 恢复后发生，本轮不扩大为全操作原子回滚保证。初次文件类型检查至实际打开之间的任意敌对替换、同步 OS spawn 无限阻塞不在本组通过声明内，调用方绝对期限仍必须生效。

原始失败与补修测试、真实 CLI、Sandcastle 状态汇总保留在 [EVIDENCE.md](EVIDENCE.md) 与 [RESULTS.json](RESULTS.json)。没有以一次 canary、组件绿灯或归档提交关闭 #1394／#1323。
