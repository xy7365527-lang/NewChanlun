# Stage17 独立审查待审包

状态：pending，`semantic=not_reviewed`。范围为 #1467/#1468 的 `R_W-DC-postcapture-v1`，不含生产系统或旧Lean定理的重新验收。依用户派发规则，当前父模型不能通过可用专用工具切换；先前CUA访问Codex自身被安全限制拒绝，未绕过限制派发。

审查者从以下最小材料开始，另开新上下文，不继承作者判断：

- 预先冻结的 [Stage17WorkCard.md](Stage17WorkCard.md)。
- 实现 [published_l1_dc.rs](published_l1_dc.rs) 与未变的 [directional_change.rs](directional_change.rs)，锁见 [stage17-core-origin.json](stage17-core-origin.json)。
- 作者检查器 [stage17_verify.mjs](stage17_verify.mjs) 与 [stage17_provenance_verify.mjs](stage17_provenance_verify.mjs)。
- [证据索引](stage17-evidence.json)记录原输入、二进制、运行收据及报告SHA；原始记录与检查点在仓外，不上传订单身份数据。
- 接口前提 [Stage16WorkCard.md](Stage16WorkCard.md) 和 [Stage16CausalViews.md](Stage16CausalViews.md)；独立核实所依赖字段，不将ready扩大成全市场完整性。

审查问题：

1. 明确的发布后时钟与原生逻辑路径是否被混淆？快照或缺口恢复时是否回填中间态、跨缺口拼接或提前结算？
2. 精确投影、舍入与溢出域是否符合声明？同价的数量变化是否确实进入观察？仅数量变化的中点是否保持不变？
3. 端点发生、方向首见、反转确认及捕获发布是否可区分？任何输出有无使用尚不可见的观察，active有无被当作completed？
4. 输入重放恢复是否被误说成内部状态持久化？重复、丢失与绑定错误还有哪些未覆盖场景？
5. 作者checker是否复制了实现的错误，或者负控制只说明实现自洽？有限结果能支持什么，不能支持什么？
6. 88次非match数量确认是否被扩大成信息优势、预测能力、F₂增量或净收益？报价中点有无被错误替代成交A？

逐项输出 finding、源码/收据定位、最小反例或检查方法、影响的具体主张。独立重跑只针对发现的问题，不以重复全部旧测试代替语义审查。不得为通过而改变时钟、δ、输入域或F₂验收；若需要新定义，应另立版本保留旧结果。
