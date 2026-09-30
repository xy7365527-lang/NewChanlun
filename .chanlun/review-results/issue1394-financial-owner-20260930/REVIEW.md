# #1394 客户端与 E/B owner 独立评审

实现与评审来自不同会话。根整合按相同 Rust blob复核；本文件记录限定差异的独评，不等于整票/整图或 main/CI验收。

## Spec

最终 PASS，source client `268f864ed48` / owner `d8b4b713e4`。初评实际发现并修复客户端完整复核/selected/片段失败漏 records及合法长标量隐藏限制；原坏页同Job三次records0被独立复现，最终1/2/3、success仅加1、cap1第二访问拒、缺祖先0/缺row1、后代访问逐次计费。

Owner初评三条HIGH：审计重开Job、提交期限未贯穿、失败stage未释放。统一Job后八规定schema路径每次16 records，同Job cap31第二轮拒；封存image不改。清理与恢复仅删除staging，不换依赖Job/不删pub。最后保留入队前原Instant；真实20ms期限/250ms阻塞新入口在256ms后拒、records0，旧入口行为保持。原回执在expired ingress仍优先返回，new身份拒。

末次COMMIT门位置曾只静态核验，新增整体SIGKILL/正式CLI证明其当前受测路径，不把它们说成跨期限COMMIT竞速单例。所有完整历史路径、原件root、新旧隔离和离线能力按C5.7对齐。未扩资金政策或场所资格。

## Standards

最终 PASS。先概念守卫，再工程轴。完整客户端和owner4文件补修、最后d045→d8b4五文件分别独立审查；未发现剩余HIGH/MEDIUM/LOW整改项。旧Job/serve接口及严格验证复用同一core；私有TimedJob不进入公开JSON/Hash；借用/线程/错误传播、同事务与cleanup失败的pending-recover错误已核。Fowler启发式未形成需整改项，未以行数或grep命中冒语义违规。

初反例/返修未隐去。编译输出仍有既有告警，Standards PASS不是lint零告警。已运行的限定测试、3阶段、24故障、半导入与正式CLI原件分别绑定；尚未实测的全9011、全资源、生产部署/真实外效以及其它TB未被此轴覆盖。

## 事实封包复核

独立会话核对初版13份原件的全部 bytes/SHA、24case/12pairs、半导入、3CLI与3离线recover，确认票仍OPEN且未扩证明域。唯一LOW“只有ignored夹具追加”已改为“仅追加验收及输入回归测试，经济运行入口保持ignored”。之后增入的第14份根挂载核对原件已由根验证：本票剩余映像为0、原件保留。没有重跑已通过案例。

随后追加最终COMMIT真实ENOSPC的10份归档原件，由根复核原执行文件、binding、manifest、源码与输出SHA，核实唯一hook返回false、真实errno28、实际disk full错误及原子失败断言。原始测试成功并recover epoch2，3新卷已回收；清理后的SQLite错误码0没有被写成13。产品Rust无新增修改，本次不是第二份产品独评或整票关票证明。归档共24份inputs，全部bytes/SHA已核。

补充独立会话随后对冷审计1836d3c38e9、当前产品来源变化attempt4和正式Q HTTP进行Spec/Standards两轴审查，均PASS且无需整改缺陷。声明域只含cold source memo未用/实际重读与损坏拒绝、subset400/1307变化重读累计及已完成请求后的信号收尾，不扩大为所有memo容量、跨期限拒绝、成功后即时stage0或完整consumer/controller/browser。报告见inputs/INDEPENDENT-SUPPLEMENT-REVIEW.md。

冷审计两文件cfg(test)已整合为dfab63cd95，与受审1836的两文件逐字一致，复用该固定源的两条实际通过检查。新证据归档共32份inputs，全部bytes/SHA已核；Q ZIP内72份成功原件和冷审计ZIP内原stdout/库副本均保持原字节。实际生产代码、main/CI和整票/整图完成不由新增测试或限定补审替代。

随后后台/2观察适配abada33cae由独立python-reviewer审查：Spec没有行为缺陷；按该工位类型标准，新增公共economic_revisions缺注解列为HIGH。最小d34dd1e0d2e仅补Mapping import与真实tuple[str,str]签名；独评对函数body AST、实际模块导入/typing.get_type_hints及四SHA复核，初版报告不改，最终Spec/Standards PASS。整合4325ab248b/219c4f2ab1的四文件与受审d34逐字相同，复用36项通过，不重复全套。

两份Python独评及原红/绿和返修证据已加入封包，共36份inputs全部bytes/SHA已核。未安装或运行不可用的静态工具；原manifest仅有包含写操作的source，因此尚未进行正式Q/B观察，须另冻结新域的独立只读源。代码评审与单元向量不替代实际HTTP或浏览器路径。

实际观察随后在初始化前增加独立只读Q源的新域完成。另一独立会话核47个封包引用、6真实HTTP标准库重Hash/cause/replyID、token、三域native image/cut/root对拍、12变化image Hash及2Original全对象等价，给限定事实PASS。它区分case1长canonical socket的PartialVector及Q writeDenied负控与case2同权限继承的成功，未计第二次写负控。资源Q19逐字与旧具名fixture相同，8经济限额不变；进程与6卷均回收。

现有封包扩至41份inputs，全部bytes/SHA已核；实际HTTP与独评仅覆盖具名TestOnly样本。browser、独立完整内容闭包、全部资源/历史/9011及其它TB仍未由此证据结清。详见inputs/ECONOMIC-HTTP-INDEPENDENT-REVIEW.md；没有重跑已通过案例。


## 严格经济 /2 测试客户端与真实浏览器

固定9f923的四文件经独立Spec与Standards PASS，精确整合3b53744。旧/1空作用域、Evidence会话/化身、决定与完整历史/权利、Reserve/grant/捕获E及原输入生成式的全部明确HIGH/MEDIUM均闭合；最终Close原因仅复用原非空text门，不trim、排序或改变跨请求重复规则。256完整向量与最后25定向向量组合复用，不相加称281。全部旧FAIL/原反例保持，见inputs/ECONOMIC-CLIENT-RAW.zip及最终独评。

实际浏览器由另一管理会话运行，独立事实会话再核418个文件、12真实HTTP、三域current/Prepared85/Watch、四Original、两截图和fresh回收状态；PASS_BOUNDED_ACTUAL_BROWSER_FACTS，findings空。首个离线审查脚本误用domain/id而非实际domain_id/object_id，改审查读者后通过，不当成产品缺陷。经济API不提供节点闭包及无前根顶层root的独立证明；没有把本域通过扩大为controller/Q3、9011、主线或整图。
