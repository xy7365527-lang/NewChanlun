# #1414 TB-02-D 独立 oracle 设计与冻结工作例

> 名分：绑定 #1414 的工作草稿与有限构造见证，不是新教义，不裁 #1344 一般扫描窗口/51+/continuation。
> 基线：f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9；独立范围：raw → 新笔 → FIRST/SECOND、strict 对象、笔破坏与段形成的具名有限前缀。
> 冻结顺序：先在本文件列出手算笔级账与成功例，再用纯整数输入工具构造 OHLC；不运行生产段函数、服务、DB 或 166/9011，不从生产输出反写 oracle。

## 已核来源与作者归属

路径一律 `docs/chanlun/text/blog/`。逐行检查当前行号、界线、行首注和行内注。

| 来源 | 可用主张与归属 |
|---|---|
| 067:18/20/22 | FIRST 反向笔、独立标准化、UP 顶/DOWN 底；正文界62。20行只取「经过非包含处理的特征序列，成为标准特征序列」，避开行末「（娇：」注。 |
| 067:38/46 | 有gap的第二序列、任意分型、SECOND不必封FIRST旧gap；均正文界62前，无行首/行内注。38的对偶示例不得覆盖46的任意分型。 |
| 078:54 | SECOND同原段方向，严格包含；正文界64前，无注。 |
| 062:28、065:20 | 方向性包含、顺序合并。062:28正文；065:20正文界56前，无注。065:18含娇注，只援用注外的 max/max 与 min/min 公式，不把注作为作者证据。 |
| 067:170/196 | 缺口被第一笔封闭归第一种；后续封闭不能倒改为第一种。界62后、问题162/164/166及192与作者答句通过168「==」和194「==」分开；170/196为作者答疑，无注，不能称正文。 |
| 065:44/46 | UP取j≥i+2且d_j≤g_i，DOWN取j≥i+2且g_j≥d_i的笔破坏；正文界56前，无注。 |
| 071:28/38/40/42 | 只在同序列内合并；假设转折点前后不合并；正文界46前，无注。 |
| 077:62/64 | 奇数、至少三笔、前三闭重叠；正文。 |
| 081:110 | 新笔的merged不共K与raw至少三中间根；该行是续排2007-09-18《风灾》的作者正文，不是81课初篇后的评论。标题100与作者上下文104/106/108/112连续，无注。 |

正本 `.chanlun/definitions/xianduan.md` G3/G4、`.chanlun/definitions/bi.md` 新笔是裁定入口。已签合同为 `/tmp/goal1323-native-20261012/tb1414-prep/SIGNED-OBLIGATIONS.json` ST007/ST008、CC012/013/014。字段适配对齐实施工位 `/tmp/goal1323-native-20261012/tb1414-schema.md`；本文件冻结几何事实，不用字段形状变动改变预期。

## 手算账与具名成功例（先于 raw 构造）

以下价格是设计单位，生成输入采用 p↦2000+100p，镜像采用P↦10000−P。所有比较是整数精确比较。笔号0起，端点p_j是笔j的起点；FIRST初始取笔1/3/5，旧段起笔0，几何终笔2、端点p3。所有种子前三笔公共区间[10,20]，方向UP，三笔交替，顶40>底0、首尾同向，四条件全真。

| 具名UP工作例 | 手算端点p0..pn | 预期 |
|---|---|---|
| second_open_up | 0,20,10,40,25,35,22,30,23,33 | FIRST [10,20],[25,40],[22,35]顶，gap=(20,25)，c.low22>20不封；SECOND笔4/6/8=[25,35],[22,30],[23,33]底分型；实际through8。全部后续低点≥22，旧gap不封而CASE_TWO成立。 |
| second_nondual_up | 0,20,10,40,25,35,22,38,26,39,24,37 | FIRST同上；SECOND笔4[25,35]、6[22,38]包含，沿DOWN min/min合成[22,35]（组4/6）；笔8[26,39]，10[24,37]构成顶分型。首个SECOND分型是非对偶TOP，通过10，旧gap仍不封。 |
| second_inclusion_up | 0,20,10,40,25,35,22,38,21,32,23,33 | FIRST初始同上；SECOND组4/6=[22,35]、笔8[21,32]、笔10[23,33]形成底分型，通过10；组4/6不能用FIRST元素或FIRST方向态替代。 |
| second_late_close_up | 0,20,10,40,25,35,22,30,15,27,18,28 | FIRST同上，c不封。后续下降笔7（30→15）第一次封旧gap，不能倒改第一情况；SECOND笔4/6/8/10=[25,35],[22,30],[15,27],[18,28]，首个底分型中心笔8，通过10。 |
| strict_c_closed_up | 0,20,10,40,25,35,18 | FIRST [10,20],[25,40],[18,35]顶，raw gap=true，c.low18≤20，strict有效CASE_ONE；无需SECOND，第二判据不适用而非false。 |
| no_gap_control_up | 0,20,10,40,15,35,12 | FIRST [10,20],[15,40],[12,35]顶，无raw gap；strict封闭比较不适用，SECOND=null/NOT_APPLICABLE，CASE_ONE。 |
| first_extension_pending_up | 0,20,10,40,25,35,22,32,21,30 | FIRST方向分型存在；SECOND笔4/6/8=[25,35],[22,32],[21,30]单调DOWN，无顶底分型。完整扫描当前有限prefix保持PENDING_CASE_TWO；不能把FIRST右侧延续改名SECOND。 |

