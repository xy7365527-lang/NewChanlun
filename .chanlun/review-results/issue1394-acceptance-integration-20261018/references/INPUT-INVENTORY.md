# #1394：根最小冻结清单与真实闭合路径

本补充不修改已封v2包；execution_authorized=false。只按现存已入Git receipt给出的精确原件路径核当前可读性/SHA，没有递归扫盘、没有重开tmp搜索、没有运行产品或构建。KNOWN-INPUT-ORIGINALS.json逐项保原路径/预期SHA/当前SHA。

## 已找到，可直接按原字节复用

完整9011 TestOnly结构输入目录：
/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-full-history-formal-prep-20261001/inputs

| 原件 | 当前SHA256（均与原receipt相符） | 可复用范围 |
|---|---|---|
| source.bars.frozen.json | 76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087 | 完整9011条冻结synthetic OHLC；不等于整图完整市场历史验收 |
| envelopes.draft.jsonl | cd69fef24d6e05a9950e14131095eef40dd9c9f16738958ea89d4900a8462283 | 原9011顺序/请求原字节；新candidate身份适配须另冻，不改原件 |
| clock-plan.full9011.draft.json | b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01 | 原结构域TestOnly时钟，不自动覆盖新经济操作时钟 |
| schedule.full9011.draft.json | 4ea1625c312cf5c63666b37dbedbb70f2f15ffe2f5a2fce1306dd61f0146039b | 原输入调度，完整保留不缩短 |
| raw-inputs.draft.jsonl | 3c5d17844c933a9879c37f4621d10f918f1f8c37a733013fa159bcb28dd20b8d | 原请求负载 |
| source-interface-binding.json | 51df7f21e95a425da7ec050621f06f29b9bf593a66681896bf7ebe84bb37d77a | 旧来源接口绑定，只复用其明确事实 |

逐9011 ordered_request_hashes已入Git，精确入口为v2 producer-source下 issue1394-financial-owner-20260930/full9011-first-attempt-inputs/034-actual-PREPARED.json。旧binary/head不继承给b46。

#1374的合法受控Voice AUTHORITY-BUNDLE及25件artifacts当前全可读、哈希全匹配原独审receipt。authority_id=testonly-gamma-source-authority-v1，bundle SHA=5f20cf8b3359a1e7d89cfa369a19e1576fa51dfcda7eef8e10a51407cd836ee2，独审receipt SHA=85d9adea47705768fd71de0a39a72e85313b77b4814082746bb9a5963de48d8f。原件详细绝对路径在KNOWN-INPUT-ORIGINALS.json（Voice表）。其合法范围只是TEST.GAMMA.V1固定单根；第二gamma只作关系对照，不能借此造第二经营绑定。

## 不可直接拼接的实例

9011输入第0bar close=1151/timestamp=1700000000；Voice输入50070条，第0bar close=753294/timestamp=0，首次确认序号50069。数据不同，9011最高序号也小于该确认时点。因此旧Voice原件可作为原域权威与实现参考，不能改input hash/profile/instrument或套到9011新域；不重新认证就无法闭合当前实例。

已知旧small九字段/2 manifest及finance-input仍可读并hash相符，精确路径见SMALL-DOMAIN-REFERENCE.json。原finance-input SHA=56189b58672b1b458fc2ec933d117c3cf1721bedc031bd86c11742ea27400566。现金基责20，权利alpha40/beta50；alpha数量2×限价14+费尾2=30，beta数量3×12+费尾4=40，总责任90。原cap100，旧驱动另有cap80等待域。可作为沿既定TestOnly模型的新具名数字提案，不复用旧S引用/Voice/回执。

该small manifest的alpha/beta是unit权威；finance-input allocation_policy_id仍是旧#1349/unresolved-common-cap-allocation，不能直接用于当前验收。当前既定语义已是#1349/atomic-slot-wait/v1（b46 finance.rs:44–45、ADR0029），这不是待选择的新资金政策；需要新实例按现行既定id、真实本域S对象/Voice/原开局重新绑定。原clock完整保存在参考JSON，仅候选，不自动与9011结构观察时点拼接。

