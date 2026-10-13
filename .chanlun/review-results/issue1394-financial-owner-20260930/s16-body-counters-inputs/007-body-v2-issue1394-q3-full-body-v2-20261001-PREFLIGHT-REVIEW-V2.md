# #1394 S16 全正文 v2 归档载体独立预运行审查

**结论：PASS，仅放行一次正式 Q21 实跑。** v2 尚无 `RESULT.json` 或 `collector-terminal.json`，所以这里没有新 HTTP、全正文、来源闭包或 Client 成功的断言。Root 执行前须重核下列源码、HEAD、Q21、当前 binary、源 DB/WAL/SHM 无持有者及私有目录空间；实跑后同时要求 runner 的 `PASS_JS_FULL_BODY_PROOF`、清理收据和独立 oracle `--actual` 的 `PASS_INDEPENDENT_S16_BODY_SOURCE_HTTP`，否则保持失败/未验证。

## 冻结绑定

| 对象 | SHA-256 |
|---|---|
| `run.py` | `ba98fed85f49597e3d9bb75fe9b4ac4dd27e867589998c1a74bf3ec5fcd363e8` |
| `collect.js` | `672fcba484d257bd41079c21af17329519d51a9d910d008c61baf11885886691` |
| `archive_v2.py` | `e19c40890697765174b126b6e0db14d03745e27e8e1d2670efdc6db41d65418e` |
| `selftest_matrix.py` | `f73e02c68145dab4794e64f42048f85646c82e709aaf2d6c376baa17ad16dcee` |
| `independent_oracle_v2.py` | `c8fd1838f7745e5494d84e47c86b165623cf1aeb00db3dc75e39f34b2e840f7d` |
| `FORMAT.md` | `7f66e7155b00827206eb41592ccdfe0bafd995714fa227f70d15af3e33fc85ee` |

Root HEAD `8fb179091a81e274214056603171b759decfdd08`、Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`，当前非 test release binary 固定 `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`。原 Q21 SHA `50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`，21 值未调大。旧 v1 `FAIL_PRIVATE_BUDGET` 和缺尾 gzip 原件保持失败；1835 条只读前缀仅证明 Q3R2 原字节往返，不替代新实跑。

## 核过的契约

- 写者为每个 Content 请求保存 Q3R2 大端长度、CRC32、请求/响应双 SHA 和完整原始 wire；每块至多 256 条且不超过 16 MiB。zstd 编码后立即解码逐字节核对，完整写入 `.part`、fsync、rename、目录 fsync，manifest 完整写入并 fsync。读取器拒绝缺块、多块、残留 `.part`、缺尾 LF、短帧/坏哈希、非规范字段、布尔冒充整数、非法完整/半包 metadata。
- 每次 HTTP 发出前，写者按未刷帧、Q21 最大单响应、记录开销和 2 MiB 收尾余量核 50 MiB 私有调度上限，必要时先冲刷或在发请求前失败。SIGTERM 销毁活动 socket，归档实际已收到的半包，再冲刷末帧；父进程只对自有 Node Popen 做 TERM、至多 180 秒等待、身份核对后的 KILL/reap。完整 503 可作为完整 wire 证据，但业务仍失败。同步行摘要写入异常也走 archive.close/非零退出。
- `run.py` 仍先核固定 source 三件套、fresh owned 副本、24 表/48,403 行/96,397 BLOB、47,671 native F；正式 Q 的 16 页、472 描述符、全 projection 和六族正文由原 Reader/GraphView 路径处理，归档节省的是证据落盘体积，并未删请求或 proof。v2 oracle 的 39 个旧共同定义中，仅 `actual_check` 和入口 `main` 变动；新增独立 Q3R2 解析，不调用生产归档解析器自证，并保留 native/source、catalog、逐记录、range/token/原字节对拍。

## 验证与限制

Python 四文件 AST、Node `--check` 通过。本次读取无 Q 测试收据：Node→Python 完整原字节、活动 socket SIGTERM 半包及末帧、同 Popen TERM/KILL、short write、行摘要 EIO、完整 503 业务失败、预算前置拒绝均通过；13 种格式损坏被拒绝。相关测试 PID 已不存活。独立 oracle 以冻结源码重验 47,671 native 节点与旧失败前缀 1,835 条原字节，结果分别为 `PASS_NATIVE_F_AND_SOURCE` 和 `PASS_Q3R2_PREFIX_BYTES_ONLY`。这两个结果均不是新 S16 HTTP 成功。

**异常边界**：若 zstd 子进程自身失败/超时、fsync/EIO 或宿主可用空间突然变化，未提交的内存末帧仍可能无法保存；现有已 fsync 块可读，但 collector/runner 必须失败，不能报告正文闭包 PASS。该边界不改变 Q21 或 Client 资源/物理分配结论。Client 原子安装、实际物理 allocation、完整 9011 和 #1460 均未测，allocation 必须写 `UNMEASURED`，不能填 0。
