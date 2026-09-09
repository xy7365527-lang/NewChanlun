# #1370 最终验收包独立静态复核

**PASS：0H / 0M，1 项非阻塞文字澄清。** 本包可进入 PR 审阅；本结论不授予 main 合入、不关闭 #1370/TB-01/#1323。

受核 HEAD `7e10c9f4d0d81a6ebbda30a7c5ffbf0893081af5`，生产固定源 `76a641606d7097292fff2dc21f58c6156ae38916`；ROOT-ACCEPTANCE SHA-256 `38c97f033d9aa783d3dc7c1ef5bb3aef6270e86e395251801b37ce9dc755cb8e`，ACCEPTANCE-INDEX `1201aa8f721f3fa9cdec3fabaee82281693987d136489a46e727e50224145fec`。未运行测试或操作运行态。

48 个 payload 共 **2,743,799 字节**，均与原件和索引哈希相同；17 个列明生产文件与 76a 逐字相同；7 个 view 只增加横幅及转换链接，未改正文/结论。28 个本地阅读链接和对应 ref 锚可解析。已审归档整合还带入 agent roster 与 devskim baseline 的变动，未被误记为本片生产源差异；ROOT 对远端 CI 仍保留实际检查门。

**domain_and_9_ac：** ROOT 保持具名 TestOnly 一对一退化 OHLC/相邻严格无包含/无同价竞争的 CC006 域；不把 mixed 包含样本、其他115目录、经济或交易扩成通过。

**l3_l4_disposition：** L3 明确订正 seq=source_coord 不是合同前提；bigint payload 实际保留 seq 0/1/2 与源坐标大于2^53。L4 raw/merged 坐标、警告未标raw、tested_domain非逐批验证及TB02残留均明写。

**catalog_initial_vs_advanced：** R6真实init的0 implemented/116未实现，与root四分支cut9和bigint cut4的1 implemented/115未实现为不同阶段；对应源JSON一致。推进态仅CC006 implemented/not_proved/run。

**durability_reuse：** e79报告7场景46主断言39native命令、双新连接与真实writer trace均保留；目录修复前完整写路径和8旧测试逐字未变，最终9测试另有正式增量报告与构建记录。

**gui_vs_container：** macOS/真实GUI由root单独负责；容器报告保留未声称GUI/fullfsync的原话。四分支200/200、bigint32/32检查均真；故障AX为503后清旧数据，恢复HTTP与健康HTTP相等。

**open_scope_and_main：** 明确#1370仍open，main未获合入批准，全图与B/C等后续未完成；正常重启和坏scope恢复不等于进程真杀恢复。远端CI/fixture漂移不能由本包代证。

非阻塞 PACK-L01：“未归档的大型原始探针输出由外部定位明确列出”容易被读成完整外部清单；UNBUNDLED-REFERENCES 当前只列阅读版 Markdown 改链产生的 10 个目标。 例如耐久 REVIEW-e79.json 的 source_and_evidence_records 仍引用未入包的 SOURCE-BINDING-CHECK-e79.json；其原始路径/哈希在 payload JSON 内，而非这 10 条中。主结论和来源字节仍可核，不构成验收阻塞。

最小文档修法：把末句改成：阅读版 Markdown 链接的未归档目标见外部定位；其他未入包明细路径及哈希保留在各 payload JSON 的来源记录中。无需改 payload 原件。

未改验收包或 payload 原件。逐项哈希、文件与检查范围见 [JSON](/tmp/newchanlun-1323-publication-20260909/inputs/TB01-A-ACCEPTANCE-PACK-REVIEW.json)。本次仅核已有材料，不冒称重新目击 GUI 或重跑 native/容器检查。
