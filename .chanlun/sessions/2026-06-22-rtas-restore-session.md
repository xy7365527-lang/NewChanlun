# Session — RTAS 完全恢复就绪点

**时间**: 2026-06-22 (session 4130a713 退化，本文件为冷启动热启动锚)
**分支**: main（大量未提交：hook修复 + 引擎 M + 548谱系 + escalation settled）
**最新提交**: de8c6e7915

## 定义基底
13 已结算 + 2 生成态(chanlun-trading-system, fengkong)。0 pending / 516 settled(新增548)。

## 冷启动后第一动作（RTAS 完全恢复）
修好的 SessionStart hook 会自动注入 ceremony(additionalContext) + team bootstrap → 自驱蜂群循环。验证：恢复后第一动作应是 ceremony 差异检查，非直接 resume。

## 中断点（三线并行 + 串行链）
**编排者优先级链**: ①回测环境数据✅ ②死锁 ③滤波器=吃各级别|涨跌幅| ④hooks✅根治

- **核心判据(③)**: nt-full-1s-l2 catalog-streaming a0=segment 全周期 P5 跑中(PID 85740, 266.8M bar, 6.2GB)。等 by-year P5(踏空逐年量化)。
- **信号层有效域精确化(L2判决)**: a0=笔让「级别=带通滤波器」成立+可部署(2017现货证: 中间带-18383→+4596, 盈亏平衡-10.9→+12.97bps)。CME负结果=contango artifact。**但踏空(牛市暴露不足)=正交残余=操作层问题**，信号层解不了。
- **死锁重裁(②)**: 选项1经Codex xhigh对审错配——只解FaceA(方向锚)，解不了FaceB(僵尸核心/enter冻结/牛市重新入场=任务目标)。编排者重裁: **Face B 从修recover欠触发攻**(核心卡塔顶只sink不recover→衰减成1e-6僵尸)。task#12 deadlock-cascade-fix实装(根因解,不放宽enter/不调EPS), bit-exact+Codex对审+谱系。
- **串行链**: #12 recover(deadlock-cascade-fix) → #13 iterate增量O(笔²)→O(笔)(l2-verify-script, 设计done见docs/incremental_iterate_design.md, blocked by #12)。O(笔²)真根因=stream.rs:225 iterate(a0)全塔重跑。
- **hooks④根治**: 通道bug(systemMessage不进context)修复 ceremony+bootstrap两SessionStart hook(additionalContext+137正面格式)。548谱系结晶(097两类hook: tool-拦截严守阻断/放行 + bootstrap结构性例外)。运行时审计: 全hook loaded真触发,无静默(被静默表象=设计性静默通过误读)。
- **谱系免疫**: genealogy-maintenance 递归 spawn 子蜂群处理 #9下游推论/#10张力边/#11 async_self_ref。#7/#8已完成。

## RTAS 完全恢复清单(冷启动自动跑)
1. SessionStart自动ceremony✅(已修) 2. 自动team bootstrap✅(已修) 3. 蜂群循环自驱 4. teammate默认递归(原则15) 5. 结构skill事件驱动(dispatch-dag)

## 待 commit(用户决定)
hook修复(ceremony/bootstrap/4guard+double-helix debug开关) + 548谱系 + escalation(settled) + dag.yaml(548注册) + .gitignore(.hook-trace*)

## 恢复指引
1. 读本文件 2. 修好的hook自动ceremony 3. TaskList恢复(#5/#12/#13 + #9-11) 4. 后台PID 85740(segment P5)+catalog已落 5. 自驱蜂群循环,不手动微管理
