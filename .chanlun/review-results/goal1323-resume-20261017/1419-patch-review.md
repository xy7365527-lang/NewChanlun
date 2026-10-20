# 代码审查报告

审查树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`，基线 `018c6542e5`。先读未暂存 diff，后读作者报告；无代码修改、cargo、外盘或重型验证。`git diff --check` 通过。三个新增测试未执行。fixture 原字节核验采用主控已给证据，本审查未重复核验。

## 概念层质询结果

**发现矛盾，结论 FAIL（禁止作为可接线/已验收结构桥接受）。** 下面区分补丁改变的合同与基线已存在的阻塞；不把未接线本身记为新增回归。发现概念矛盾后不进入工程质量评分。

### 1. HIGH：第二种候选的同次折叠游标未被传入（基线设计缺陷，补丁未闭环）

`rust/src/bin/s_session_v3/content_structure.rs:45-59` 新建 `SecondFoldStore`，只调用 `restore_cursors` 就传给 `tb02c::build`，没有执行本次 parser。反例：传入任一 `segment_facts.candidates` 含 `second: Some(...)` 的真实 parser 产物，`rust/src/bin/s_session_v2/tb02c.rs:210-220` 查找该候选必然失败，返回 `IdentityConflict`。即使 prior 有 checkpoint 也不成立：`second_fold.rs:598-615` 的 restore 只写 restored；`cursor()` 在 651-652 只读 cursors；只有 `fold()` 的 625-641 才核验恢复并填 cursors。其因果与 `.chanlun/definitions/xianduan.md:205` 的“恢复即续左折叠”合同不一致。

最小改法：让 `StructureInput` 借用产生本次 `SegmentFacts` 的同一 `SecondFoldStore`；恢复必须在 parser 折叠前完成，结构投影只消费返回的 store。不要再次解析或把恢复 wire 直接塞入活动游标来假装同次事实。`tb02e.rs:1228-1251` 的测试适配已经示范正确顺序：parse_layer_with_inclusion_facts_store 返回 store，再传 tb02c。

### 2. HIGH：对象身份升级，但所有依赖对象和关系仍使用旧身份（补丁改变了对象 ID，基线另有合成身份缺陷）

`content_structure.rs:139-149` 现在用工厂生成 obj3 ID，而 `:109-114` 原样写入旧 tb02c/e/f 关系；`tb02c.rs:312` 的 witness_object_id 来自旧 Builder 的 obj- ID，后者配方见 `tb02a_facts.rs:246`。因此至少一条关系的见证对象指向本次对象集合中不存在的 ID；C/D payload 内 sequence_object_id / second_sequence_object_id 同样不改。相比018原函数保留旧 object_id，这个局部 diff 引入了明确的混合身份图，不能以注释声明“后续迁移”视为合同通过。不过018本身不具备可编译且正确的 TypedObject 构造，因此本报告不声称线上产生了回归。

最小改法：按真实依赖顺序构造对象并从返回的 TypedObject 获得 ID，把所有嵌套对象引用、relation witness、request subject 作为同一变更迁移；内容含引用时必须先更新引用再计算身份，不能只事后改字符串。增加闭包检查：每个对象引用在输出对象或明确授权 prior 中可解析。

### 3. HIGH：旧完整 JSON 再拆内容图仍违背已采纳生产构造合同（基线既有）

`content_structure.rs:49-60,79-97,129-145` 经旧 tb02* 完整 Value/JSON 后再拆成内容图。已采纳 `.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md:181` 明确要求直接从同核共享证据建值边；`ADOPTION.json:8-10` 确认采纳第5节。`content_producer.rs:1028-1030` 的工厂复用并不豁免调用者。该矛盾在018就存在，换工厂不能消除。最小改法是保留旧 JSON 桥为有界诊断 oracle，生产结构构造另从同次共享事实直接建边；或者按已有架构决策流程取得明确过渡合同，不能暗中降低要求。

### 仍应保留的基线阻塞

- E/F 的 `content_structure.rs:69-72` 使用合成 segment/object 身份和位置 i 来源，且 `:77` 永远无 developing 单元；需要真实 C/D unit 及来源映射，不能凭生成序号替代。
- `content_structure.rs:139` 使用旧 PROFILE/RULE_REVISION；与 v3 正式版本的绑定未闭环。工厂 identity 的 profile/rule 输入影响 ID，不能只在 manifest 改版本。
- 工厂 `content_producer.rs:1022` 默认 `request: None`；调用点未建立 C/D/E/F 的知识请求闭包。018也显式为 None。
- 本次局部文件未直接改变中枢/背驰判据，未发现新增宽松兜底。谱系目录针对本次身份/游标的检索未找到更具体的已结算豁免；#1344/#1420与#1394采纳合同是当前直接依据。

## 已排除疑点与测试边界（概念核验记录）

- **prior记录形状匹配**：工厂 `content_producer.rs:978-1006` 同时生成 payload 和 0x07 JsonText payload_json；后者规范渲染为 JSON 字符串，旧 tb02c/e/f 消费者可读。不能报“只有嵌套payload、缺payload_json”。018只存 payload 的错误在本补丁得到修正。
- `Value::Bool`、对象条目 Vec、`NumberToken::parse` 与当前 codec API 一致；`decode_prior` 不再静默跳过真实读取错误。数字测试覆盖 serde_json 已能表示的值，不证明保留任意原始数字 token（1/1.0/1e0）；当前修改不可被扩张宣称原件词法保真。
- 三测试确有行为断言：类型/数字域经真实节点写读往返；第二个使用另一 MemoryNodes 中的真实缺节点并确认错误不吞；第三个拒绝实际标量并检查非法 JSON。它们均未经过 typed_from_json/build_structure，未覆盖工厂 record→prior 消费、第二种游标因果、身份闭包、request或跨cut。测试价值有效，覆盖范围有限，且尚未执行。
- 新fixture描述自己是封存CC-013输入合同，不是现行parser结果；恢复原件可补文件缺失，但不能据此证明最新parser/全图通过。

## 工程层审查

未进入：概念层矛盾尚未解决。**整体 FAIL；可保留 codec 辅助修复作为未验收中间成果，不批准结构桥生产接线，不关闭 #1419/#1323/#1279。**
