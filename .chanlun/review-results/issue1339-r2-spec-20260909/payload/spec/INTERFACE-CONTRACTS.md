# #1323 SPEC 接口合同附件

名分：S2 的具体接口提案，待独立复核与同份 SPEC 批准；不改变 R2 的职责、条件阶段或两段交付窗口。消息名是模块接口，不冻结网络路由或具体代码文件。所有写调用只能由相应获准主体发起。

## 共同消息与身份

每个跨进程请求都含 schema_revision、session_id、source_namespace、source_epoch、message_id、producer_id/epoch、payload_hash、causal_refs。结构坐标、账版本、来源游标、命令 ID 分字段；不同来源的数字不可比较。原始 received 时间、真实 first_known、持久提交序号和展示时间分别保留，不能互相替代。

身份与内容绑定是所有权威写入口的共同合同：同一逻辑身份、同内容重放返回原持久结果；同身份异内容返回 IdentityConflict，不覆盖。输送尝试 ID 可以变化，业务 message/command/fact 身份不能随重试变化。观察请求不会铸造写入资格。

生产数值保留单位、货币、tick/quantum 和来源 policy；跨语言 wire 中精确整数使用十进制字符串，不能由 JavaScript Number 静默舍入。原始价格/费用的文字表示与规范值并存；没有获准的量化、舍入或换算规则时记录缺项，不用浮点 epsilon 改动严格端点或资金界。schema 的规范序列化有字节级测试；payload_hash 绑定规范字节，不能仅散列任意 JSON pretty-print。

共同错误分为：SchemaUnsupported、IdentityConflict、StaleWriter、InvalidDomain、MissingDependency、AwaitingPolicy、UnsupportedCapability、StorageUnavailable。写调用失去响应时客户端只知道 DeliveryUnknown，须 Query 原身份；它不是对权威域状态的断言。某个错误是否可重试由下表和原身份规则决定，不设“所有网络错都自动重发”的全局中间件。

## 接口、提交边界与读写权

| 入口 | 必需输入与 owner | 持久成功含义 | 失败/重放合同 |
|---|---|---|---|
| S.AcceptInput | 原始事件与修订、来源顺序/证据、scope；S 唯一输入接纳器 | S 自有输入记录可恢复；不是结构计算完成 | 未确认只查/补相同输入身份；不要求 E 已接纳 |
| S.Advance | 待处理输入、规则/目录版本；仅 S 结构写者 | Begin 已封相关门；完成时 Commit 将完整结构批次与可达索引一起公布 | 中断保留 Begin，恢复仍闭门；任何实际判定前先有本地屏障 |
| S.ReadCatalog | session、catalog_revision、scope | 返回完整适用轴、域/分支、来源与当前实现/证明状态 | 无当前实例也返回目录；未实现不隐去条目 |
| S.Snapshot | session/generation、scope，可选指定 structure_cut | 返回固定切面 token、完整对象/关系/见证索引及分页位置 | 不跨 cut 拼页；失效 token 返回显式 Gap/需要重建 |
| S.Watch | 同会话/代际 token 与 cursor | 返回 base_cut、next_cut、seq_range、原子变化批次 | 重复不重生效；缺口返回 Gap 和可用重建 cut；不等待经济 ack |
| O.ReadOperation / Locate | 已发布 Classification 引用、重操作级别、定位条件/明确 Sel policy | 只读返回分解、候选全集、固定选中及完整适用判据/证书；不写主塔 | 选中失败不重选；N/A/未裁/缺证据独立承载 |
| B.PrepareDecision | 重/Voice、消费的结构及操作结果、量/政策/权利版本、attempt_id | 本重持久 Prepared、锁定权利/计划及必要原料；尚无外效资格 | 不能把缺政策当 Hold；同身份冲突拒绝 |
| E.Reserve | 固定 Prepared 引用、全部潜量、slot/申请代际及完整共同证据 | 在 E 事务先增加责任，返回绑定本重/计划/证据的唯一 grant | Pending/缺证据/争用缺政策不能放行；不读坏账来取已独立保存的责任上界 |
| B.CommitDecision | grant、原 Prepared 及本地版本 | 本重提交并返回永久 Application/Commit 收据 | 原 Committed 不改写为未发生；收到旧 grant 须核其当前范围 |
| E.MakeDispatchable | 本地 Commit 收据、grant、冻结计划 | 保存可交付的财务状态；仍不得访问场所 | 旧状态不能代替随后 Release 的当前条件核验 |
| E.ReleaseExact | command_id、logical_family/revision、plan_hash、完整责任/条件证书、唯一 X/epoch | 与 Capture/Pending 同序写 ReleasedExact 和不可改交付对象；此后才交付 X | 先持久后调用；失响应保全责按原身份查，不造新经济动作 |
| X.AcceptExact | 完整 ReleasedExact 与权威身份/内容/目标证明 | X 记录原命令接纳或返回原状态；本步不是网络调用权 | 不能拆成未来候选或改量改价；重复只查同命令 |
| S.RequestConditionalExecution | 固定命令/证书、X 身份和 epoch | 持久本地就绪身份，或返回已有 S 结果；在稳定点核验后才有 ExecuteCommitted/最终拒绝 | busy 不终态拒绝；重复不增加队列项；响应丢失可 Query 原 ID |
| X.OwnAttemptAndCall | 原 S 收据、plan/certificate hash、epoch、所有调用阶段条件 | X 同 command 唯一 CAS Ready→MayHaveCalled；仅本次成功者可尝试一次受控场所调用 | MayHaveCalled 以后不因进程重启、旧收据或查询无记录补发；SDK 内写重试亦受约束 |
| E.CaptureExternal | 原字节或已持久可读 blob、可信连接身份/范围、token、source epoch/cursor、received 时间 | 同事务写 Recorded 和保守 Pending；随后才可解释风险/用于经营 | 未规范接纳的缓存不算 Recorded；字节不可读保持 Pending，不能只留无实体 hash |
| E.ResolveEvidence | token、完整来源前沿、依赖版本、影响范围/包含证明 | 同安全事务更新已核证据/责任，只移除被证明覆盖的 Pending | 旧证据/新 token 不被错误清除；失败保持封闭 |
| B.ApplyAllocation | fact/allocation_id、归属重/Voice/family、数量/现金/费用/更正关系及已批分配政策 | 本地唯一应用并返回持久收据；真实经济事件驱动毛账/成本/阶段 | 重复同内容无变化；异内容冲突；缺前件/配对保留未匹配，不归新 Voice |
| E.ReduceResponsibility | 特定责任或已分配份额的不可再执行/累计完整证据与所有实际参与账收据 | 同一责任界在 E 原子下降/关闭 | 缺任何前件保原潜量；不要求不相关 A 的 Applied；无记录和超时无效 |
| E.TransferRights | 唯一 transfer、kind、源/目标/权利、policy/evidence 与双边回执 | 完成源锁定→共同托管→源扣出→目标待激活→唯一交付激活 | 未激活/未结尾责不能超时退款或暗迁；恢复按原 transfer 决议 |
| Q.ReadEconomicView / History | session、指定域前沿或 AsKnown 依赖向量、scope | 返回每个域真实 cut、consumed_structure、完整性、缺项和权威状态 | 不阻 S；不把恢复应用回填旧时刻；重算版本另模式 |
| QueryCommand / QueryFact | 原身份、期望权威域、可选 last_known | 返回权威状态/查询前沿，或明确 Unavailable；可供恢复取证 | NoRecord、Undecided 和 Unknown 各有含义，查询本身不释放责任或产生新外效 |

