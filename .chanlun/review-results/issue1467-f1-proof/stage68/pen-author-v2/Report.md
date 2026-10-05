# Stage68 DP68-v2：撤销传播与冻结身份检查

研究票1467，2026-10-06。作者自检，待独评。针对[独评唯一MEDIUM](../pen-review/review.md)，本版实际实现模板端点撤销传播。v1全部冻结字节保留；完整代码为本目录`check_pen.py`，差异为`v1-to-v2.diff`。

**本版有限结果：撤销消息会清除引用该末笔的raw/whole当前端点；未被历史proposal表示的新active只能进入pending；重建及冻结必须核对实际末笔身份。** 一次443前缀主回放通过，11份核心产物与v1逐字节相同；8条接口状态消息控制也通过。峰值35,258,368字节，约33.62MiB，低于96MiB。无新行情历史、无R_W重跑、无Lean。

## 修正的状态合同

v1只有`retracted-confirmed-rw`笔事件，没有清raw/whole的end及terminalCandidate；后续又凭end非空保持旧边界。本版把这条路径接到实际对象。

| 接口/条件 | 实际行为 |
|---|---|
| `withdraw_active(p,t)`且p未stable | 从当前有效确认集合移除p；在raw/whole查找terminalCandidate或pendingTerminalPen指向p的对象；逐个清当前端点及依赖状态，另存取消前后快照 |
| 撤销对象已stable | 先记录`rejected-state-transition`并抛CommitmentError；不清端点、不改冻结L或旧证齐状态 |
| `observe_active`找不到已有合格proposal | 记`unrepresented-active`，不新造过去版本；`bind_terminal`将对应模板保持end=null、pending-unrepresented-terminal |
| 后来通知能匹配已有proposal | 重新核条件点K线几何；历史confirmed首见证书保留，另记reactivated事件；模板重新绑定此既有版本 |
| 每步模板末笔检查 | 比较terminalCandidate对应完整笔元组与当前末笔；不同则取消旧tentative端点后再候选，不能因旧end非空跳过 |
| `freeze_span` | 同时核版本完整元组、对象end、当前末笔及stable证书的candidateVersion；任何一项不符即拒绝，不写冻结值 |
| 驱动收到稳定前缀改写 | 在追加当前观察前拒绝并保存旧/新前缀证据；原稳定列表和对象不改 |

撤销时清理字段是`end`（置null）、`endpointKnownAt`、`terminalCandidate`、`frozenL`、`wholeFrozenL`、`cFrozenL`、`frozenAt`、`rawEvidenceKnownAt`、`rawEvidencePending`、`pendingTerminalPen`、`pendingTerminalKnownAt`。对象start与首次候选时钟保持；旧字段值完整保存在append-only取消事件的previous快照。若这些字段由真实stable承诺支持，就走拒绝路径，不允许“为了传播撤销”清掉已确认对象。

`confirmed`字典保留历史上首次匹配active的证据；`live_confirmed`表示未撤销的匹配证据集合（包括已经stable的证据，不等于只有此刻唯一active）。重新接纳另发事件，不把首见knownAt改晚。proposal版本、历史价格和历史力度读数全部保留。331个waiting-spacing、110个pending-opposite-fractal及330次端点版本替代的含义沿用v1；330不是330条逻辑笔淘汰。

若一笔进入解析器stable却没有当前有效proposal匹配，本代码保留`stable-rw-unrepresented`的解析器事实，candidateVersion=null，模板仍不可据它冻结候选力度。这个分支没有在本历史或本次八步控制中覆盖；不宣称通用候选覆盖已经解决。稳定前缀一致性由驱动先验核对，再接收单笔stable通知；单笔回调本身不替代完整前缀协议。

## 一次主回放：数值和时钟未漂移

使用原443已审prefix及原Stage67同cut五行，step次数443。`run/v1-core-equality.json`逐项记录11份SHA256相等：all-prefixes、events、versions、completion-candidates、pointbar-geometry、initialization、raw-spans、whole-spans、key-trajectories、post-seal-comparison、weighted-lemma。主回放没有发生真实active替换，所以不能把这项等值核验当成撤销分支覆盖；撤销分支由下一节单独测试。

| 片 | end | active相认 | stable冻结 | 原始证齐 | 末c冻结L | whole冻结L |
|---|---:|---:|---:|---:|---:|---:|
| P0 |121|122|126|138|−500|−1500|
| P1 |221|222|226|238|−1200|−1000|
| P2 |321|322|326|338|−200|−200|
| c0 |421|422|426|438|−4200|−3500|