## 数值候选与真正缺件

ROOT-RESOURCE-CANDIDATES.json逐项列值、来源、对象、授权状态，避免全null空转。retained=268435456固定金融上限；会话capture805306368/request193120/reply827022/request45000ms/startup300000ms/requests4096是保旧值提案，closure_nodes262144为A6旧建议提案。均不声明新候选实测容量。Q18保持原值，三Delta可提原9011配置16777216B/100000节点/10000记录，由根冻结为新具名输入；不能把旧配置当新PASS。

reader四单次候选为193120/827022/193120B/45000ms。E↔B旧small例frame16777216B/response60000ms供根比较，不是当前Long已批帽。Job8旧small值全部列出，包含12小时max_elapsed与2614节点，明确不可盲目沿用为Long；当前完整闭包节点/记录规模、操作绝对总窗需要根根据当前真实域冻结，作者不捏造容量证明。

外围建议cleanup总60000ms内TERM2000+KILL/wait8000+余50000、seal65000另预留，保旧外围上限；仍只提案。所有阶段与服务帽同时生效最早截止，旧basis16MiB/10秒保持，不按片/重试重置。新driver/两ARTIFACTS/toolchain/监督脚本SHA/根单次seed尚缺，当前不能执行。

## 从9011真实正式S到本域Voice的最短闭合

1. 根冻结已找到的完整9011原件、输入顺序与新source身份，不缩输入、不改原件；在另行具名资源/监督合同下取得当前正式S的完整输出与源根。当前清单只证明输入字节存在，不证明新b46已经产出完整有效本域γ。
2. 从同一正式输出枚举实际Classification终端BSP和完整K载体，按规范锚/同级/完整源区间证明对应；构造本域定义γ并作独立语义/Lean边界核验。若9011输出没有合法γ，明确返回该缺口，不用旧端点、旧权威、unit夹具或宽松判据补成成功。
3. 冻结本域authority bundle与独立核验receipt，绑定相同input/profile/instrument/chong/原开局和观察时点。两个真实经营owner如都需Voice，必须各有被批准的本域绑定；旧单根权威不能给两个新owner兜底。
4. 通过真实CaptureExternal/ApplyAllocation产生原开局记录与e_cut，再正式BindVoice。按既定TestOnly现金模型和atomic-slot-wait语义冻结新finance实例，保所有原账未知项；随后当前driver、两角色ARTIFACTS、资源/监督闭合，才交精确执行准入审及完整fresh A/B/恢复。

现成消费接口：b46 rust/src/economic_session/evidence.rs:130 load、:179 verify_witness、:207–248核bundle/独审/artifact exact字节、:328时钟映射；voice.rs:57–93 exact BindVoice、:121 bind_image、:136–182核本B已应用开局及同标的/重/权威，不允许假e_cut。

找到可读确定性参考，但没有找到可直接认证新9011正式S域的通用入口：旧binding-v1/prepare.py（SHA52140c7ad39127da4c7f0b71ac127030d4aa2e690203e63cb16623c67b8a258b）固定50069/森林328/两旧terminal/单根；旧endpoint-probe-v1/src/main.rs（SHA9d433c9700dc222f84d341acabc4efd7654e1da999a93e85aebedd2fb1f5c79f）可收input/outdir，但直接调用旧theta_v0 classify并硬编27636/33152/39438三端点。generate_definition_evidence.py也属于旧冻结域。它们只能供适配参考，不能替代当前正式S、不能冒认证或旧PASS；完整绝对路径/sha在ROOT-MINIMUM-FREEZE.json。此缺口是同域γ/terminal/carrier/权威实例与适配入口，不能靠改模型、重贴旧JSON或选新资金政策解决。

本补充已同步driver工位。根可先冻结确定输入与不扩帽的数值实例；当前域桥接和产物缺口真实保留，execution_authorized=false。
