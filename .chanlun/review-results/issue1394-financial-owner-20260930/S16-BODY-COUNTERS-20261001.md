# #1394 S16 完整正文与同材料审计计数增补

本文件是 open #1394 的工作草稿，归总图 #1323、SPEC #1340 已批准 C 增补。本轮完成 S16 完整正文与来源 HTTP 验收，以及同机动态审计计数和收益核验。只在隔离集成分支归档，未合入 main、未推送；完整 9011、C 全部验收和六项 Destination 仍未完成。#1460 前端是独立图。

生产源码仍为候选 `c6255d308fc69d94178916a1dc5534546fb77591`，Root Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`，五份生产文件字节与已审候选相同。正式 Q 使用现成非 test release `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`，绑定原 patch、源码和产物；构建时 Git HEAD 的显示标签不单独充当产物溯源证据。计数观测采用独立 test binary `5f9ef56e7cd0ee27dd00094faaf5c73821d6501fe38b4783c238c67d80cb286f`，诊断插桩未进入 Root 生产源码。

正式 S16 运行得到 `PASS_JS_FULL_BODY_PROOF`，独立原始 HTTP、native/source 和正文 oracle 得到 `PASS_INDEPENDENT_S16_BODY_SOURCE_HTTP`。16 页共 472 条六族正文，objects 65、withdrawn objects 82、witnesses 81、relations 199、observations 29、raw history 16。全部 5,538 个 Content 请求原始 wire、范围、路径、单位、token/cursor、每个 F、完整 DAG 可达性及全部正文 SHA/bytes 通过核验。投影正文为 1,253,831 B，SHA-256 `ec1465f6c20af978e23d8eea7fc6b7ecfb0a0ff8683202e9e5cf2e95e025565a`，根为 `sc1-460b659a1fd449f701c5c6f30e20a1a6b07343661d54443dd01f03298918b314`。

唯一 proof 节点 26,685 个，native 15,709 个、Q 临时构造 10,976 个；未要求临时 proof 预先持久化。响应 body 共 171,692,298 B、响应 wire 共 172,428,851 B。Q3R2 24 个独立 zstd 块保存请求与响应共 182,847,013 B 原始 wire，压缩后 4,261,967 B，complete 5,538、partial 0。私有观察目录最终 st_blocks 分配 27,660,288 B，通过原 50 MiB 观察门。原 Q21 的全部 21 值保持不变。

独立 oracle 另核 native 47,671 个 F、SQL catalog 116 项及 axis 13 项、16 条 raw revision 链、source coordinate 5,444 项与 source reference 2,086 项、414 项 JSON_TEXT 和 81 组 witness bars。CC006 指定公开 revision/seq/merged_index 的原 number token 转十进制字符串共有 1,553 次，严格限于声明 schema。catalog 元数据仍为 implemented 13 / not_implemented 103、run 12 / not_run 104，不能据此宣布全部结构已实现。

旧 v1 的 `FAIL_PRIVATE_BUDGET` 原件保留。1836 次 HTTP200 后触发 50 MiB 门，缺尾 gzip 只有 1835 条完整可读记录，末条原 wire 未声称留存。v2 仅改变无损证据载体，没有放宽 Q21、减少读取或改写旧失败。正式 Q PID 1135 和 Node PID 1278 已 exit0 并由同 Popen 回收；Controller close、端口重绑、原 source 三件套不变和 owned source 全表一致均有清理收据。

同机动态 audit 的旧版和候选各运行两次，`LOCAL-REPORT` 为 PASS，独立事实核验为 PASS_BOUNDED。每轮 F 从 4,406,786 次降至 1,386,860 次，少 3,019,926 次，恰等于 reuse hits，下降 68.529%。实际 found SQL read 仍为 3,082,463 次、unique digest 35,272 个。miss 62,537、mismatch 0、三类 admission reject 均为 0；retained 观测峰为 35,272 entries / 2,025,582 B。每轮 Public/Template 成功根 32/16，clock parse 一次核 9011 events，stored clock 一次。核整份 clock 不等于执行全部历史。

| 配对 | 旧 audit 秒 | 候选 audit 秒 | 减少 |
|---|---:|---:|---:|
| 1 | 4.117122791 | 3.803846416 | 7.61% |
| 2 | 4.112855125 | 3.769924541 | 8.34% |

两轮完整返回四成员、16 个 cut 各五成员、578 个阶段 marker 顺序一致，返回指纹 `1de8cc89baf383fc18db84c1ed68bec633fa4887ce21f9b1fc2aa9ce13b6659f`。完整 24 表、48,403 行、96,397 个 BLOB 与正式 A 相同。原 A 三件与原 589 个 target 的前后清单保持；独立评审没有重读全部 589 当前文件，仅独立核前后清单投影，范围照实保留。owned clone 的 SHM 仅允许 mtime 漂移，bytes/dev/inode/SHA 严格，DB/WAL mtime 及全部原件仍严格。首次 audit 前因旧 SHM mtime 门安全停止的失败原件保留。构建、baseline 与 candidate 子进程均退出回收。

audit 的 RSS 采样最大值为旧版 66,043,904 B、候选 75,497,472 B，只是 0.1 秒采样值。JS 末时 RSS 894,681,088 B、heapUsed 561,197,432 B，不是严格峰值。最慢整 HTTP 为 4737.709ms，不是内部 verify 独占时长。retained capacity 和观察目录 st_blocks 均不是全进程 heap/RSS 上界，也不证明 Client 实际物理 allocation。Client 原子安装、physical allocation、JS 堆硬限保持 NOT_ATTEMPTED/UNMEASURED。

旧 92+24+33+43 共 192 inputs 原样保留，本轮 76 份小型冻结脚本、报告、绑定及清理证据位于 `s16-body-counters-inputs/`。大型 DB、binary、完整原 wire 块与其他观察原件仍为 caller-owned；同名 JSON 列出 500 个完整路径、大小与 SHA-256。带硬换行或 patch 上下文空白的文本封入 `.raw.json` 的 `content_utf8`，解码后 SHA 与原件相同，不修剪冻结字节。归档机械重核未重复构建、源码测试、HTTP 或 audit。

接下来准备当前源码的全 9011 正式 A/B runner、具名 S 资源与 caller 期限、两套 fresh 独占物理域及完整逻辑/资源验收。已有 runner 写死首 16 条，旧有限资源档和先前源码二进制不能自动取得全量资格。本机约 11 GB 空闲也不能从 S16 推导全量容量或时间；不依赖已挂起的外盘。随后仍须完成旧新 crosswalk、全负载 Q/E/B、适用严格峰值和原 TB。Client 实际分配口径的未回答选择只阻塞依赖该选择的实施与成功声明，不阻塞独立 S/Q/crosswalk 准备。当前结果没有收束 C 或关总图。
