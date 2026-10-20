# #1488 C1a 与来源身份上游接口独立审查

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1488`。基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。本报告只读源码；未修改生产代码。审查对象为 C1a 的 source_audit/source_audit_oracle，以及主控的 for_source_audit、witness_identity_native 接口。其他并行变更仅作为调用上下文阅读，不据此签发整片认可。

## 概念层质询结果

**限定通过。** 本片改变分配所有权，不改变缠论判据。已检索知识库、definitions.yaml、docs/spec、教义定义目录以及谱系；未发现本片新引入教义定义或已知矛盾。资源侧按已裁定的 `1488-raii-migration-plan.md` 核：原许可从实际构造点移动；费用随真实拥有者；缓存只存不可变原件；每次消费仍核当前 cut；完整同父帽、原六成功不变。

- 字段 memo 键仍含完整 format/root/kind/extent/digest；请求名仍保序、保重复。结果缓存取规范唯一字段，不用请求名中的重复项构造重复字段；未改成宽松准入。
- witness memo 保留 capture/object_id/slot/mi/digest/kind/extent 身份。完整原件闭包在查命中前验证；overlay 不进入缓存；纯函数成功且 stored ID 相等才插入。坏 ID、坏头、缺闭包、不同 capture、重复/乱序字段是已核源码中的反例边界。
- 冻结 oracle 的差异只有初始化 owner、空文本类型接线及同配方 witness 返回类型。原独立遍历和业务条件未被新增量来源核替代；未发现严格核失败后改跑宽松核。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**PASS（仅本片静态审查）**。没有发现置信度超过 80% 的新确定缺陷。不是编译通过、资源实测通过、六成功通过或 #1488 交付通过。

### 原费用点、同父复制与销毁

1. `source_audit.rs:89` 的 NativeFieldRefKey、字段名 CodecVec、scalar/fields/witness/原件索引的 BusinessMap 均从当前 CodecRead 的 scope 准入；删除了对应旧 native_admit/native_std_map_room，未见新旧重复收费。值和桶许可分别由其实际 owner 持有。
2. 字段命中在 `:592` 通过 `NativeRefFields::try_from_views(..., self.source.codec_work())` 独立复制；旧 memo 不退账。scalar 命中在 `:483` 通过当前 native_json_clone；miss 的缓存副本另收费。临时查找 key 在分支返回后销毁，不能把命中当成直接移动旧缓存。
3. NativeRawLedger 的 originals 从构造到 into_facts 原样移动，clear 的 retain 清条目而保桶，最终 BusinessMap drop 才释放桶。sources 的替换是实际销毁旧 map。RetainedDelta 只记统计，未在本片引入按统计退款。
4. `ArrayAuditSource::codec_work` 直接转发当前 source；NativeSourceAudit 的 with_codec_work 同时设置 reader 与字段 memo。未在本片发现新建第二父账或存旧 scope 以便跨请求扩容。

### SourceIdentityProducer 的精确承诺

`content_producer.rs:667` 与旧 source/oracle 两处初始化收费位置相同，先检查长度溢出并签发 profile/rule 两段文本与两个 Rc<()> 控制块的原结构上界，再调用 new。结构字段 inner 在 _initial 之前声明，producer 的实际字段先析构，许可最后析构；此顺序覆盖正常返回和随后 drop。

wrapper 无 DerefMut，Cc004Producer 无内部可变缓存，未暴露初始字符串所有权或裸 Rc 克隆口。`parser_fractal_sources(&self, ...)` 虽返回 Rc，但先查 prepared 的同 owner/snapshot；合法 prepared 只能经需要 `&mut self` 的 prepare 路径建立，wrapper 不提供该路径，其他 producer 的 prepared 会被拒绝。借用返回由 Rust 生命周期束缚，未发现初始两 Rc 的可达逃逸。

**Deref 只限制 producer 的可变访问，并非“所有只读方法都零分配”的能力封闭。** 例如 build_object(&self, ...) 和 typed_object_id(&self, ...) 仍可另造输出；其分配不由初始化许可覆盖。当前来源消费调用的是显式 scoped 的身份方法，本片没有将初始化收费外推成整个可变 producer 或所有只读输出已收费。

### witness 同核和错误顺序

`generation.rs:1711` 提取的 witness_identity_bytes 保留 object_id 转义拒绝 → 原件 ref → 原 renderer → SHA256 的原顺序与输入布局。只将最终 wit- 加 16 hex 的构造改成固定 20 字节栈数组。旧 String 入口仍在最终分配前 native_admit；新 Native 入口在相同位置使用 native_string、让返回值持有许可。没有新旧双签，也没有预先把原件完整渲染成堆字符串。

source 的完整原件验证、当前 cut 条件与缓存 success-only 次序均保留。资源实现变更导致实际峰值和分配失败点可能变化，不能靠静态阅读宣称资源失败轨迹逐次相同；需后续实际测试。

### SourceFields::get 下游签名单列

当前 get 返回借用 RefView。所读工作树中直接消费 get 的三处值读取均为本文件测试，已去掉旧 `.view()`，接收方 built 直接吃 RefView。生产调用与 source_pending/source_pending_oracle/source_contributions 仍用 get_admitted，未看到将借用 RefView 传给旧 `&RefValue` 参数的遗留点。SourceFields 本身仍单字段包装 RefFields；借用不能越过 SourceFields 的寿命。此结论是当前调用点静态枚举，不等于 Rust 类型检查已通过。

## 测试源码与验证边界

- 新 raw 生命周期测试确实给 ledger 三条非空事件，依次 advance、rewind、重走并比较原件 hash 复用，再 into_facts；不是只构造空容器。该 helper 是八字段原件身份夹具，OHLC 缺项投影不代表完整市场历史验收。
- 新 producer 生命周期测试确实使用非空 profile/rule，调用真实 typed_identity_scoped，与普通 producer 同核对照；载荷 Null、refs 空数组适合初始化所有权测试，但不证明真实多对象完整来源审计。
- 既有 witness 测试用非空 bars、成功插入/命中、错误 stored ID、不同对象/slot/mi/头、缺闭包和 overlay；真实生成态 current-cut 测试直接读取第一个 witness，空 witness 会失败而非悄悄跳过。
- **现有 witness/field memo 测试主要用无 scope HashReader。新增两项 scoped 生命周期测试不覆盖字段命中副本与 witness memo 的实际父账 drop 读数。** 这是尚未获得的验证证据；不能用本报告替代这两类实测，也不能将无 scope 逻辑对拍当作持费已测。
- 执行了四文件 `rustfmt --check --edition 2021 --config skip_children=true` 与限定 `git diff --check`：均 exit 0。已看 staged/unstaged 统计；这四文件无 staged 差异。
- **NOT_RUN**：cargo/rustc、Rust 单测、六成功、运行峰值/回基线、全历史、远端 CI。未启动 HTTP/正式服务、未访问外盘、未合入或关票。
- 旧裸 CodecRead/frame 兼容 retain、其他消费线和全 producer 分配尚未闭合，属已声明外部范围，本报告不把它们标成已解决。

## 冻结 SHA256

| 文件（rust/src/bin/ 下） | SHA256 |
|---|---|
| s_session_v3/source_audit.rs | cfc7587865864de4ec8391bace3b9e054283d9bfb7963531b309aa702b87062d |
| s_session_v3/source_audit_oracle.rs | 41a7c0681eb801af236cbc0dda7f08fd6f8eac8ab140b00945a55ff2f02f51bf |
| s_session_v2/content_producer.rs | d149f818ce299f17dce9ef638859265d1db1265c9418d64ebe7b99dfc5ef189b |
| s_session_v3/generation.rs | 2c32f7b49db8ba3e2acf59c86c7dcee08b69f4e30eed38d63f48ce9ac522ae0d |

图查询对本片新增符号返回零结果，已据此回退精读实际恢复树源码；未将主仓图当作恢复树新符号证据。