441个端点版本、111个候选族、110个active匹配、109个stable匹配保留。P0的118历史读数仍为−500，119只替代其“当前待确认终点”身份；121版本在122相认、126stable，122之后新残段不写入旧片。已结束待证齐whole与当前残段仍分槽。`templateCurrentRawSlot`沿用兼容字段名，其值是primitive笔槽，5笔原始段槽需整除5；没有把它当原义Owner。

## 八条接口消息控制

唯一主回放到122和138时各捕获一个Machine状态副本；测试只向这些副本发状态消息，没有调用step追加行情，价格数量分别一直保持123与139。详见`run/state-control.json`，每步有raw5/P0状态及拒绝证据。

| 消息 | 输入与验证 | 实际结果 |
|---:|---|---|
|1|122状态中已有117→121 active及raw5/P0待稳定端点；发撤销通知|两对象end=null，endpointKnownAt和terminalCandidate清除；旧证书和441以前的相应版本集合不变|
|2|发117→122身份通知，坐标和价格均已观察，但历史无这一同起点proposal|unrepresented/pending；end仍null，无terminalCandidate；零proposal回填|
|3|撤销消息2的未表示身份|pendingTerminalPen及其时钟清除，回open-after-terminal-withdrawal|
|4|重新通知原本存在的117→121版本|只引用既有`DP:117@118:v121`，raw5/P0重新绑定end121；历史首见证书不重写|
|5|接收从真实138检查点取得的实际stable回执knownAt126，再执行冻结|raw5冻结−500、P0末c−500/whole−1500；此副本尚无138原始证，不自填rawEvidenceKnownAt|
|6|对真实138证齐状态尝试用不同的当前末笔冻结P0|以freeze-terminal-identity-not-stable拒绝；end/L/126/138全部保留|
|7|对该真实138状态尝试撤销已stable的117→121|以stable-pen-cannot-be-retracted拒绝；对象及稳定证书不变|
|8|向该状态提交修改既有稳定笔端点的前缀|以stable-prefix-rewrite拒绝，保存incoming/previous；对象及稳定证书不变|

消息2是**接口身份通知的容错控制**，没有把117→122说成合法市场分型或R_W确实产出的active。消息5复用真实证书时钟，并未制造122到126之间四个新观察。三条应拒绝消息本身故意违反接口承诺，用来核拒绝行为。这八条不是第二条真实/合法行情历史，也不是八个市场样本；真实历史中的active替换次数仍0。

消息1之前end、endpointKnownAt、terminalCandidate实际存在并被清除；冻结和原始证字段当时本就不存在，控制验证其仍为空。对**已经存在**的冻结/证齐字段，消息6–8验证应拒绝修改而非清除。清理函数防御性移除未获stable承诺的旧缓存，但本轮没有人为往合法待稳定状态塞入假冻结值来伪造“清除了真实确认对象”的覆盖。

## 来源、读取和可采信范围

`source-evidence.json`逐字复制v1已审直接来源，内容哈希相同，没有新来源裁定。062/065笔及分型正文、082分型可能不延续成笔、081续排博文新笔条件的归属和注释排除沿用；订单观察映射为原文K线的桥仍未建立。新的变动是状态传播程序，不是原义笔定义。

v2预先解析v1 manifest，只含路径/长度/哈希；对输入及v1全部冻结文件做字节散列。source观察/事件数组仍整体预载；prefix按65536**字符**文本缓冲逐项解析，Stage67行到当前cut逐行解析。终值pens/objects/parent/certs的JSON解析仍在online-seal之后；v1核心结果只在seal后散列对照。`run/reading-order.json`和online-seal明确区分字节预读、JSON解析与step依赖，不声称物理未来信息不可达。

一次命令exit0、stderr空，主回放及八条控制共用该进程；AST/不写pyc的compile通过。v1所有27个manifest内容项目及其manifest/FINAL共29个文件前后SHA相同，详见输入前后哈希与`v1-preservation.json`。没有改仓库、正本、生产、formal或tracker。作者包为待独评状态，不自称独立审查通过。

## 尚未建立的命题

本版修复的是有限模板的撤销传播与冻结身份检查。真实active替换、同类锚替换、同价/方向非法分支的市场行为，以及任意历史上的候选覆盖/接口总性仍未证明。端点版本不回写只说明记录层，不是原义身份稳定。三条拒绝检查也不是任意错误输入的全域证明。

StructuralPenNow、每时刻数值原义L、RootArm67、GeneralDiv、原义Completed、Owner/Next、固定F2及完整F1全部仍null。原始证齐时钟不升级为原义完成时钟；冻结−1000的b_low仍不补RootArm67。这里的“未建立”是交付和证明边界，没有得到原义反例或F1不可能性定理。
