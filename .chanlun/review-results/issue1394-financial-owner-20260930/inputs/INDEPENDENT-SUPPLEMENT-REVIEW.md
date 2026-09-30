# #1394 独立补充评审

只读固定结果审查，未参与这三份实现/运行，未重跑案例。规范沿已采纳C的5.6.1/5.7/5.9及SPEC-ADDENDUM；工程沿AGENTS的同判据纪律、CONTEXT职责边界和交付纪律第7条。此文件仅为限定补审，不授权main/push/关票或#1394/goal完成。

## Spec

结论 PASS，限定为下列实际证据。未发现这三份固定结果内需整改的HIGH/MEDIUM/LOW事实或实现缺陷。

冷审计固定base909ccff→commit1836d3c38e9036a193322e1a47634097ffb17306，差异为state/driver两文件308行，仅cfg(test)。SHA256SUMS五项、两份数据库摘要、PATCH与git固定diff及stdout→evidence→RESULTS逐层一致。bindings的reported_code_revision为生产base909ccff；实际include_bytes(state/driver)摘要对应1836测试源码，不能把该env自报当唯一版本证据。

memo不是伪造enabled常量：state.rs:319–323调用真实旧cache::configure，cache.rs:633–642实际写thread-local Memo.limit并读取limit!=0，容量1714/1708来自实际两输入规范字节。每阶段对真实旧memo留存、capture与全counter前后相同且为零；同连接drop后、新query-only连接、重开driver均实际fallback读取SQLite。state/driver成功ready阶段各有114094 source_reads；翻转已发布snapshot节点body后均实际node_digest失败，恢复原body后再读成功。没有额外“存入合法memo并命中”的正控制，故本锁证明正配置下S3未使用该旧memo、当次回读和损坏拒绝；不证明所有容量的memo可用性。输入/raw、control SQL、生成器TxStore、HashReader的派生overlay和F调用计数是不同LANE；overlay非零、node_checks=0及F调用不按唯一节点解释。生成器/RSS/峰值、七writer字段限额映射、实际serve及全9011均明确未证明。

来源漂移只取attempt4 current产品；attempt1配置错误、attempt3旧产品快照不能给当前产品结论。manifest内24个具名原件bytes/SHA均匹配。仅删400/1307个未引用setup-cache节点，真实S在staging300窗口advance到新basis；对象hash仍合法，实际Prepared delta1绑定新root/commit_ns22，不把合法重读后成功当成应拒绝案例。另一份新域的显式第二次read_current复用同一个Job与同一Instant截止：requests197→198、wire6577398→6615805、records900→918、elapsed52301→52833ms。两次均未过期，因此证明累计和未重置绝对期限，不证明跨期限拒绝。第二次显式调用也不冒作内部retry计量的逐次trace。

重要边界：经济成功write后staging=21，reader-only路径staging=1161，均在recover后才为0。不得把该结果写为“成功提交立即stage0”或“所有未引用暂存即时释放已证明”；它也不是全量冷导入、正式E/B Unix或全部source-change场景的验收。

正式Q HTTP清单72个文件bytes/SHA、ZIP CRC及ZIP内72份同字节原件匹配。ZIP内BINDING及v2/v3脚本SHA与各fresh-case来源一致；ROOT只给原RESULT添加path/files/script绑定，未改原有语义字段。正式query-serve argv、启动公告、资源配置SHA、binary/source-input/runtime-DB摘要吻合。SIGTERM/SIGINT两个新进程分别完成7页/7行snapshot、独立重算projection/qcut、37个返回node帧Hash与Watch1，退出0且reap后端口可重新绑定。脚本发送信号后才wait；无forced-kill成功化。此为已完成请求后的有界teardown，不是inflight中止/读恢复。元数据重算不导入产品reducer，但不核所有数字token/全部内容语义，不等于controller/browser、完整consumer安装或全资源执法，原件的false边界字段保留。

## Standards

结论 PASS。冷审计助手由两个真实测试复用，cfg(test)不进入release；保留Fowler12项为启发式，未发现需整改的命名、重复、职责、类型、泛化或继承问题，不报告工具已强制的格式。readonly审计与损坏仅作用于该测试私有DB，原body恢复后归档；archive的TRUNCATE在测量阶段结束之后，明示非peak/实际wire值。新的create_new证据目录、固定SOURCE/hash和错误/失败attempt保留，没有宽松判据接管严格失败。

source-drift的合法Prepared与来源拒绝属于合同允许的不同结果；实际attempt4只证明前者，未把其它attempt错误算当前产品反例。same-Job reader始终用同一budget与deadline，没有靠新Job重置余额。HTTP原件同时保留tag十进制误读/TIME_WAIT和不完整公告行的测试器失败；后续只修测试器与新实例，未假称产品修复或放宽资源。所有root报告的FULL/consumer/inflight限制与实际脚本相符。

## 固定原件库存与未完成

| 对象 | SHA256 |
|---|---|
| cold RESULTS.json | `b716dc67d6fd1ae7d220675bb1b61fc1f5d8cf6aa492c73a3c7d6a6496f5cad6` |
| cold PATCH.diff | `49981b638e2a42d40527e9a09599ea96b344e32639303e5f41db04284f25fca0` |
| SOURCE-DRIFT-ACCEPTANCE-MANIFEST.json | `6e4264ba8a6b682794ca5445dcef111c3c0d77be5b742f3850a832835e34e2e1` |
| ROOT-HTTP-EVIDENCE.json | `b3a4fdab4a8549560e6597ae4a67da964751f6ece6d9918a9593ecde7bf56dde` |
| RAW-HTTP-EVIDENCE.zip | `9d259b7626be874e0216458ea41320432d0cfe1b6cba3a5a07eb02bbc6200d2e` |

冷审计原件目录：`/Users/silencehan/Projects/NewChanlun-1394-cold-audit-20260930/rust/.scratch/issue1394-cold-audit-20260930`。来源漂移原件由manifest的attempt4/frozen_inputs/build_binding定位；正式HTTP原件由ROOT清单与ZIP定位。已核固定源码差异及现有日志，不运行新测试、不扫全SPEC/全DevSkim、不修改任何源码或原件。#1394联合/生产、六Destination、真实浏览器与9011仍不由本文件结清。
