# Stage18 独立审查待审包

状态：pending，`semantic=not_reviewed`。当前父模型切换能力限制未解除，不绕过已记录的CUA安全限制派发。该限制不妨碍作者研究，但作者不能代替独立审查者。

先由新上下文只读[NineRegroupSpec.lean](NineRegroupSpec.lean)、所导入的`LiftBoundarySpec.lean`、`RetileSpec.lean`及实际Origin定义，复述真实量词与假设；另由独立比较者对照[开工卡](Stage18WorkCard.md)。不要把本文的预期结果发给盲读工位。

随后核查[NineRegroupProof.lean](NineRegroupProof.lean)、[源绑定](source-sha256-stage18.json)与[机器摘要](stage18-lean-verification-summary.json)。完整通过证据须按[报告](Stage18NineRegroup.md)解压后恢复原路径；两个归档均有逐文件SHA收据。机器通过与资源估计超限分别报告，不混成单一pass。

重点攻击：

- `SourceTriplet`是否真正允许块长变化和未消费间隙，唯一性是否被藏进定义？
- 长度9和每块至少3如何导致所有间隙为空，是否遗漏共享端点与重复消费单元的区别？
- 非空分组、合法DC、正量及局部成功证据是否真实进入根依赖？
- `Carries`使用完整外缘还是子核心？实际Origin严格谓词是否确实被否定？
- 为什么缺少完成证书仍能作必要几何条件的否定，而不能得到所有F₂的不可能性？
- 至少三原成员条件是否被越权套到Z-8连接对象？结论是否被扩大到未来新增输入或直接订单路线？

逐项报告具体finding、声明/证明定位、反例或理由。若要扩大候选类，须新声明与新证据，不能把本轮结论直接推广。
