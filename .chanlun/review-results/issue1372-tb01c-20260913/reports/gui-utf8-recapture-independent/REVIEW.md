无损 GUI 导出与紧凑 Ready 增量独评：**有界通过；完整 C 仍 NOT_VERIFIED**。

审阅身份 `/root/r7_codex_product_review`，`codex_native_subagent`。仅用冻结文件和 Git 对象；没有启动产品、打开SQLite连接、访问活API或浏览器。原R4独评包32件全部SHA复核未改；原24处损坏、两个G96分页截断及首次Q Ready失败均保留。

六份 `*-ascii-envelope.json` 的外层字节和内层JSON文本均为ASCII；外层/内层各解析一次后，与另存decoded JSON完整相等，U+FFFD均为0。导出脚本在JSON.stringify之后转义非ASCII UTF16单元，没有修改业务值。五份完整候选直接与原495权威记录的独立投影逐字段、按原顺序对拍；没有导入Q投影器或替换字符。state、projection、cursor、counts和浏览器projection SHA均一致。

| 新候选 | cut / 模式 | 完整行 / 页 | 结论 |
|---|---|---|---|
| recomputed135 | 135 / RecomputedWithRevision | 1400 / 46 | PASS |
| as-known96 | 96 / AsKnown | 849 / 28 | PASS |
| recomputed156 | 156 / RecomputedWithRevision | 1689 / 55 | PASS |
| healthy-watch156-160 | 160 / RecomputedWithRevision | 1745 / 57 | PASS |
| final160 | 160 / RecomputedWithRevision | 1745 / 57 | PASS |

真实 Watch 原帧2,728,573字节，SHA256 `0a8654b88045cf1d7c1d7207d9b8dbd14853c740faa70c7ac74bc9dbce187866`；G157–160四个原批完整相等，G159含3撤回/3替代，原游标156与新游标160准确。迟到AsKnown96首31行的完整原帧128,960字节，SHA256 `26abf3f518bb427c8b3ff6afdc320157079ba1003ff178f8b213ba04990b7aca`；公共因果/摘要、固定cut、完整页值与token均一致。冻结浏览器脚本先持有真实旧页，等新160完整读取后释放，旧读取返回“已被较新读取取代”，最终cursor160；页内baseline及rendered完整串比较为真。该回执不再有旧导出的字符破坏。

这是单独Q读取R4-A静态备份的新补验，query_epoch=3；不是重新运行S持续输入。recomputed135是显式load，不能另称此次重跑了自动Gap。自动Gap原wire仍由原R4审阅支持；旧G96 lane截断也不能因此被改写。六个坏帧负控此次没有重复执行。

来源：新导出SOURCE记录head `68198a6cdabbdbfeee916a00321442999c3de673`；六个Q/UI/controller源Git对象与R4相同SHA。备份原/后声明SHA一致，本工位另读备份文件全部130,269,184字节核得 `a30aaa8d62b4658f9891ce89b082ca6d9030928be631a5c9fa6f013c321de74a`，未连接SQLite。

封存链有真实例外：原MANIFEST40件中38件仍精确匹配，`control/q.log`和`control/q.process.json`在封存后被紧凑Ready诊断复用修改，原40件全部静止的声明失效。本工位首次就因此退出1，差异保留在 MANIFEST-DIFFERENCES.json。根另存的原日志前33,976字节核得原SHA `11dda10998d5f71cbe77c6cbe77b68a7899adc014846c69ef2c3a1e92198e6a5`；没有捏造原q.process.json可恢复。38件语义/ASCII/脚本/元数据原件再次逐件核SHA后用于本结论，不能说40件未变。详见 POST-SEAL-CONTROL-MUTATION.json 与 EXAMINATION.json#/manifest_postseal_changes。

旧启动失败是真失败：完整 `/api/state` 1,144,017字节大于控制器1,048,576界限；START-Q stdout空，stderr `ok=false` 明确声明未在期限内就绪。之后GUI成功不能追认该次Ready。

另审固定 `14c15bd44eeff6c0ecc75f3768a7fb734f398230`：只新增 `/api/ready` 并切控制器请求路径。`s_readonly_server.py:897-920`仍先执行同一 `capture_verified()` 和完整 `project_state()`，之后只序列化cut/ok/producer_epoch/control_instance_id；坏库与资源失败仍走原503。`:1039-1044`拒查询参数。`s_service_control.py:72-105`保留1MiB正文、16KiB头和绝对剩余期限，后续Ready身份/nonce/PID门不变，未以轻量回复取代完整审计。

独立核根真实G160回执：新Ready448字节，cut/epoch4/nonce与完整state对应字段逐值相等，START-Q真正返回ready。根24控制及30查询回归日志均通过；新增HTTP用例同时验证非法参数400、坏raw导致state和ready同为503。本工位阅读该源码与原日志，没有重复运行服务/测试。根报告3742.95ms未在本包保存可独立重算的起止回执，故此处不把该数记为独测。紧凑Ready是控制回复大小修复，不是持续读取性能优化，不能称成本O(1)。

原R4报告不回写；新材料有界补齐完整GUI导出和held原帧缺证，紧凑Ready兼容增量亦有界通过。持续读取AC3的30秒窗口未由本审阅验收，其他现役修复/完整C和#1323未完成，不新增通过标签。精确原件指针、SHA、源码行号及执行边界见 REVIEW.json、READING-LOG.json、READY-REVIEW.json。
