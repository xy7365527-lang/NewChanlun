# DP68-v2 独立复审

结论为 **Approve，限本版撤销传播修复和所列控制域**。v1 独评的 MEDIUM 在该域结清，没有新的必修项。接受的是实际暂定端点撤销、已有版本重建、稳定身份核对及拒绝保留状态。真实行情中的 active 替换、通用候选覆盖和原义资格没有因此通过。

研究票 #1467。本审阅使用新上下文，只读冻结作者包、v1 独评及其输入；未读 MEMORY、ResearchProgress 或作者会话。全部新增产物在 `stage68/pen-v2-review/`。未改作者包、仓库、正本、生产、formal 或 tracker。

## 冻结身份及原结果保留

作者 FINAL SHA256 为 `07394ba791fec2413c83a46ff8aa0cd718572b498f3ff1a97b3d0539980d39aa`，manifest SHA256 为 `50e04c6534fdbd805d2a590c0544730c1331d688958985e6b8a3b294c357543f`，均与委派冻结值相同。v1 的 27 个清单项目和 v2 的 29 个清单项目逐项核对长度、哈希，无不符。

另做了一次既有 443 前缀实际回放。11 份核心文件与 v1、v2 冻结产物三方逐字节相同，见 [replay-equality.json](replay-equality.json)。这保留了已由 v1 独评核实的 441 版本、111 候选族、110 次 active 相认、109 次 stable 相认及 330 次端点版本替代。330 次没有改称逻辑笔淘汰；真实 active 替换仍为 0。

109 份受保护输入的前后哈希、长度完全相同，包含两作者版本、v1 独评、修订指针、实际回放输入及本轮查阅正本。见 [inputhash-before.json](inputhash-before.json) 和 [inputhash-after.json](inputhash-after.json)。

## 独立期望与接口结果

[independent_review.py](independent_review.py) 执行被审 `Machine`，没有调用作者 `state_control()` 或 `main()`，没有采用作者自检输出作为期望。对象初始字段及首次创建时钟从已审 prefix 合同独立构造；稳定时钟取输入 prefix 首见该笔进入 stable 的切点；冻结力度直接用源观察的价格、时间算精确分数。字段期望与实际完整对象均保存在 [independent-control-results.json](independent-control-results.json)。

作者八条义务在本轮拆为十条消息，增加首次写入冻结值之前的末笔错配检查。所有消息在唯一主回放捕获的 t122、t138 副本上执行。

| 消息 | 独立期望 | 结果 |
|---:|---|---|
| 1 | 撤销 t122 的 117→121。raw5/P0 的 end 置 null，实际存在的 endpointKnownAt、terminalCandidate 删除；start 和首次候选时钟保留，历史快照完整 | 通过 |
| 2 | 通知未由历史 proposal 表示的 117→122，仅记 pendingTerminalPen 和时钟；end 仍 null，版本字典及 lookup 不增项 | 通过 |
| 3 | 撤销消息 2，删除确实存在的两个 pending 字段；清 unrepresented 和当前 active 引用 | 通过 |
| 4 | 重用 `DP:117@118:v121`。raw5/P0 重建 end=121、endpointKnownAt=122，首见 confirmed 证书不改 | 通过 |
| 5 | 收到该笔在已审 prefix 中的实际 stable 首见时钟 126，稳定证书绑定既有 v121，不追加市场价格 | 通过 |
| 6 | 在 wholeFrozenL 尚不存在时，以错误末笔申请冻结，必须拒绝且所有状态不变 | 通过 |
| 7 | 以正确末笔冻结，raw5 的 L=−500，P0 的 cL=−500、wholeL=−1500，frozenAt=126；不得自填原始证齐时钟 | 通过 |
| 8 | t138 已证齐快照用错误末笔再次申请冻结，拒绝且保留 end、L、126 和 138 | 通过 |
| 9 | t138 撤销已 stable 的 117→121，拒绝且保留全部承诺 | 通过 |
| 10 | 经 step 入口提交 t139 的既有稳定前缀改写，在追加观察前拒绝，价格数量仍为139 | 通过 |

