# #1394 最小 07 直接 inner header 补修：独立 Spec 审查

结论：**Spec PASS**。发现 0 项缺失、越界或错误实现。仅适用于下述固定四文件补修；不代表 Standards／工程通过、真实消费者接入或整图验收。

审查者为独立 Spec 工位，非作者；产品只读。基线 `b1a8d8d45c5dacd1245e2f33f20f7d514ae2446e`，候选 `08241aa12fa429245ba5132e6b6c2480f40029e4`；merge-base 等于基线，只有一个候选提交。实时 HEAD 匹配、tracked/staged diff 为空；未跟踪项仅 scratch。完整 diff 四文件为 `rust/src/s_content/path.rs`、`rust/tests/s_content_fragment.rs`、`s_session/browser/s-content-v3.js`、`s_session/tests/test_s_content_v3.cjs`。

## 概念层与 Spec

定义回溯以指定冻结批准源 §5.2、§5.6、§5.6.1 及本次明确的直接 header 补修范围为准。本 diff 不修改缠论领域定义或判定，不借无关教义文档替代此协议正本。批准源第 278 行的 value items 语义保留：直接 inner 是证明节点，不变成新的 items，也不意味着内部全文闭包已验证。

| 条款与边界 | 核实结果及代码位置 |
|---|---|
| §5.2:151、159、165：07 指向真实普通 JSON 值头，F、角色、extent 必须核实 | Rust `path.rs:207–212` 经 `read_value_node(..., true)` 与 `header_extent`；该读取核 F，拒 07／08／internal／坏 shape。JS `s-content-v3.js:1576` 复用已核图的 resolve，`GraphView.create:745` 核 F、`resolve:929` 核 JsonInner。 |
| §5.6:269–282：唯一 items、独立证明、缓存不能补证明 | `fragment.rs:239、362–413` 使生产与独立 verifier 重走同一检查；`Recorder:477` 原排序、去重、计费链不变。JS `:1531` 只由响应 nodes 建图，`:1576` 在 cache 写入 `:1597` 之前检查；offset=1 仍检查同 target。 |
| §5.6:278、286：读取内部内容仍须显式路径；范围不等于完整语义 | 新读取止于直接头，没有读取其序列或正文。独立最大 extent 探针无需其子图即可通过；`semanticValidated=false` 保留；完整验证因缺节点或非法 JsonInner 后代拒绝。 |
| §5.6.1:300–302：真实不同节点照原规则计费 | 作者 Rust 新测试核 producer/verifier 的 2 节点与分帧字节，节点／字节／reply 边界不放宽；JS 指标含新增头。没有默认资源值或新存储政策。 |
| 原有 07 完整路径、ordinary/JsonInner、异步不可变要求 | 全部产品 diff 仅 7 行 Rust 与 3 行 JS 新增；原检查无删除或弱化。复用既有原核心绿输出，另独立验证 object→array→07 路径及原完整 JsonInner 路径继续拒嵌套 07。 |

反例构造没有发现概念矛盾；新增直接头检查与已有完整路径／闭包校验层次一致。没有发现需进入谱系比对的矛盾。本工位未开展工程层审查。

## 证据与验证边界

- 批准源真实路径：`/Users/silencehan/Projects/NewChanlun-1394-content-v3/.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`；SHA256 `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`。
- 作者 `RESULT.json` SHA256 `8da3ee66054d224581be8b4a414b8649f368902aadcb3cfb0b3d511a9d44f0b9`；完整实时 diff 与 `candidate.patch` SHA256 均为 `a267cad6a53f17ae6d70254a003325f23ceb63e261ea3d427f35abacaad4a31d`。
- 26 项 live 完整性检查通过：四源码及其基线、批准源、RESULT、全部作者证据、live patch、HEAD、tracked/staged clean。详见 `SPEC-probes-integrity.json`。
- **复用**同字节 `GREEN-rust-final.txt` 的五套 60/60，以及 `GREEN-js-final.txt` 的 56/56（原核心 23 + 新 3 + Job 30）；已读 API、RED-GREEN、完整差异和相关调用上下文。红证据显示原基线 Rust 0/4、JS 1/3；setup 编译／anchor 错误不计作有效红测。本次没有重跑这些套件。
- **新增独立实跑** `node --test .scratch/issue1394-q3-jsontext-header-20261001/SPEC-probes.cjs`：3/3，通过。独立手工 F 分帧构图，覆盖四种最大 extent × 两 offset／超域拒绝；深一层路径的 false header 与空终点缺头（cache 和 decoy 均不能遮盖且不污染 cache）；直接头成功但原 JsonInner 完整路径／完整验证拒非法后代。
- `git diff --check <base> <candidate>` 通过。未改产品、未开服务／Browser、未跑经济／9011／全 cargo、未声称新资源或真实消费者验收。

报告、探针原件、实跑输出、完整性检查及源码摘要统一索引见 `SPEC-REVIEW.sha256` 与 `SPEC-REVIEW.json`。
