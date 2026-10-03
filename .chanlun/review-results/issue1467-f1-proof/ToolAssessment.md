# 数学研究与量化实验工具核查

日期：2026-10-03。服务于[订单基底候选构造与合法性证据](https://github.com/xy7365527-lang/NewChanlun/issues/1467)，沿用现有研究图和证据包，不另建调度系统。以下为一手源码/文档核查，另有明确标注的本机局部验证；没有安装或运行外部LLM研究框架。

## 采用范围

| 工具 | 核实的能力 | 对本研究的判断 |
|---|---|---|
| cameronfreer/lean4-skills | prove/autoprove固定声明header；disprove区分已认证反驳、未认证见证和未决；Lean编译与假定白名单检查 | 可借用证明/反驳纪律。框架启动不替代本项目语义验收；当前保持手动调用本机Lean，不安装其hooks |
| matt-w-horn/lean-skills | 提供读取elaborated environment的声明锁示例；同时记录定义体；要求用增删声明、共享variable改变验证锁 | 它与上一项目不同。本轮采用该设计思想，自写适合Lean4.31、无Mathlib的研究导出器，并冻结导入对象及文字验收 |
| Kripner/openprover | planner/worker、时间或输出token预算、运行目录恢复、Lean提交检查都有实现 | 适合作为未来候选执行器；本次源码发现声明检查和自定义模型路径不足，不直接迁入当前研究 |
| Microsoft RD-Agent | 自动假设/实现/评价循环；LiteLLM默认后端；官方写明当前只支持Linux，多数场景需Docker | 将来可做受控实验提案/实现，不能负责确认集反复选优；本机macOS未做容器兼容验证 |
| Microsoft Qlib | 有数据/训练/评价/回测链及高频例子，Exchange有报价字段、量约束和费用参数 | 可作模型评价层候选；所查通用Exchange不是本研究订单ID/队列撮合契约的实现，不能以通用回测替代逐事件验证 |

来源固定为：

- [lean4-skills prove](https://github.com/cameronfreer/lean4-skills/blob/b6243b85b9b0a0ddff5bb6773889044daf687f8e/plugins/lean4/commands/prove.md#L174)、[disprove](https://github.com/cameronfreer/lean4-skills/blob/b6243b85b9b0a0ddff5bb6773889044daf687f8e/plugins/lean4/commands/disprove.md#L10)。
- [lean-skills statement-freeze](https://github.com/matt-w-horn/lean-skills/blob/5e5d61caacae7f159c3e583e7efd7cbcdf0ca0ed/skills/lean-refactoring/references/statement-freeze.md)。上游示例验证工具链为4.32.1，不能因此声称本机4.31已兼容；本轮自写版本另行编译验证。
- [OpenProver CLI](https://github.com/Kripner/openprover/blob/e200251b34349ab6c34548d30319abde86cb6bc6/openprover/cli.py)、[prover提交检查](https://github.com/Kripner/openprover/blob/e200251b34349ab6c34548d30319abde86cb6bc6/openprover/prover.py#L1071)、[HTTP客户端](https://github.com/Kripner/openprover/blob/e200251b34349ab6c34548d30319abde86cb6bc6/openprover/llm/hf.py#L228)。
- [RD-Agent安装与后端说明](https://github.com/microsoft/RD-Agent/blob/484776c211e4fbbeef03e0ec00d6bbee7362a4f4/README.md#L133)。
- [Qlib高频例子](https://github.com/microsoft/qlib/blob/be725493eb1a6bbb42bf11b37aa7669f59610ff1/examples/highfreq/README.md)、[Exchange源码](https://github.com/microsoft/qlib/blob/be725493eb1a6bbb42bf11b37aa7669f59610ff1/qlib/backtest/exchange.py#L44)。

## OpenProver的两个具名缺口

**声明检查。** `_check_proof_preserves_theorem` 在首个theorem/lemma/def之前截去前导文本，再按sorry分片匹配。前导 `variable` 会影响展开后类型，却不被该检查比较。本轮从固定版本源码通过AST仅提取两个纯静态方法，未导入框架、未调用LLM，得到以下局部反例：

```lean
-- 原题
variable (P : Prop)
theorem target : P := by sorry
```

```lean
-- 提交
def P : Prop := True
theorem target : P := by trivial
```

检查函数返回 `None`，即通过；本机Lean4.31编译后显示 `target : P := True.intro` 且无假定。原题要求对任意命题P证明P，提交只证明定义为True的P。这个局部检查不足以锁原题。没有运行完整框架或宣称所有上层评审都会漏掉该问题。原始输入和结果见 `openprover-preamble-check.json`。

**自定义模型。** 当前CLI的模型choices固定为七个别名；`HF_MODEL_MAP`和`VLLM_MODELS`为空。README写“任意本地模型加provider-url”，但该固定源码路径不能据此直接接入任意Router slug。即使补模型分派，所查HF客户端使用 `/health`、`/v1/chat/completions`，请求头只有Content-Type；尚不能据此保证所需鉴权、Responses接口、用量计数或Ultra服务档位。未做任何带凭据请求，DeepSeek可用性也没有被验证。

因此本轮不把“README支持多后端”记为“现有订阅已适配”。恢复功能同样只核到源码有保存配置及同版本恢复限制，没有断电或跨日长跑实测。

## 与当前工作流的连接

继续由现有goal和研究票登记任务、失败路线和证据。候选定义/命题先冻结；证明阶段只填证明，反例阶段核反驳命题与假定。市场侧以独立事件回放和A/B/B_seed/C对照为验收，框架只能提交候选，不得修改冻结终点或消费确认集来继续调参。

当前运行时没有父会话换模工具，用户要求派发前先切换父模型仍生效。本轮由主控进行资料核查、实现及自检，没有独立代理评审，也不以框架内部worker绕过该约束。后续真正接入外部执行器前，需要小型无敏感数据任务验证模型路由、预算耗尽、重启恢复、声明漂移拒绝及Lean失败传播；这些检查尚未完成。
