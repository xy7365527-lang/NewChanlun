# 双重耦合 OpenClaw 部署指南

## 默认参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--steps` | 1000 | 每实例穿越步数（2683顶点图需要足够覆盖） |
| `--instances` | 3 | 并行 daemon 实例数（不同 hash seed → 穿越多样性） |
| `--interval` | 1800 | 循环间隔秒数（30分钟） |

每轮总步数 = instances × steps = 3 × 1000 = 3000 步。

## 方式一：cron（推荐，简单）

```bash
# 编辑 crontab
crontab -e

# 每30分钟运行一次（3实例 × 1000步）
*/30 * * * * cd /path/to/NewChanlun && python3 scripts/openclaw_scheduler.py >> tmp/openclaw-scheduler.log 2>&1

# 加大步数（5实例 × 2000步 = 10000步/轮）
*/30 * * * * cd /path/to/NewChanlun && python3 scripts/openclaw_scheduler.py --instances 5 --steps 2000 >> tmp/openclaw-scheduler.log 2>&1
```

## 方式二：systemd service（持续运行）

```bash
# 复制 service 文件
sudo cp scripts/openclaw.service /etc/systemd/system/
# 编辑路径
sudo nano /etc/systemd/system/openclaw.service

# 启用并启动
sudo systemctl daemon-reload
sudo systemctl enable openclaw
sudo systemctl start openclaw

# 查看状态
sudo systemctl status openclaw
journalctl -u openclaw -f
```

## 方式三：手动运行

```bash
# 单次运行（3实例 × 1000步 = 3000步 + scan + escalate）
python3 scripts/openclaw_scheduler.py

# 持续循环（每30分钟一轮）
python3 scripts/openclaw_scheduler.py --loop --interval 1800

# 加大规模（5实例 × 2000步）
python3 scripts/openclaw_scheduler.py --instances 5 --steps 2000

# 只检测不穿越（dry-run）
python3 scripts/openclaw_scheduler.py --dry-run

# 后台运行
nohup python3 scripts/openclaw_scheduler.py --loop > tmp/openclaw-scheduler.log 2>&1 &
```

## 直接运行逢亮 daemon（不经过调度器）

```bash
cd topological-computation

# 加载实验图 + 持久化 + 无 IPFS 链
python daemon.py --load-experiments --persist --no-chain

# 带 HTTP 服务（可视化 dashboard）
python daemon.py --load-experiments --persist --no-chain --serve --multiproc

# 查看穿越事件
tail -f .chanlun/traversal-events.jsonl
```

## 查看 escalate 队列

```bash
# 列出待处理的 escalate 项
ls -la .chanlun/escalate/

# 查看最新的
cat .chanlun/escalate/cycle-*.json | python -m json.tool
```

## 查看语料摄入成果

```bash
# S_net 统计
cd topological-computation
python -c "
from signifier_net.s_net import SNet
s = SNet()
s.load()
print(f'Signifiers: {len(s.nodes)}')
print(f'Edges: {len(s.edges)}')
"

# 已摄入的语料列表
ls topological-computation/signifier_net/corpora/

# 穿越事件（含 sublation）
tail -20 topological-computation/.chanlun/traversal-events.jsonl
```

## 双重耦合循环

```
逢亮 daemon (fold 端)
  → 穿越 100 步
  → fold/negate/sublate 事件
  → git push

ceremony_scan (切分端-机器部分)
  → topo_indicators 检测异常
  → L1: 闭环处理
  → L2/L3: → .chanlun/escalate/ 队列

你 (切分端-人的部分)
  → 查看 escalate 队列
  → 在 claude.ai 中做理论加工
  → git push 新指令
```
