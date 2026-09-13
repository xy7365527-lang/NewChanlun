Q 编码复用候选已完成，状态 READY_FOR_INDEPENDENT_REVIEW；未裁定 C 或真实 30s Watch 验收。

原热点在每次变化版本的 capture_digest：对全部旧 Delta SQL 文本重复 JSON 字符串转义。G160 同镜像强制清 cached、保留 memo 的原开销 0.438–0.441s；cProfile 显示摘要阶段占核验 0.240/0.444s。113 页纯 Python 暖投影不足 1s，优先优化审计编码而非减少任何页面或比较。

最小改动仅 s_query_integrity.py 和专属测试：逐代证书新增压缩的 canonical(delta_json 字符串) 编码原字节。当前全 typed 捕获、schema、所有 raw/pending、独立索引、控制/epoch、旧 Delta 每列及原 UTF8 和 batch 全字节比较保持；源不同仍按既有策略重建后缀。全部健康门完成后才使用本次已证明源相同的编码。capture_digest 的类型标签、长度、内容、次序与最终值不变。use_memo=False 仍走原算法。

| G160 条件 | 原版本 | 候选 |
|---|---:|---:|
| cold 完整捕获+核验+计量 | 3.637s | 3.664s |
| 无变化缓存 | 0.055ms | 0.047ms |
| 同镜像强制 fresh，三次 | 0.438–0.441s | 0.326–0.329s |
| fresh capture / 读事务 | 51.9–53.0ms | 51.4–54.4ms |
| fresh verify | 350–352ms | 238–239ms |
| object_bytes | 35.3–35.7ms | 34.5–35.1ms |
| cut96..99，完整113页纯Python暖读 | 0.973s | 0.944s |

强制 fresh 不是实际新提交。另在自建副本做实际独立 SQLite Commit（meta 附注），观察 data_version 2→3、结构代仍160，候选耗时0.324s、160个memo命中，新的 capture_digest 与不带编码证书的原算法相同。随后改坏旧159代Delta为重复JSON键，立即拒绝，memo/cache清空、读事务释放。没有打开原取证库进行SQL，也未触碰其sidecar；原数据库完整SHA复核未变。

预算先量化后实施：proof+memo 实测32,090,730→40,771,243 B，新增8,680,513 B，全由现有对象计量纳入；每worker限额134,217,728 B，总缓存及捕获限制都保持268,435,456 B。捕获对象188,952,195 B。超额时原逻辑丢memo并保留完整历史/全核，低预算回归通过。离线脚本含profile/分页的峰值RSS为358,219,776→433,340,416 B；这是真实峰值观察，不能把retained预算说成RSS限制。

验证：31项Q回归通过，含根新增Ready测试、低预算丢memo、外部坏raw/索引/schema/旧源、重复JSON键以及新增空白/Unicode/引号/控制转义对拍。真实v2整合5次ingest、21行历史token跨Commit/GC/S停止恢复、8坏控制均通过。冻结同输入816个完整公开值逐字段/规范字节相同：G0..160两种模式完整投影、legacy state/cursor、160完整Delta、4个legacy Delta区间、7个Watch/Gap；未排除字段。全部源SHA、分项原数据、profile、旧诊断和新结果分目录保留。

生产源未提交；s_query.py 无须修改。未改readonly_server/controller/browser/fixture。候选需要独立工位复核，然后由根在同候选上重新执行正式160与真实HTTP验收；上述局部证据不宣告AC3或C完成。

缓存索引的前置条件已由现有 verify 门证明：capture 在已核固定 schema 下按主键取 structure_deltas（L232–240）；_typed_rows 验证字段精确类型，随后 len(deltas)==G（L715–717），遍历又要求每行 generation 等于从1开始的枚举值（L800–804）。因此完整已核链恰为1..G。每代证书只在当前全 typed 非文本列、Delta 原 UTF8 字节、可达 batch 当前全字节相同时复用（L813–821）；任一差异使本代及后缀重新核验。next_entries 按同一枚举每代追加一次（L836–841），完整独立索引及控制门均通过后才计算摘要（L901–902）。故摘要中的 entries[generation-1] 指向本次已核的同一代，不依赖未经核验的外部 generation 或旧摘要；G0 为零行/零证书。
