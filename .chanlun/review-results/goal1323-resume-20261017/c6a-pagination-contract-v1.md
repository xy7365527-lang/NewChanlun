# C6a 分页接口 v1（主控已冻结）

日期：2026-10-17。依据：用户已批准 `c6a-wire-revision-proposal.md` 实施；主控已核认并冻结本稿字段、三op、统一query+fetch核心、整数文本、期限和预算归属，owner/Q/client 三工位共同执行。不得各自新增或改名字段。本文作者仅写合同，不改源码、不运行产品。

## 0. 适用入口与共同字节定义

- 页协议 payload 版本固定 `economic-read-pages/1`。公共 envelope 仍使用对应会话已登记的 `economic-session/1` 或 `/2`，不是新增经济写版本。
- owner Unix 接收 `EconomicReadOpen`、`EconomicReadPage`、`EconomicReadClose` 三个只读 op，纳入 `Config::authorized` 明确只读 ACL。
- Q HTTP 新增唯一路由 **POST `/api/economic/read`**，只接受上述三 op。现有 route末段等于op的检查须对此明确分支；不伪造 `/snapshot` 返回页。
- Open.query 在 owner 只接受原 `ReadView`、`Watch` 完整payload；Q只接受原 `snapshot`、`watch` 完整payload。原 original、金融写操作不进入此版本。
- `C(x)` 是现有规范JSON：UTF-8、键排序、无空白分隔、无非精确浮点/常量、重复键拒绝。`H(b)=SHA256(b)` 小写64hex，`JH(x)=H(C(x))`。
- 所有协议整数为规范十进制字符串：零为`"0"`，正整数无前导零，无符号/空白；bool不当整数。hash小写64hex，transfer_id小写32hex。
- 原完整**逻辑payload**为分页对象，不含外层逐请求envelope。owner为原 EconomicView/EconomicChanges；Q为原 EconomicSnapshot（含snapshot_token）/EconomicWatch。原经济图像与证明不得删改。

## 1. 精确字段集合

未知字段和缺字段均拒绝。下述 `schema_revision` 值一律为 `economic-read-pages/1`。

| 请求 | 精确字段 |
|---|---|
| Open | `schema_revision, op, query, requested_remaining_ms` |
| Page | `schema_revision, op, transfer_id, descriptor_sha256, offset, requested_remaining_ms` |
| Close | `schema_revision, op, transfer_id, descriptor_sha256` |

Open.query 保留原op及原完整选择参数。例如：`{"op":"ReadView","cut":null,"as_known_ns":null}`；Q snapshot 为`{"op":"snapshot","cuts":null,"as_known_ns":null,"snapshot_token":null}`；owner Watch 为`{"op":"Watch","after_cut":CUT}`；Q watch 为`{"op":"watch","cuts":CUT_VECTOR}`。

`requested_remaining_ms`为1至30000的整数文本。Page.offset必须是chunk_bytes整数倍且严格小于document_bytes。Close不产生新的运行窗口。

| 成功响应 | 精确字段 |
|---|---|
| Open | `schema_revision, kind, descriptor, descriptor_sha256`；kind=`EconomicReadOpened` |
| Page | `schema_revision, kind, transfer_id, descriptor_sha256, offset, raw_byte_count, chunk_sha256, data_base64, next_offset, done`；kind=`EconomicReadPage` |
| Close | `schema_revision, kind, transfer_id, descriptor_sha256`；kind=`EconomicReadClosed` |

### descriptor 精确15字段

