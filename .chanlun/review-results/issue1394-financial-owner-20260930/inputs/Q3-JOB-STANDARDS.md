# #1394 C 纯累计 Job：revision2 独立工程复评

结论：**PASS**。固定 `b1a8d8d45c5dacd1245e2f33f20f7d514ae2446e` 的原 ENG-1、ENG-2、ENG-3 全部关闭；完整复查两文件修订差异及当前实现后，没有其余置信度超过 80% 的工程发现。本结论只批准纯 Job 单元，不批准消费者接线、物理分配或产品合入。

## 固定对象与概念层

| 对象 | SHA256／commit |
|---|---|
| parent | `61acbd43babe28397265e530b59c56342706547e` |
| candidate | `b1a8d8d45c5dacd1245e2f33f20f7d514ae2446e` |
| `s_session/browser/s-content-job-v3.js` | `df8c2a3c9864fd2290e1c929dfb5d9cdad799539cca585e424440d29776b402c` |
| `s_session/tests/test_s_content_job_v3.cjs` | `642ff3efe62a43518dd187420508ce38bd8f6226bc33e52aef51992674e1cba4` |
| 固定内容核心 | `483d2c175b44320944112696a755462e0ece40d8b90f64214224bb72f5a7334e` |
| `APPROVED-SOURCE.md` | `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30` |
| 本修订独立 Spec PASS 报告 | `3e1b3e01ee4b0dfbba3ae88ccf19cb19d349d2c1bea64667204711c939fd182e` |

概念层通过。批准源 §5.6.1 的累计、绝对期限、空间子集和固定配置含义不变；既有 Rust Job 只用于解释账目与停止语义。callback 内重入在产生变化前永久拒绝，未放宽资源合同，也未新增物理存储定义。

本工位图未索引的结论沿用，直接读取已指定文件。已核 HEAD、staged／unstaged diff 和 parent→candidate 全量两文件差异；工作树没有受审文件的未提交修改。固定内容核心不重审。

## 原发现逐项闭合

| 发现 | 新源码位置 | 独立复核结果 |
|---|---|---|
| ENG-1 HIGH，clock 重入绕过取消／finish | `:82–112`、`:126–155`、`:195–222` | 11 个入口 callback cancel 均拒绝；节点与新 pending/open 均为 0，已经发生的 actual=7 保留。finish callback 重入 reserveStore 在创建预留前报 job_clock，open=0。回调吞掉重入异常仍不可安装；回调结束后合法清理不受禁令阻挡。 |
| ENG-2 HIGH，clock throw 跳过 cap／永久停止 | `:89–101`、`:126–135` | 三个 actual 入口 cap=9，3+7=10 均永久 job_clock；恢复时钟后 preIoCheck／finishCheck 仍拒绝。finally 清除回调标记，停止后旧 grant 可 reconcile/release，已对账 staged 可 reclaim。 |
| ENG-3 MEDIUM，schema 双读替换未验证对象 | `:24–53`、`:65–70`、`:140–146` | 字段单次捕获后只验证副本。旧 schema getter 反例输出合法字符串、读数=1；所有 limits／existing 字段也只读一次。配置与快照无调用方可变对象引用。 |

原独立工程六组复制到本 revision 复跑。唯一行为断言调整明确保留：原 finish 重入反例期待创建 open 后报 job_reservation，新合同在创建 open 前报 job_clock；新断言同时要求 open=0、pending=0。这是更早拒绝相同非法操作，未将成功条件放宽。旧红探针与输出不改。

## 实际检查

执行：`node .scratch/issue1394-q3-client-job-20261001/revision2/STANDARDS-probes.cjs`。入口强制核源码 SHA，**9/9 PASS，exit 0**。见 [探针](STANDARDS-probes.cjs)、[原始结果](STANDARDS-PROBES.json)。

新增三组补足工程余量：

1. clock 抛出的对象带恶意 code/message/stack/toString/Symbol.toPrimitive getter，均未被读取。六种场景核首次原因：原 live 变 job_clock；既有 cancelled／limit／reconcile／deadline／clock 原因均保留。随后真实清理正常，actual 不丢失。
2. clock 内重入 async acceptNode，在 Node 和无 Node globals 的 Browser VM／WebCrypto 环境中，Promise settle 前已永久停止；吞掉异步拒绝不能接纳节点或恢复 finish，wire=10、nodes=0。
3. 八个 limits 加两个 existing getter 全部只读一次；以 `2^63-1` 的精确文本核最大基线与 cap，reserve 失败不改变两账 peak/open。

复用相同源码与测试字节的独立 Spec **14/14**、单测 **30/30**、两文件 syntax 与 parent diff 检查，不重复执行。已核其证据涵盖异步晚到取消、坏 hash 不能走 dedup、节点双 cap、>u64 永久累计、首次起钟、空间原子预留、opaque handle、stopped 清理、getter 类型域与单读。完整工程读检查未发现安全、兼容、性能或错误处理方面的其余明确缺陷；未按行数、风格或 test 个数判定通过。

| 严重级别 | 未解决数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | pass |
| LOW | 0 | note |

## 保留证据与边界

旧工程报告 SHA256 `423571c40a5b2c818f8ea26204fd78b2b040b91254b49dcbcac1ece5a868fad1`，旧红输出 SHA256 `93d42727390d2d6a3a6a9e6668d2c2c3a3d22fb3f4e5615a36ebf1e9cb45ce8b`，复核前后保持。只新增本 revision 的工程报告、JSON、探针与输出，没有修改实现、测试、原 Spec 或旧红证据。

仍未证明实际 Browser 消费者、existing 实测、物理 allocation 域、资源 profile、操作 epoch／在途 I/O 结算、完整语义或安装。Browser VM 只是同模块兼容与回调探针，不是真实网页验收。未启动 Browser、服务、full cargo、旧 economic、9011；未改资源、原件、main 或 push。后续消费者接线须以本固定 SHA 的两轴结果为单元证据，再按其自身范围验收。
