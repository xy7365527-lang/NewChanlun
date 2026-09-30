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