| 字段 | 定义 |
|---|---|
| schema_revision | 页协议版本 |
| transfer_id | 服务端随机32hex；不得以正文hash代替传输身份 |
| session_id / session_generation | 原会话及代次 |
| manifest_hash | 原冻结manifest的JH |
| producer_id / producer_epoch | 实际外层envelope生产者完整ID及epoch，例如`E:s`、`B:chong-N1:s`、`Q:s` |
| query_sha256 | JH(Open.query)，不包含remaining_ms或外层信封 |
| document_kind | 完整逻辑payload的kind；owner仅EconomicView/EconomicChanges，Q仅EconomicSnapshot/EconomicWatch |
| fixed_cuts | owner `{自身domain:原cut}`；Q每个配置域一个键，Available为原cut，Unavailable为null |
| owner_epochs | owner `{自身domain:本epoch文本}`；Q为全部配置域的expected_epochs，不能从返回正文反向抄成可信来源 |
| document_bytes | 完整C(payload)长度，正整数文本，≤4194304 |
| document_sha256 | H(C(payload))，Q snapshot包含snapshot_token |
| chunk_bytes | 固定`"262144"`，等于1MiB/4，作为分片步长 |
| page_count | ceil(document_bytes/262144) 的整数文本，1至16 |

`descriptor_sha256=JH(descriptor)`。描述符本身不是授权令牌：服务端查已签发状态并逐项匹配；传输还绑定Open外层原调用者身份。身份为session/generation/source_namespace/source_epoch/producer_id/producer_epoch，不含每次不同的message_id。任一不符拒绝。

Page.raw_byte_count=`min(262144, document_bytes-offset)`；data_base64为该区间的标准RFC4648 base64（含需要的`=`，无换行，解码后重新编码必须相同）；chunk_sha256为原片字节H。done当且仅当offset+count==total；末页next_offset=null，否则为offset+count整数文本。末页之前禁止空页。传输完整envelope仍必须实测≤1MiB，owner计末尾LF，HTTP正文计envelope、HTTP头另沿既有16384B接收余量；固定stride不免除实际帧核验。

## 2. 错误与关闭

已验证公共头的页协议错误统一返回合法envelope，payload精确：`schema_revision,kind,code,detail,transfer_id`。kind=`EconomicReadError`；transfer_id未取得时null，否则原ID；detail为无凭据的可读文本，不参与错误分类。

| code | 条件 |
|---|---|
| SchemaUnsupported | 页版本、op、query类型未支持 |
| InvalidQuery | 字段/类型/整数/base64/offset边界非法 |
| IdentityConflict | 会话、源身份、epoch、descriptor或同open消息ID异payload冲突 |
| MissingDependency | 指定历史cut确实不存在；不能换最新cut |
| SnapshotExpired | 原snapshot_token缓存已失效 |
| TransferExpired | 原传输截止已到或ID不在本进程有效状态 |
| QueryBudgetExceeded | 帧/逻辑/共享缓存/数量/时限/变化数上限触发 |
| SourceUnavailable | 权威读取不可完成且无法形成原协议定义的逻辑结果 |
| IntegrityError | 持久root、长度、正文或片hash与冻结身份不一致 |

Q对已验证请求的协议成功/错误均HTTP200并使用原reply envelope；无合法envelope的HTTP/公共头错误保留现有400/503传输错误口径。客户端必须将EconomicReadError当失败，不能交金融oracle。owner不要再把此payload套成另一层`kind:Error`；在既有handle内部形成合法页错误结果后交统一reply_envelope。

Close匹配活动ID/descriptor/来源即释放传输。活动ID身份不符必须IdentityConflict。不存在/已关闭且字段格式正确的ID可幂等返回EconomicReadClosed（只回显，不创建任何状态、不证明读取成功）。Snapshot close释放pin但保留原可缓存snapshot；watch/owner close释放正文。

**成功交付还要求Close成功**：所有页重建、hash/strict decode/协议投影核验完成后，在原根deadline内收到原ID/descriptor精确匹配的EconomicReadClosed，最后再核同一deadline，才允许helper向调用方返回完整逻辑payload。Close超时、异常、错误响应或字段不符均使本次逻辑读取失败，即便正文已经正确也不得交付成功；不得靠缓存副本绕过。若前面已失败，finally尽力Close，保留首个失败，次生关闭失败另记证据；超时后的清理受外层原清理窗约束，不能复活本读成功或另开30秒。

## 3. 固定切面、重送与epoch

