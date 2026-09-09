> 链接转换阅读版；正文语义与原稿一致。签署原字节：[原稿](../../../../payload/sources/newchanlun-1339-architecture-r2-20260909/author/AUTHOR-REPAIRS.md)，SHA256 `fb6d5a5e512c6894358d69fc0a33f294ce5599684a232dbb3b448664d7dd3eb0`。当前批准状态见归档根APPROVAL.json；原稿中的待审状态保留为发生时记录。

# R2 初评后修订说明

本体作者修订；独立关项待完成。基线是PROTOCOL-R2 8570fc96b47ec90f3a87ac829b78322f1aad9384d95694aafdb53fd5f712e1c5；初评4项发现完整保留在review/REVIEW-RF02-R2-FIRST-PASS.md/json。

| 发现 | 本次修订 | 仍须核实 |
|---|---|---|
| RF02-M01 | §4/5/7完整条件及阶段绑定；适用未分类/覆盖不足/不可核即Unsupported，E-only须具名权威和责任覆盖；市场证书明确集合与修订。 | 场所和政策实例并未穷举或验证；阶段必须服从实际语义。 |
| RF02-M02 | §3/5/6/9取消busy即最终Denied；S本地就绪队列在Commit与下个Begin之间获有限非零服务，每个请求有限稳定点内核验，真实失效不可复活。 | 该合同需独评；工程调度参数、资源边界及运行活性没有验证。 |
| RF02-M03 | §5 X权威同ID持久OwnAttempt→MayHaveCalled，只有fresh原子赢家调用；重复只查，崩溃/未知不重发，同ID换epoch不重置。 | 实际发送栅栏、场所身份/查询/幂等未实证。 |
| RF02-L01 | §6 S权威Undecided与X状态、观察Unknown/Unavailable分轴，保留last_known与query_frontier；缺终态不能推未发。 | 观察接口和故障展示尚未实现。 |

反例的修文排除依据：原busy循环在Commit之后必须先履行有限稳定服务，不能合法地让所有就绪请求只在busy时被拒；同ID两条调用路径争用同一持久OwnAttempt，只有第一次原子迁移能授调用权。两者是合同论证，不是已运行状态机、模型或生产测试。

检查仅限MD/JSON一致、链接存在、17件原稿归档和旧R1绑定字节未变。四事件24排列复用原结果，守卫未改；新增条件阶段、S队列与X原子占有未被该模型覆盖。未修改仓代码、Lean、UI、GitHub或服务。