每例对应 `_down` 是确定性价格镜像：高低对调，方向反转，UP的SECOND底/顶对应DOWN的顶/底，G4改c.high≥a.low。同向SECOND初始包含方向也镜像，不能固定复用UP min/min。

跨序列见证：second_nondual_up 的FIRST笔5=[22,35]与SECOND标准组4/6=[22,35]几何相等，却sequence_id不同，不准合并；SECOND原笔4/6与FIRST笔1/3/5集合不交。second_inclusion_up 同样提供组4/6独立包含账。

笔破坏单独按065定义。除no_gap_control外各例在笔5完成时首次已有i=2,j=4，g2=p3=40、d4=p6=22（strict例18），j=i+2，d4≤g2；具名earlier_stroke=2,later_stroke=5。SECOND未齐时向量为(笔破坏true,新段false,段破坏false)，成立时为(true,true,true)。此比较不等于「FIRST旧gap封闭」，不以终结enum抹去独立事实；本组不声称排除了01，也不声称四签名均可达。no_gap_control更早在笔3完成时有i=1,j=3、d3=15≤g1=20，见证0→3、最早cut19；不得将FIRST出现前的笔破坏全部填false。

## raw、新笔与真实首次可知

端点中心根 anchor(j)=1+4j；底分型中心q=P+1，顶分型中心q=P−1。每根O=C=q、L=q−1、H=q+1；两端之间三个中心q用整数向下取整插值。raw0位于首底上方，最后端点后补两个同向护根；相邻中心q始终严格单调且不同，OHLC不包含，局部极值只有指定anchor。raw与merged一一对应，两个分型中心距4，分型不共根，原始严格中间根恰3，故落正式新笔域。该证明独立于生产识别器。

端点p_j第一次可知raw=anchor(j)+1，输入条数cut=anchor(j)+2；只见中心根未见右根时不能算成分型/笔。seed最早cut15（raw14，前三笔0..2）。FIRST与笔破坏最早cut27（raw26，六笔0..5）。正常open的SECOND在cut39（raw38，九笔0..8）才齐；其前cut38的最后顶/底尚未获知。nondual/inclusion/late_close通过笔10，最早cut47（raw46，十一笔）。几何终点固定raw13，不能据其回填获知时间。late_close笔7封gap最早cut35，与SECOND cut47分列。

每例冻结seed cut15、FIRST前cut26/FIRST cut27、SECOND最后分型右根前后，以及完整末尾护根prefix。负例枚举当前有限全集，不对无限未来判否。scan resource_truncated=false，完整负例状态COMPLETE，正例可凭有效命中提前结束并精确报实际through；不引入cap。

## 实施与有限自检计划

仅写本文件与 `s_session/tests/fixtures/tb02d/`。冻结 `raw-ledger.json`（所有具名自然OHLC）、`hand-oracle.json`（手算端点/每笔raw映射/首次可知/各cut的FIRST窗口、strict三元素及阈值、SECOND每次包含与首命中、破坏见证）、`generate-input.py`（纯输入生成与几何自检工具）。后者只用标准库整数算术，不import项目、不启动任何进程/服务，不接生产结果输入。独立验证器/正式重放/持久恢复由验收工位执行，不记本工位自检为端到端验收。

## 固定原件与量词澄清

实际固定14case（七类及双向镜像）。生成器纯几何自检全部成立，报告仅是作者自检，不是生产验收。`MANIFEST.json`绑定两JSON与输入工具SHA；`hand-oracle.json.cases.<name>.cuts`用raw接纳条数为键，wire decimal字符串适配由正式验证器处理。

`first_members/first_window/strict`冻结首次具名FIRST check的1/3/5与第一右笔5。pending中FIRST可继续含后来的7/9等，其全集另列`all_available_reverse_strokes`，不能把原见证当全集。SECOND的raw/standard/steps/scan按每个cut完整列出；有效命中可提前结束，不命中须扫完当前有限prefix。后来FIRST包含组的价格更新不能重写067:170/196的第一笔封闭时点；late_close笔7在cut35才封，原c笔5的22>20始终未封。