- owner Open在短事务中核epoch/audit并固定ReadView cut，或Watch base及next cut。Watch完整变化数≤32且严格连续；分页不可每次重新查询当前latest。记录原runtime等诊断字段，整个逻辑payload字节固定。
- owner页可读已冻结buffer，逐次重核原epoch与所选cut/root；不每页重做全域audit，不跨请求持有事务。恢复/epoch不符拒绝旧transfer。
- Q先按原语义获取并核各域，再固定完整逻辑对象；Unavailable原样保留。fixed_cuts相应域null，后续页不重查该域补成新值。完整页传输不等于CompleteVector；C6a正常轨继续由原断言拒PartialVector。
- Q页仅检查本Q有效状态和固定文档，不每页重新RPC所有owner。descriptor.owner_epochs是取证时绑定，不把历史完整快照强制变成实时可用性查询。
- 精确Open重送以调用者身份、message_id、payload_hash识别，返回原descriptor且不延长deadline；同消息ID异payload拒绝。Opening未完成的重复返回SourceUnavailable或等待原操作剩余窗，不能并行创建另一份。
- 关闭/过期Open键保留有界终态元数据至本serve进程结束，计同缓存字节；满则明确QueryBudgetExceeded，不删键后让旧Open复活。终态元数据不占4个正文项，但占4MiB。无需新增持久金融去重记录。
- 进程重启后旧随机transfer无效，即便配置epoch字符串相同也不能复活。snapshot旧token是否可读仍按原cache/历史合同；不得以transfer有效性覆盖它。

## 4. 30秒跨进程合同

**wire禁止monotonic绝对值。** 根调用方在Open之前用自己的单调时钟建30秒截止；若case/work或调用链上游更早，取最早。Open/Page只传当前剩余毫秒向下取整，≤30000，≤0不得发送。根调用方在发送前、接收后、每片核验后、完整hash/strict decode/语义核验后再次检查同一截止，迟到绝不接受。

接收端首次收到Open时用自身本地clock建立`receive_now + requested_remaining_ms`，并与已有本地operation/ingress截止取min；保存于transfer状态。Page的remaining_ms只能进一步收紧此次工作窗，不能延长该transfer原截止。Q调用owner仍使用Q当前剩余毫秒。不得把外来毫秒数解释为延长本地已存在预算，更不能用客户端声称的新deadline恢复过期状态。

**精确边界**：相对预算不能证明跨进程本地截止等于源端截止（传输有延迟）。因此30秒成功判据必须由源端全程绝对门兜住；服务本地窗是额外界限，不能单凭它声称端到端30秒已证。根超时停止接收/成功发布并尽力Close/取消；接收端仍受本地≤30秒、既有ingress及外层进程监督限制。验收须注入Open/Page传输延迟，证明源端迟到拒绝，不能只检查服务计时。没有声称在无共时钟/无即时取消的系统里，服务端所有CPU会在根30秒同一瞬间停止。

Q `_start()` 不得在每页重开完整窗。原case300/work600/final630与外层五端点不变。原资源IO超时若更短仍取更短，不扩大owner response_timeout_ms。

## 5. 缓存与所有权

- 每逻辑C(payload)≤4MiB；Q的snapshot及activewatch共享最多4正文项、4MiB实际持有的canonical正文及元数据总帽。cache保存唯一完整canonical payload（snapshot含token）；snapshot_token仍是原body（去snapshot_token字段）摘要，身份算法不变。
- active transfer pin，不淘汰在读正文；无可回收空间立即拒绝本读，不阻塞经济写者。页副本/decoded对象/base64临时buffer仍计真实运行组RSS2GiB，不能只记cache数字。
- owner新只读缓存同样局部最多4正文项/4MiB（含metadata），且全部实际内存仍计原全运行组2GiB。这不是声称全机所有进程合计cache仅4MiB，原4MiB Q共享帽保持不变。不能再额外放一份无帽owner正文库。
- snapshot重复token可共享同一不可变正文，metadata与多个transfer仍实际计费。watch关闭即释放；snapshot关闭后可供原token重读直到按原有界缓存策略淘汰。终态去重键也计费，不无界增长。
- 开读前做长度上限预检，再构造；失败不留下半份可用正文。descriptor、正文及pin一次性提交至缓存，写到一半不签EconomicReadOpened。open成本与构造峰值都在原期限及资源内。
- 所有页原件、重建临时件和归档仍在受监控文件系统，计原增长6GiB及余量门；不新开外盘路径。