## 正式结果型

StructureRecord 包含 object_id、kind、rule_revision、input_refs、object_revision、lifecycle、knowledge、validity、scope、axis_results、relation_refs、witness_refs、first_known、published_cut。每个 axis result 绑定 CC、有效验收类型和量化对象域：真正分区按具名子轴返回唯一分支及良构域证据；角色、集合、公式或描述分别返回全部适用成员、值或事实，不强塞为互斥枚举。混合目录同时保存各子轴与非分区义务，缺证据和不适用按真实原因承载；组合事实按 LC 保留。没有实例是实例集合为空，不能省掉目录。

StructureDelta 包含 session/generation、catalog_revision、base_cut/next_cut、seq_range、完整 upsert/撤回/替代及关系变化、见证引用和结构索引前沿。对象撤回的原身份、首次获知和历史链可查询；只是从当前视图移除，历史不删除。跨域消费关系单列其经济索引前沿。

BookView 包含重身份、权利来源及版本、各 Voice 生命史、TStage/目标/成本事件、固定 Q/实际量/增长权益、毛责任/预留、实际已应用 fact/allocation、未匹配与尾账，以及当前消费的结构版本。未裁指标保原始输入和 policy 缺项，不输出 0 冒充计算结果。

ExactCommand 包含 logical_order_family、首次量/物理 revision/分配身份、command_id、固定买卖/量价/场所/投影、责任引用、E-Release 收据、适用条件清单、结构/规则/市场依赖、唯一 X/epoch。不同 revision 不另造首次量，改单/取消亦有自己的指令身份与原 family 关系。

ConditionEvidence 每项明确 condition_id、authority_revision、dependency_set、invalidation_sources、required_check_stages、evidence_refs、coverage_proof、validator_capability 和失败结果。所有适用前件由获准语义/权限/场所 manifest 展开，命令作者不能选择删去检查项。这里的通用型不证明任何具体场所条件已可核。

## 持久化最小原子组与恢复读集

- S：输入身份与接纳顺序；Begin 与 scope 门；Commit 的对象/关系/修订/索引可达根；就绪身份与顺序；最终条件结果各自是明确本地原子组。Commit 对外只在引用的全部批次对象已持久可读后发生。
- B：Prepared 与本地权利锁；Committed 与本地收据；同 fact/allocation 的唯一应用、经济变化和回执同组。账坏时 E 仍能只读自身独立责任证据判断健康重。
- E：Recorded 与 Pending；Resolve 与 token 集合更新/证据/扩界；reserve 与增加责任；Release 与固定交付；特定责任减保留与所需回执证明；transfer 托管/唯一激活决议。各组在本 E 顺序中提交，不冒充跨库事务。
- X：命令身份/内容永久绑定；Ready→MayHaveCalled 的原子占有；真实调用/回报的观测记录。MayHaveCalled 到外部场所之间没有伪装成数据库事务的全局原子保证。

每个域重启先恢复本域未决状态、引用完整性和 writer 代际；然后按原身份读取其他域所需证据。恢复读集不能包括不相关坏账；E/S 分区期间只关闭确实依赖缺失证据的新操作，S 正式历史照自身健康状态推进。

## 成功路径和失败路径的验收界限

类型可编码、schema 校验通过、同 ID Query 可调用，仅证明接口形状。完整验收仍要实际构造所有范围内对象、运行经营成功路径、通过跨进程故障/调用隔离试验并向同源前端交付。此附件不会把未裁政策、场所能力或 G 类证明改成“已实现”。
