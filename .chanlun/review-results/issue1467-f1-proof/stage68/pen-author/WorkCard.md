# Stage68 DevelopingPen 工作卡 v1

研究票1467；作者A；2026-10-06。唯一写入 stage68/pen-author。冻结后只运行一个主状态机 DP68-v1，443 个既有前缀，96 MiB；零新历史、零 R_W 重跑、零参数搜索。本卡不把产物称为独评通过。

候选身份是 `(anchor.raw, anchor.known_at, direction)` 加不可变版本 `(observedAt,end,price)`。每次当前端点变化，撤销旧的端点提案并发布新版；候选族不因此被当作已经成笔。方向由已知底向上、已知顶向下。合法当前提案还必须具有正事件时长、相应方向位移、从锚以来的当前极值；间隔未满足时明确为 waiting-spacing。足够间隔只给 pending-opposite-fractal，缺少终点右侧见证仍不授原义笔。

选用同一历史 point-bar 条件解释 `H_i=L_i=p_i`，直接核已出现的严格三点顶底、邻接、无包含、端点相隔至少4（因此原文旧笔要求满足），而不借 active 字段授原义资格。订单观察可否承担原文 K 线仍是未决桥。新笔续排来源核归属，062正文与082正文直接作基本证据；不用027:854/856。

已知反向锚到达且完整端点/价格/方向与此前某一候选版本相同，才能生成 `confirmed-rw-active` 的具名关系；该证书是当时解析器 active 的事实，不承诺不可修改。若 active 身份随后被改，撤销该 tentative 证书而非改内容。相同元组进入 stable 时发布不可变 `stable-rw` 证书。confirmed/stable 几何对象与候选共享端点几何但属于不同证据状态；stable 若被重写立即失败。新锚另起新候选，不能让上一支笔吞当前残段。

初始化为 no-known-anchor；首笔待定为 first-pen-not-stable；零时长为 zero-duration，均是具名未定值而非0。每次锚刚到达同时记录从锚到锚的零时长初始状态，随后接入当前已到达观测。仅有正时长且同价时才允许公式给0。

力度用上述候选的当前速度减同一片已稳定首笔速度，名为 `L_proposed`；waiting-spacing 与 pending-opposite-fractal 保留在数值旁，原义 L/StructuralPenNow 均为null。原始5笔块及5块整片用预声明模板；五笔端点进入active时记 ended-awaiting-stability，进入stable时冻结该片的首末速度差；这不是067封口或原义Completed。已有Stage67当前行首次显示证齐时才将同一固定端点整片映射为 selected-raw-evidence-complete。新残段分到独立后继槽位（仅模板Own/Next），不改旧冻结L。

118等局部弱化事件按端点版本逐项保存；延伸则撤销旧端点提案，转假则记录取消，不锁存Completed。121的端点只在相应版后来成active/stable时得到确认，不能从118的true保持到旧end121冒作同一次完成。

输入：Stage66 source.json；已审 independent-prefixes.json；Stage67 independent-prefix-results.jsonl。驱动器允许整体预载source观察/事件，前缀和Stage67当前行逐项解码（最多64KiB字节预读）；step只收当前prefix及五条当前行。完整输入哈希预检读取字节但不解析最终pens/objects。所有最终pens/objects/parent及最终证书表仅在线seal后解析。作者研究此前已看过选定轨迹，不能因此声称盲测；程序依赖与作者知识分开。

验收：全443关系；关键117/118/121/122/126、217/221/222、417/421/422；不回写stable；端点确认与当前新残段分离；冻结L与旧证书对拍；首笔/零时长初始化实际覆盖；普通加权平均引理及121精确回代。失败保留，不增加对照除非具体失败确需。交付脚本、来源证据、输入前后哈希、机器状态、Report、manifest、命令exit/RSS、FINAL；冻结供独立审查。
