# #1394 完整历史正式路径前置事实与输入草稿

本工位只在指定 scratch 写备料。绑定 HEAD `ed1f184bf7513d960d87b6687c279a657c8fa481`、Rust tree `5a2acd1e72c3846be38cc29689a82aeb57c68dc6`。没有启动服务、执行 DataPath、build、修改价格/profile/资源原件、main/push/tracker 或目标。C 第5.1–5.9已由 SPEC-ADDENDUM.md 与 ADOPTION.json 采纳；APPROVED-SOURCE.md 的旧“待采纳”标题是历史文字，不再要求重复批准。

## 已完成的精确来源准备

原源9011条、1,006,026B，SHA `76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087`。冻结副本 `inputs/source.bars.frozen.json` 与原源逐字节相等，源文件前后bytes/SHA/mtime/inode未变。完整六个整数列均为JSON整数、i64内；所有OHLC几何正确；source_index严格0..9010，timestamp每步60秒。volume数值token全为1.0。完整统计和重复价格实例见 `inputs/statistics.json` 与四个 `*-same-price-source-instances.json`。

现在driver.rs:1123的诊断入口用 `bar[field].to_string()`直接生成价格，不含round或f64转i64。对这份原源没有改价前置。冻结原bar含volume1.0，canonical raw_text保留这个token，事件volume沿已经使用的具名单位成交量映射为字符串"1"。不得把raw_text改成整数volume后冒用原hash，也不得把另一份浮点价源round后混入本域。

已生成完整9011条草稿raw_input/envelope、逐消息payload/envelope/raw_text SHA和完整串行schedule。全部LF请求9,235,548B，最大单帧1,030B。full ClockPlan790,213B，SHA `b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01`。首16条与旧fresh1/fresh2保存的canonical_envelope 32次逐字节相等，对应旧clock16也逐字节相等。`prefix16-exact-byte-check.json`记录原索引hash。这里只复用原件，没有重跑16条。

全clock增添input17..9011，whole-clock SHA必然不同于旧16计划。正式探针A/B从一开始都冻结同一fullclock，首16只是该全输入的消费前缀。旧clock16根和新fullclock根的差异是合法上下文差异；不能要求旧batch/native根逐字相同。跨版本/clock须按C5.8做明列映射和完整逻辑字段检查。

## profile、身份及正式入口

固定profile `s_session/profiles/ohlc_integer_tb02a_v1.json`须完整等于编入binary的声明，load_profile在s_structure_session.rs:1630核此点。tick_size1、exact整数串、provided_ohlc、resample none、volume_unit1保留。tb02a.rs:48/154核输入exact形状与精确整数/几何，raw_input.rs:24核单位成交量并持久保存(namespace,epoch,instrument,event_id)身份。profile没有instrument白名单，TEST.LONG.TB03B不受TEST.OHLC名称限制；这只是接口允许，不是全源业务语义批准。

正式init_v3在mod.rs:342初始化CompleteCut/economic not_started、profile/catalog/rule/session/clock身份和authority/content/control表。ordinary S3 ingest没有要求先完成#1407。scope只说明结构域，不授予财务成功或native分类完整性。两新物理域A/B保留相同session_id s-session-testonly-001、generation1、writer_epoch1、source testonly.ohlc/1、producer driver-fixture/1、TEST.LONG.TB03B、同完整source/envelope顺序；DB/state/socket必须各自全新隔离。

当前driver测试Fixture在:417/434直接调用init_v3并Driver::open，不是kernel::Fixture，但仍在cfg(test)绕过Controller、正式init CLI、resource文件、serve_unix/Ready/PID与收据运输，retain_generations4是测试已用值；resource_profile=null。不能复制这个Fixture冒充正式CLI路径。

正式路线已存在。Controller.read_config显式s-launcher/3，start_s在s_service_control.py:551实际执行 `init --protocol-revision s-session/3`，继而 `serve --protocol-revision s-session/3 --resource-config ...`；freeze_s_resources核child实际资源SHA。wait_ready核nonce/PID/session/epoch。每条调用s_socket_client.exchange一次，validate_reply producer S epoch1核cause/native payload hash，随后核Committed和expected generation，才允许下一条。结束status→stop→wait/reap。已冻结binary路径/SHA和详细输入计划见 `inputs/formal-caller-plan.draft.json`；本工位未调用这些步骤。

