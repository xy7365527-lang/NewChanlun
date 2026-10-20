# #1394 独立 Spec 审查报告

结论：**Spec PASS**。概念层通过，未发现本候选违反已限定合同的真实反例。此结论仅覆盖同次 SQLite 已核帧传递，不是工程、性能、合入或整图验收。

基线 `51855354d06e2e84aa697f1a1772de15af2fc769`；候选 `c8d850af7c3adab7e9b9c36c270f3cae92f84101`。完整差异仅两个 Rust 产品文件及 roster；暂存和未暂存产品差异为空。审查者非作者，产品与构建目录只读。

## 概念层质询结果

本变更涉及存储证明边界，不新增缠论教义。采用已批准 `APPROVED-SOURCE.md` §5.6/5.6.1（:276、286、292、301–305、317）及现有 `s_content.rs:413–427` 同次读取合同；批准源与指定诊断 SHA 均匹配。

- **定义回溯：** `content_store.rs:90–106` 每次执行当前事务 SELECT，实际 tag/body 经唯一 `CheckedFrame::checked` 构造；普通 read 在 :80–87 消费原帧返回原 NodeRead。`history.rs:786–798` 只在无 Budget、非 overlay 时透传当次帧，并逐次核请求 digest、读后核 deadline。没有新增保留表或跨 read/audit/事务/角色/cut/source/epoch 成功缓存。
- **反例：** 已核实作者原件所记录的同事务 body/tag UPDATE、DELETE、第二次 source Err、合法但属于另一 digest 的帧、读前/读后超期均拒绝或返回 Missing。真正 frame 计数器位于 `s_content.rs:258–274`；它计实际 F 调用，不是节点去重数。
- **推论检验：** `CheckedFrame` 只证 F，字段私有且消费转移所有权（`job.rs:370–430`）。原验证器继续核 Public/Template/JsonInner 与类型（`s_content.rs:1221–1237,2170–2196`）；源码与基线逐字相同，角色反例证据通过。三次 audit、业务/来源/目录判据及 Budget API 均未改。overlay 与 Budget Some 原尾段除排版及一个尾逗号外相同（`history.rs:800–828`）；实证仍为合法 SQLite F2/prepare1/accept1，SQLite 坏帧 prepare0，Memory 坏帧 prepare1。
- **错误兼容：** 读坏摘要仍为 `node_digest`，Missing/source/tag 边界保留。detail 改用既有 CheckedFrame 文案并增加实算摘要；完整树文字检索未发现产品按旧全文分支。未发现合同承诺 detail 逐字稳定，此变化不构成 Spec 违约。

没有发现概念矛盾，故无须谱系矛盾比对。禁止将本结论扩张成缓存授权或放宽准入。

## 证据与边界

逐份重算作者 12 个原件的 SHA/字节并核两产品文件与候选 blob 一致；复用 history 15、store 6、budget 6 项共 27 项绿测及非 test cargo check，不重复运行。testbin SHA `65afcdc838e628fd95fc1d38c401688f934fad6f5a67684ecd6bf416fcd78cbb` 与冻结正式 bin SHA `991dd18dd0239b4d234a1f607cf72b405b79ec2a75ee7a3350c33d7a7cfca5b4` 均匹配。

新原件为 `SPEC-probes-static.json`，包含完整 diff、版本/范围、逐文件完整性核对及未改判据核对。静态尾段比较首次因合法尾逗号失败，查明并在原件明记；不是产品失败。未新增运行时探针、未写 target、未启动服务、未跑 S16/9011、未改变资源配置。只有同次 F 次数证据，无耗时提速结论。工程轴由另一工位独立出结论。
