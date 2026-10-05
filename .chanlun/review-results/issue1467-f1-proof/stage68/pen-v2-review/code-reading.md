# DP68-v2 变更源码及原义边界核对

范围是外置作者文件 `../pen-author-v2/check_pen.py` 对 v1 的真实 diff。完整文件已读，新增流程聚焦如下。

| 位置 | 读取结论 | 动态支持及限制 |
|---|---|---|
| `:74–76`，`:197–200` | stable 前缀逐字比较先于 prices/times 追加；拒绝证据深复制旧、新 prefix | 控制10通过真正 step 入口，所有非事件状态不变 |
| `:77–92` | 依次通过 terminalCandidate 或 pendingTerminalPen 找末笔；clear_span 拒绝 stable，保留 previous/current 深复制快照，清端点及依赖字段 | 控制1/3核实际存在端点和 pending 引用；无假冻结缓存注入 |
| `:93–106` | withdraw_active 先检查笔 stable 和全部 affected 对象，再改 live_confirmed/unrepresented/previous_active，随后传播到 raw/whole | 控制1/3/9；原字典和事件历史保留 |
| `:107–123` | 仅查询既有 lookup 版本；缺少合格版本记 unrepresented，不调用 version；重接纳重新检查条件点几何，保留首次 confirmed | 控制2/4，全版本、lookup、confirmed 逐项不变 |
| `:124–130` | stable 回执深复制；无当前有效候选时 candidateVersion=null，不冒造候选 | 控制5覆盖已有表示；stable-rw-unrepresented 未覆盖且作者明示未证 |
| `:131–149` | bind_terminal 每步比当前末笔完整元组；变更先取消暂定对象；表示不足保留 pending，表示成立只绑定现存版本 | 控制1–4；驱动提供稳定前缀的前提仍须保留 |
| `:150–164` | 任何冻结写入之前，核版本五元组、end、stable存在、stable.candidateVersion；正确时才计算原始/whole/c 力度 | 控制6核首次写入前拒绝，7核独立分数，8核已冻结对象拒绝 |
| `:197–255` | step 先接新 stable，再处理旧 active 撤销；raw/whole 每次重新 bind。原始证齐须已有表示和冻结 cL | 原443前缀逐字一致；t138完整字段期望通过。真实active替换仍0 |
| `:270–295` | 弱化候选和力度保持原路径；原义字段仍为None；历史raw slot名对应len(anchors)-1 | 11核心产物三方同；沿用v1已审槽语义，须/5才是段槽 |
| `:372–447` | main 仍在online seal后才解析四个最终JSON；source整体预载，prefix字符缓冲，v1核心只做seal后散列 | 本轮未重做v1读取时序插桩；源码复核无边界扩大。独审自建驱动未调用作者main/state_control |

完整状态拒绝比较排除的唯一字段为 events，且另核仅追加一条拒绝事件、既有事件前缀字典逐项不变。正常撤销的取消事件含真实完整 previous/current 对象，未只核计数或字符串。

## 独立期望的来源

- raw5 起点从已审 prefix 的 anchors[25] 取值，首次可建立时钟独立扫描为106；P0 起点从 anchors[5] 取值，首次可建立时钟为22。末笔117→121首次 active 为122，起点锚 known_at=118，绑定版本为 DP:117@118:v121。
- 对象常驻字段来自工作合同，旧历史字段精确保留；取消与pending态用完整expected对象相等核验。清字段列表不直接复制作者循环作oracle。
- stable 首见时钟独立扫描输入 prefix 为126，不读取作者stable证书作时钟oracle。回执候选版本独立由起点锚与既有终点构造。
- L 直接从源 observations 的价格、时间构成 Fraction。raw5/c 为末笔速度减第26笔速度，whole为末笔速度减第6笔速度，分别为−500和−1500。没有调用作者velocity/pack计算expected。
- 原始证齐使用已审Stage67当前cut输入，t138副本字段与独立expected完整一致；原始窄证本身继承v1审查，未重跑或扩大其证明资格。

## 原义边界

v2 Report:23、:61、:69、:71 保留候选状态、条件几何与原义资格区分。机器字段亦保留null，源码未引入自动授原义资格的路径。

直接回查正本 `.chanlun/definitions/beichi.md:317` 的进行段定义及 `:563–569` 的每刻可算、未完成笔取值留白，支持不得把完成笔端点条件偷换成所有进行笔的即时合法性要求。等待和null只是本次资格未交，不构成原义反例。

直接回查正本 `docs/chanlun/text/blog/084-第84课.md:52`、`:54`、`:56`。12行署作者，68行为正文界；所用三行在界内，无行首/行内编者注。52允许别的递归起点并要求唯一分解，54–56区分f1/f2。故本版source_pointbar_bridge仅是DP68继承原文笔定义的条件，不能升级为所有替代F1都要还原原文K线的通用要求。

没有回头修改v1的原文归属审查；其他已审来源仍依v1冻结结论和本版字节相同的source-evidence使用。