消息 6、8、9、10 均比较整个 `Machine.__dict__`，仅允许追加一条带正确原因的 `rejected-state-transition` 事件。原事件前缀及其他所有字段完整不变，检查范围不限于 end/L。稳定前缀拒绝事件保留 incoming/previous 证据。总共有 443 次有效 step 和 1 次在追加观察前拒绝的接口调用，没有第 444 个市场观察。

消息 1 的冻结/原始证字段原本不存在，本审阅只核其继续不存在，未称已清除了冻结数据。消息 3 则实际清除了 pendingTerminalPen 和 pendingTerminalKnownAt。真正已有的冻结与原始证字段来自真实 t138 快照，用于检查拒绝后保持不变。rawEvidencePending 和未获 stable 承诺的缓存清理在源码路径上存在，本轮没有用伪造缓存扩称动态覆盖。

主回放真实 t138 快照还逐字段匹配独立期望。其 P0/raw5 均保留端点121、冻结时钟126、原始证齐时钟138；t122 副本重建并冻结后没有获得138的证据。两副本价格数量始终为123和139。消息 2 只是一条接口身份通知，未声称117→122是合法 R_W active 或市场分型。

## 源码判读与声明边界

详细逐段记录见 [code-reading.md](code-reading.md)。`withdraw_active` 在修改当前集合前拒绝 stable 撤销，随后清 raw/whole 的实际引用；`bind_terminal` 每次核末笔完整元组；`freeze_span` 在任何冻结赋值前核版本、对象端点、当前末笔和 stable 证书；稳定前缀守卫位于追加价格之前。有限控制的结果与这些分支一致。

v2 的 `source-evidence.json` 与 v1 字节相同。继承 v1 对笔定义原文归属和有限几何的审阅，本轮只回查 `beichi.md:317`、`:563`、`:565`、`:569` 及第84课 `:52`、`:54`、`:56`。v2 没有把完成笔的端点/间隔要求推成所有进行笔当下都须满足，也没有把 waiting/null 当作原义反例。

`source_pointbar_bridge` 只表示 DP68 若要继承本版所引用的原文 K 线笔定义，仍欠对象域桥。它不构成对所有替代 F1 的新门槛。第84课允许别的递归起始定义，但并未因此批准本版 DP68；这两层均保留。`templateCurrentRawSlot` 仍是 primitive 笔槽，原始五笔段槽须整除5，不能当 Owner。

StructuralPenNow、每时刻原义 L、RootArm67、GeneralDiv、原义 Completed、Owner、Next、固定 F2 及完整 F1 均保持 null。FINAL、machine-readback、machine-status 的相关机器字段已核对，未发现误升级。

## 验证及剩余域

回放与控制命令 exit=0，stderr 为空；峰值 39,698,432 字节，即 37.86 MiB，低于96 MiB。回执见 [command-receipt.json](command-receipt.json)，汇总见 [independent-results.json](independent-results.json)。零新行情历史、零 R_W、零 Lean。

指定 R 工作树的 `git diff -- '*.py'` 为空，本轮代码差异位于外置冻结包，已直接比较 v1/v2。图工具未索引该项目/外置包，故读取指定文件。ruff、mypy、pylint、black 的可执行文件及模块均不可用，未安装；作者和审阅脚本 AST/不写 pyc 的 compile 通过。本轮没有把研究原型改造成生产模块。

本次通过不证明任意输入下的状态机总性、稳定但无候选表示分支、真实 active 替换、同类锚替换或通用市场正确性。单笔接口及末笔冻结检查依赖驱动提供的稳定前缀协议，不替代整段来源和任意错误输入的完整验证。未冻结的历史版本保持不回写，也不等于原义笔身份已经稳定。

**必修项：无。** 可接纳 v2 在上述有限控制域实际修复了 v1 的模板取消传播缺口；v1 冻结结果及更广的未证边界保持原身份。
