# #1419 K1 恢复持费接入边界

工作草稿，绑定 #1419。状态：**PENDING_BUDGET_API**。本轮未收到 #1488 预算 0008 的最终精确签名及约束；按管理者最新协调令，不另造许可机制、不写 scratch/work/audit_owner/q/effects。以下是待统一接口到位后应用的具体边界，不是已经实现或已批准的 API。

当前已完成的独立部分：`SecondFoldCursor::from_wire` 用 `wire_match::matches(&probe.wire_view(), wire)` 直接比较借用 JSON。比对核只在栈上格式化数字、借用数组/字段，保留缺键、多键、bool/null、数字转十进制文本的旧比较语义；不再调用 `probe.to_wire()`、`wire.clone()` 或生产态 `normalized`。`probe` 的数学折叠工作区仍为原 parser 容器，不声称 Native 化。新旧比较核差分与实际非空恢复锁源码已落地，Rust **NOT_RUN**。

尚未修好的实际路径：

```text
generation.rs::generate（结构域分支）
  -> SecondFoldStore::new(round_budget())
  -> content_structure.rs::restore_second_cursors
     -> business_json::render_json(nodes, item.view())
        -> JsonOwner::Plain(PlainJson { JSON 树, 原许可, 子许可 })
     -> SecondFoldStore::restore(value.as_ref())
        -> restored: BTreeMap<key, serde_json::Value>
        -> wire.clone()                         [未准入的新树]
     -> 原 PlainJson 在循环末析构                [原许可释放]
  -> parse_layer_with_group_evidence_store(..., fold)
     -> segment -> FeatureSeqState 按值持有 fold
     -> SecondFoldStore::fold
        -> restored.remove(key)
        -> SecondFoldCursor::from_wire(&w, strokes)
           -> 重折叠 + 本轮借用比对              [已去除两棵临时 JSON 树]
        -> w 析构
```

不能仅延长 `PlainJson` 生命周期就给 `wire.clone()` 付费；两棵树不是同一份分配。本轮没有把剩余 clone 解释为 parser 数学工作区。

## 等待的具体接口事实

1. #1488 提供的**按值转交原 JSON 树与非复制许可**的拥有者类型/构造签名。当前 `PlainJson` 在 bin，`NativeValue::from_reserved` 在 lib 且为 `pub(crate)`，本轮不能假定 bin 可以直接调用。`child_permits` 的承接也不能丢失，虽然当前单 cursor 通常是 AggregateTree。
2. 恢复索引新增容量的唯一准入/持费接口及失败语义。优先复用 #1488 最终版 `CodecVec`/WorkScope；不把普通 BTreeMap 的条目分配藏在“已持有 wire 许可”里。若最终允许现有 `try_reserve_native/try_insert_native`，需确认参数顺序、同父与 scope 活性验证、释放/转移规则。
3. 原 owner 从 caller 移进 parser、再从索引移到 `fold` 局部时能否跨这几个调用作用域只读存活，以及该 owner 是否禁止 Clone。不能借老许可授予新复制资格。

## 可应用的补丁边界

| 文件/锚点 | 接口到位后的机械替换 | 不变合同 |
|---|---|---|
| `business_json.rs::PlainJson` | 仅新增消费 `self` 的拥有者桥，移动 value/原 permit/child_permits；落在 #1488 确定的公开承接类型上。不可 `clone`、不可 `into_unscoped` 拆出 Native payload。 | JSON 类型与原树不变；先毁 payload 后释放许可。 |
| `second_fold.rs::SecondFoldStore` | 恢复索引改为能保存该持费 owner 的非复制容器；Native 新槽先经当前 scope 准入。若用排序 CodecVec，保留 `(方向, b_stroke, P)` 的完整键、重复键最后值覆盖语义和按键取走。旧无 scope `.restore(&Value)` 是独立旧入口，不能成为 Native 准入失败的回退。 | 候选键、预算、verified/granted 计数和游标数学工作区不改。 |
| `second_fold.rs::restore` / 新 owned 入口 | Native 入参按值接收 owner；身份不合法时直接析构它；新槽准入失败返回 Err，未变更索引。重复键换入新 owner 后旧 owner 析构。禁止产生新 JSON clone。 | 身份不可读的历史 wire 仍不成为可恢复候选；无新判据。 |
| `second_fold.rs::fold` | remove 返回 owner；用 `owner.as_ref()` 借给现有 `from_wire`；匹配/拒绝返回后本地 owner 析构。 | 未核验仍按原合同重折叠并保留累计预算；不降级换判据。 |
| `content_structure.rs::restore_second_cursors` | 将 `match &wire` 改为消费 `wire`；Native 分支把 `PlainJson` 经统一桥移入恢复索引，并传实际 `nodes.codec_work()` 的 scope；传播真实准入错误。 | 仍先 verify_prior，拒绝 OwnedJson 不同域；不改规则/版本验证。 |
| `generation.rs` 结构域分支 | 如最终索引类型需要 Native 构造器，在此绑定实际 scope 后创建；一次 parser 调用及按值传 fold 保持。 | `ROUND_BUDGET=0`、正式结构规则门、已发布 prior 授权不改。 |

`SecondFoldStore` 现有 `derive(Clone)` 不能拿来复制非复制 owner。定向检索 `SecondFoldStore` 的全部消费者（parser/mod、segment、feature_seq、tb02c、content_structure、generation、s_structure_session 与测试），当前使用均为按值移交或借用；未查到其 `.clone()` 需求。接入时去掉该 derive，并再静核所有消费者；不要给许可实现 Clone 来迁就 derive。若采用泛型 owner，须同步以上 parser 签名，不能把生命周期/默认类型错误留给后续编译。

## 释放与待写的 Native 锁

- 身份无效：拥有者随入口局部析构；索引未变化。
- 新槽准入失败/失效 scope/异父：临时拥有者和失败准入各释放一次；不得覆盖原索引。
- 重复候选替换：新 owner 成为唯一现值；旧树先毁、旧许可后释放，不漏账。
- 成功/核验失败：`fold` 借用期间 owner 保持；核验结束后释放，再按原成功或重折叠结果继续。
- 从未消费的候选和 parser/generation 中途 Err：随恢复簿/调用帧析构全部释放。
- 必须补实际非空 Native 成功、消费前持费、消费后回收、未消费 drop、重复覆盖、预算不足与异父/失效 scope 锁。当前普通 parser 非空锁仅证明核验语义，不能替代这些资源锁。

默认预算 0 仍使新生成的 checkpoint 恒空；不改值、不删非空合同。最终统一源上的 Native 非空恢复、实际类型/借用检查与正式恢复均未验。