## 同价事实须保留

每种OHLC列8,649个不同价格，362次额外出现。相邻high/low/close均无同价，open与前close相等是生成定义；非相邻重复值真实存在。这些计数不等同原始根/包含组/笔端点同价竞争oracle。原参数的“零包含合并”也不证明#1405全域无竞争。正式运行时若遇未裁适用对象须保留失败原件并精确归属，不改源、不删断言、不改严格比较。

## 最小正式前缀探针的资源提案

单bar已有七档为frame4096/read1000/write1000/response6000/queue1/connections1/audit_source1。它的原件没有改，不能把名字不变的档自动升级为S16。旧8f9树16条内部实测两次handle合计58.706013669/59.175779374s，最大10.259819042/10.389861625s，冷审计3.500323042/3.518640375s。它们没有formal service或resource profile，也不绑定新5a2树，不能作工期因果或9011预测。

提议新名称 `TestOnly frozen9011 fullclock prefix16 serial probe`，七值保留frame4096/read1000/write1000/queue1/connections1/audit_source1，response提议30000ms。30000是为了有限测量而明确选择的窗口，旧最大10.3899秒解释为什么不能用6000；它不是推导出的生产保证。单请求输入上界1030已精确冻结，但正式Committed响应是否<=4096尚须观察，失败须保原完整回复，不截断。原单bar档已测frame/read/connections只能提供这些机制证据，仍不能直接证明新负载可用。调用者总期限应明确冻结且不短于拟测operation窗口；Ready与启动30秒只沿原控制档，不承诺fullclock冷恢复可在其中完成。retain4是与旧输入测试一致的草稿选择，不能当9011全历史交付窗口。

根随后已接受此精确有限探针预算，见 `inputs/finite-probe-root-acceptance.json`。已冻结 exact `s-structure-resources/1` 资源文件 `inputs/TestOnly-frozen9011-fullclock-prefix16-serial.s-resources.json` 与 Controller-A/B.draft.json，仍未运行。资源SHA `b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69`。这项新范围决定已完成，无需重复批准；config中的新DB/state/socket路径仍须runtime owner在启动前核独占与不存在。原C接口不需要重批。跨请求audit源零保留是当前生命周期证明，cap1不是生成缓存、临时heap、RSS、DB/WAL预算。那些量分别计量。

## 全源义务与下一可执行动作

1. 先核这份input manifest与首16exactbytes，再由runtime owner采用已接受的具名fullclock+prefix16资源档，核两域草稿物理路径和实际测量方法。新树binary已绑定，无需重复build。将同一完整clock/schedule交给正式Controller，首16实测只校接口/截止点/成本；保存完整原生回复及失败，不包装成9011成功。
2. 用正式探针实际输入/回复尺寸、冷/暖审计与DB/WAL/IO/RSS采样，逐项判断哪道旧门足够、哪项新输入仍缺。饱和queue/write/response和严格峰值仍需具名边界试验；离散采样仅给观察下界。无需等待frontend1460才能做这些后台工作。
3. 同时备齐C5.8全字段crosswalk、旧原native/hash与新原native/hash、来源revision/顺序/目录/first_known/withdrawn/Delta，以及原#1392/#1405适用oracle。现9011source虽已全冻结，旧classifier双call并非S3完整双跑控制。缺少旧全历史对应证据时按具体对象登记，不能降低源量词。
4. 完整9011新S3两进程同输入/clock/order双跑、Q原capture/verify门与全部content/consumer/job、S→E/B完整闭包/三阶段/六故障，仍按C5.9承担。所有逻辑比较/构造/Hash/audit/唯一依赖/落盘/RSS/DB/WAL/network/job与allocated峰值均须实测。全source首批Q/client/financial数值不能从16线性推导或套用单bar档。
5. 全源资源与执行路线备齐后根继续当前fullgoal；其它TB、六Destination、main/push/真实外效的原批准门不变。本报告是已批准#1394范围内备料完成，不把所有未知概括成全局blocked，也不新开票。

完整JSON报告列明approved_choices、pure_facts_completed、true_new_choices和missing_obligations。所有新数据文件是draft输入，未产生服务达标或新语义采纳。