## 6. Python统一客户端职责（签名供双方接线）

共同页解析/组装逻辑只在`s_session/s_socket_client.py`一份；Q和driver共用生产侧helper，**独立semantics另写独立验证，不靠同helper自证**。

建议冻结公共helper为：

```python
read_economic_pages(query, fetch, *, session_id, session_generation,
                    producer_id, producer_epoch, manifest_hash, owner_epochs,
                    deadline, clock=time.monotonic,
                    max_frame_bytes=1048576, max_document_bytes=4194304)
```

`fetch(payload, remaining_ms)`由调用方提供，负责生成原公共信封、按HTTP或Unix真实发送、保全原wire、核每帧容量并用原validate_reply核身份/因果/payload hash；返回已核页payload。fetch不得重开根窗。helper核本合同精确字段、descriptor绑定、页顺序、base64/长度/hash、完整strict decode、document_kind与fixed_cuts/owner_epochs对应，返回原完整逻辑payload；成功/失败finally在原允许清理界内Close。caller仍负责原完整业务oracle及整个逻辑操作末尾deadline检查。

producer_id参数是完整ID；fetch仍可向现有validate_reply传其既有producer短名，不能把两者混用。owner_epochs由调用方可信配置提供。Q域不可用时descriptor允许null cut，但decoded正文必须确实为相应Unavailable；不能因此放松其他域cut核验。

helper默认数字仅复用已批准常量；调用方不允许在C6a传更高值。若本地已有更低帽则取min。canonical生产编码与Rust原canonical_json按共同原字节向量对拍；两边不得各自选择字段/数值编码。

主控另允许`exchange_economic_pages`便利wrapper：它仅构造可信fetch闭包、公共信封与Unix传输并委托上述唯一read_economic_pages核心，不得包含第二套descriptor/分片/截止/错误判断。wrapper参数由client与Q共同接线；共享核心签名按本节冻结。

### 重建逻辑payload到descriptor的精确投影

对所有kind，先核`document_kind == payload.kind`及完整C(payload)字节数/hash；正文重新canonical编码必须逐字等于拼出的原字节。然后按下表核，不能从正文更新调用方预期的manifest/epoch。

| kind / 原query | descriptor.fixed_cuts 精确投影 | 其他必核对应 |
|---|---|---|
| EconomicView / ReadView | `{payload.domain_id: payload.cut}` | domain_id必须是owner_epochs唯一键；writer_epoch等于该预期epoch；cut.domain_id等于该domain；image_hash=JH(image)；非空query.cut必须等于payload.cut；非空as_known_ns要求commit_ns≤as_known_ns |
| EconomicChanges / Watch | `{payload.domain_id: payload.cut}`，该cut为Open固定的next | domain_id与writer_epoch同上；每条change.cut域一致、image_hash=JH(image)、commit_seq从query.after_cut+1连续至payload.cut；after_cut=null时从创世序号0起。空changes要求非空after_cut且payload.cut等于after_cut。变化总数≤32；末尾cut等于payload.cut；每条时间不倒退 |
| EconomicSnapshot / snapshot | 遍历payload.domains全键集合：Available取row.cut，Unavailable取null；不允许第三种status | domains键集合等于可信owner_epochs全键；session/generation/manifest_hash核可信配置；Available的cut.domain_id匹配键、image_hash=JH(image)；completeness必须是全Available时CompleteVector，否则PartialVector；snapshot_token=`economic-snapshot-`+JH(去掉snapshot_token的原body) |
| EconomicWatch / watch | 遍历payload.domains全键集合：Available取row.next_cut，Unavailable取null | domains键集合等于可信owner_epochs全键；session/generation/manifest_hash核可信配置；每Available.row.base_cut必须等于query.cuts[domain]，变化按base序号连续至next_cut，单域≤32，每change图像摘要/域/时间顺序正确；空changes时next_cut==base_cut |

