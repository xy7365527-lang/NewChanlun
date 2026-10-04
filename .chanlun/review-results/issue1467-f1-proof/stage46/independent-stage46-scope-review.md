# #1467 Stage46 汇总与订正独立复核

2026-10-04。复核者为新的独立上下文，未参与作者稿、转折来源独评、DirectHull 冷读或数学对应比较。只读指定研究树，唯一新增交付为本仓外文件。

结论：通过本次汇总与订正复核。未发现阻断问题，也未发现指定的已知非阻断订正遗漏。通过对象是两份文件合读时的限定研究口径，不是原义 F₁/F₂、EXT-own、市场有效性或生产采用资格。

## 输入身份与方法

研究包根为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/`。以下短文件名均相对此根，`stage46/` 下文件省略该前缀。实际全文阅读了汇总、订正及指定六份报告；另读 DirectHull 声明与证明、MovingQuote 的 Flow/QuoteOf/Step/leg/readout/locate 及 construct 小声明。没有另开原文、候选或市场研究。

两份目标的当前 SHA-256 与派发指定值完全相同：

- `Stage46TurnBoundaryAndDirectHull.md`：`7559aa5a1faef0d19e0049f3e15da1415b09831dce87a676067016c2b3176152`。
- `stage46/ScopeClarifications-v1.md`：`8a44e4afb6930f68b5e687e03aa03ef88f9089316b460c176453acac8dfe18a0`。

本次复用既有 Lean 与数值通过证据，没有重新编译、重跑数值探针或全量枚举。对两份大型 run-manifest 只经 JSON 提取相关身份、semantic、root_closure、目标公理与 build 命令记录字段，另计算文件哈希。原课归属核验沿用指定来源独评，不冒称本次重做了原课逐字考据。

## 逐项结果

| 项目 | 结论与当前落点 | 对照证据 |
|---|---|---|
| T43 场景 | 通过。汇总16–18、订正7–9保留本级背驰未成立、只有低级别背驰、随后真实同级 C 与 B 上方分离。不是程序缺证书；不能推广成所有 J，也未排除本级背驰的另一分支。 | J稿28–37；来源独评50、63。 |
| 全域 J 与时点 | 通过。汇总18、20及订正9、13明确未证成或推翻全域 J，未把原文 C 的形成时刻自动视为任意 β_C。 | J稿57–65、77–91；来源独评42、65。 |
| Active 的独立作用 | 通过。汇总20、订正11–13明确真实活动加 EXT-own 的精确结束对应与 T_M<β_k 已直接矛盾，不依赖 J_live 归属结论。未确认、¬Closed 或改 guard 均不补真实活动证据。 | 来源独评 N1，29–42。 |
| E-NORM 前件 | 通过。汇总28明写双方已完成、已选 Norm、原相接边界；同高点与同低点分开。订正17须与此及21合读，没有将“存在标准形式”写成“原切分已标准化”。 | 端点稿41–57；来源独评71–73。 |
| E-CERT 前件 | 通过。汇总30、订正19保留所借原文证书的级别、结构、发生锚。确认较晚不能把异价极值平台改成原结束点；没有把三点几何直接认作合格背驰，也未禁止独立初始完成合同。 | 端点稿86–112；来源独评75。 |
| 8 个自身 end 极值 | 通过。汇总28、32和订正21明确是4份指定历史、8个对象的有限数值结论，未声称所有 RT 或所有完成走势的一般定理。 | 来源独评83–90、119。 |
| 4 个共同接点 | 通过。汇总28、32和订正21保留4个失败及未完成 R 排除。20≠30、15≠5与镜像20≠10、25≠35不能被自身 end 检查替代。 | 来源独评90；端点稿63–68。 |
| 有限改切点 | 通过。汇总34–39、订正23保留9/18/27结束与12/21/30观察证据，两份新三交均为[10,30]，区别于原[15,20]及镜像[20,25]。仅否定该具体改切点是不改结果的显示修复，没有唯一 normalizer 或一般标准化不变量结论。 | 端点稿74–82；来源独评92–99。 |
| Flow 与价格域 | 通过。汇总43、49及订正27–29保留逐 Step、全程有效双边 QuoteOf、统一事件极性、有限列表及 Int spread。不存在任意行情都能提供合法 Flow 的断言。 | MovingQuoteSpec.lean:48–69；冷读53–58；比较42–46。 |
| 空 Flow、活动尾 | 通过。汇总49、订正33明确 HULL 不要求非空、maximal 或 Completed；当前 spread 包络不授予非空/完成资格，也不固定活动尾未来端点。 | DirectHullSpec.lean:19–21；冷读85–87。 |
| Support 来源身份 | 通过。汇总45–46、49与订正31将关联限于参数及存在见证，不恢复唯一轨迹、来源重数、顺序或订单身份。Support 本体不调用 leg。 | DirectHullSpec.lean:8–11；冷读68–70；比较38、84。 |
| ExactHull 不填满 | 通过。汇总49、订正31只接受两端取到与全支撑有界，未声称区间内部全属于 Support，更未声称发生过成交。也没有反向声称具体 Support 必有空洞。 | DirectHullSpec.lean:13–14；冷读72；比较48。 |
| 模型与市场、丢包 | 通过。订正29、35明确抽象模型合法见证不等于市场采集，Flow 无序号或消息完整性字段，未建模丢包仍可留下合法抽象 Flow。汇总51亦保留此限制。 | 比较85–86，纠正原直接稿39的“缺口”措辞。 |
| 反例确认事件 | 通过。汇总47、订正35明确首块只拥有 bid add90 的[0,1)，ask add110在观察数2确认而不属于首块。自身90已足以构成全触价/全簿范围反例。 | DirectHullSpec.lean:23–41；冷读103–109；比较58–60。 |
| 输出来源接线 | 通过。汇总51、59和订正33明确 construct 长度门/可空几何未验证 Step/QuoteOf，逐字段来源桥仍未完成。下一步说明没有冒称已经执行。 | MovingQuoteWireSpec.lean:9–19；比较66–74。 |
| 编译与资格分账 | 通过。汇总55–57、订正39–41分别记录7研究模块 fresh、7 Origin 预编译导入、根依赖标准三公理；旧 semantic=not_reviewed 保持，新比较仅接受锁定 L1 数学对应。 | 冷读25–35；比较23–25、88；两份实际机器回执选定字段。 |

## 已知订正的落实

来源独评 N1 的活动前件直接矛盾已完整进入订正11–13与汇总20；T43 同刻边界限制也进入订正9。独立 DirectHull 比较末尾的三项非阻断订正分别进入订正31、35、29，汇总49、51有相应简写。当前已不存在必须新增的订正项。

原直接稿29仍有“保留事件、簿与中间状态”，39仍有“缺口不在域内”等旧措辞，原J稿89的活动证据也须受 β_C 对齐限制。这些原稿保持历史字节，而汇总3、订正3明确要求结合采用口径与独评引用，因此不构成此次两份目标的残留阻断。若脱离订正单独摘引旧句，本报告不为该摘引背书。

## 验证身份核对

作者运行 `a7166b7d233147b88226a6c5effc948c` 与冷读运行 `ed74a9224d7b4dcca693709d8713830c` 的实际 manifest 均为 machine_verification_passed=true、exact_root_passed=true、root_closure=closed；各有7条本地目标编译记录，全部 exit 0。两者原 semantic.status 均仍是 not_reviewed。

共同根实际类型为 `And DirectHull.SPLIT_TARGET (And DirectHull.HULL_TARGET DirectHull.LIMIT_TARGET)`。两份实际 target 字段的公理集均为 propext、Classical.choice、Quot.sound，unexpected_axioms、unknown_dependencies、unsafe_dependencies为空。semantic SHA-256 均为 `83c95e8fba961d9a19fa0e88c83a208fcf366dce3e56119afd92c29585aa0d77`，它只绑定抽取内容。

本次实际重算仓内7份研究 Lean 源哈希，全部匹配精简绑定记录。Origin 的7份源码与预编译导入身份已在冷读绑定摘要分列；本次读取并核对该分列记录，没有重新构建 Origin，也未将哈希记录当作源码独立重建。根闭包结论不扩展到导入模块的全部未使用声明。

本次没有发现重跑既有 Lean 或数值检查的必要性。未修改任何仓内文件、formal、生产、候选或原稿；未提交、推送或外发。既有未完成的原义来源、真实完成、F₂ 与输出接线义务保持开放。

## 已读输入指纹

下表为本次实际计算的 SHA-256。

| 文件 | SHA-256 |
|---|---|
| `stage46/j-after-completion.md` | `7f6719d1ca1a27b96853ea525ee6d5eb398a2c5590a2aea963200764908ba71c` |
| `stage46/completed-endpoint-scope.md` | `77103f3125845c7fc4b890e04fd0de7a9e7b4e70150451706ab674cbd6cf6c2b` |
| `stage46/independent-turn-sources-review.md` | `dd6da023c1d718b66c27fb784970bf78dfbc893a93ef1fcd1e8dbe2b4500b447` |
| `stage46/direct-route-reentry.md` | `3b6085fa42ca274af0c64bd1ea6f82b71144ca94fe721c67c084672a98d65bde` |
| `stage46/direct-hull-readback.md` | `2e0dd43a12796e21745aff433a7b973b35611141fad2a7f60ae5b06008070e20` |
| `stage46/independent-direct-hull-comparison.md` | `d1784d95a82e2c64630d495f2a7132e4af202e9d46e32ed19713912829b56652` |
| `stage46/lean-verification-summary.json` | `9fda7fa97386e92219ef6b4c8559fa8dbb1112f40a74895cbe4a55f8c859602f` |
| `DirectHullSpec.lean` | `14f4d869525d1c9eaba3c353ff3aff8ffa7dec07b295068742b40bffe7ce088c` |
| `DirectHullProof.lean` | `62779c7ab71c0469f840ddeef4694896bedaf60a8bc8791f8ee46101e048d21f` |
| `MovingQuoteSpec.lean` | `90d6289fba1f1cfeeaa7ceac234bad76dcda515f2ef63330c5d8e7c6a6aaa762` |
| `MovingQuoteWireSpec.lean` | `9c838d7ef90c84f497daee2420ba1bad4d002b31a11d6c6d383dfcd36a3d130a` |
| 仓外 `direct-route-l1-hull/check3/run-manifest.json` | `effd0f13f42f8a1f4f66e52886eae0a97499c1bd7e8bfee257deee4e9df237f6` |
| 仓外 `readback-direct-hull/run-manifest.json` | `d6a65f075f3455487f5b34d4b1226c0a173862fed044437afeb83330d416741e` |