descriptor.owner_epochs必须逐位等于可信调用方传入的map：owner只含自身domain一项，Q含所有配置域。Q Available图像没有writer_epoch字段时不得补造；其epoch绑定来自原取证时可信配置与已验owner envelope。owner逻辑payload未带session/manifest字段时，这些绑定由已验证公共头及descriptor提供，不在原正文添加字段。

Snapshot额外选择检查：query.snapshot_token非null时返回token必须相同，原mode/as_known保持缓存原件，不按这次query重写；token为null时mode按原规则（cuts或as_known_ns非null则AsKnown，否则Current），payload.as_known_ns等于query.as_known_ns。query.cuts非null时，各Available域cut必须等于给定cut；Unavailable域仍为null冻结。时间与图像模式的其余原业务检查继续执行，不因分页减少。

Watch原Q逻辑中“变化图像等于同cut ReadView、base根确为权威历史”的额外读核仍由原Q及独立oracle执行；单靠上述页投影不能从一个摘要证明其原始来源，不得删掉原读核。部分域Unavailable也不能让正常轨C6a通过原完整断言。

## 7. 纯数据示例与必要锁

Open（不带外层公共头的payload）：

```json
{"schema_revision":"economic-read-pages/1","op":"EconomicReadOpen","query":{"op":"ReadView","cut":null,"as_known_ns":null},"requested_remaining_ms":"29990"}
```

Page示意（ID和hash由Open实际返回，不得使用此占位值实跑）：

```json
{"schema_revision":"economic-read-pages/1","op":"EconomicReadPage","transfer_id":"00000000000000000000000000000000","descriptor_sha256":"0000000000000000000000000000000000000000000000000000000000000000","offset":"0","requested_remaining_ms":"29900"}
```

codec原字节向量：`{"ok":true}`，11 B，base64=`eyJvayI6dHJ1ZX0=`，SHA256=`4062edaf750fb8074e7e83e0c9028c94e32468a8b6f1614774328ef045150f93`。这是**字节codec向量，不是合法EconomicView业务正控**，不能把它计作41步或金融成功。跨语言对同一字节串独立算SHA256并比较；正式测试fixture另提供四种完整逻辑payload。

必需正控：四种logical kind；1页/跨262144边界/恰4MiB文档（共享metadata不足时应保守拒而非越界）；UTF-8及转义跨片；原完整RAW+proof重组逐字相等；相同Open重送无续窗；同页重送载荷一致；close幂等；原snapshot token重读；watch固定base/next及逐cut相等；Unavailable原样保留；所有41步/每步所有观察断言不减。

必需负控：同ID异payload、错producer/epoch/manifest、跨cut/描述符hash、字段多缺、非法数、错base64/片hash/总hash、重叠/跳页/空非末页/伪done/尾垃圾、声明长度>4MiB、完整frame>1MiB、缓存pin耗尽/metadata耗尽、终态Open复活、延迟跨30秒、page试图刷新deadline、慢Q不拖健康owner/S。负控失败不能替正控成功。

## 8. 工位接线与变更纪律

- owner：service/store只读入口、ACL、serve closure状态、原字节有界构造、各本地截止；不改金融reducer。
- Q：economic_query/economic_http共享cache、逻辑query固定、HTTP路由、调用统一helper；不得再独写第二页格式。
- client：s_socket_client.py统一helper与纯替身测试；driver/semantics后续分别接其生产/独立验证界面。全部新增实际wire纳入证据，不减少请求断言。
- 本稿已由主控冻结。任何变更先通知主控和全部三工位，经主控核认后再改一处合同；以文件为准，不以较早聊天中字段计数误写为准。

本稿不证明当前真实文档/时间/内存已经可验收；所有开关仍待新源码、协议独审、全41步及资源/恢复证据闭合。
