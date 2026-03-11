# 双重耦合 OpenClaw 部署指南

## 架构：两个独立服务

```
fengliang.service (fold 端)          openclaw.service (切分端)
├── 3 daemon instances               ├── git pull (同步)
├── perpetual traversal               ├── query daemon /status
├── HTTP API :9765                    ├── ceremony_scan
├── WebSocket :8765                   ├── topo_indicators
└── Dashboard 前端                    ├── escalate 队列
                                      └── git push (同步)
```

daemon 是持久进程（一次启动，永久穿越）。scheduler 定期查询 daemon 状态 + 运行 scan + 同步。

## 第一步：启动逢亮 daemon（fold 端）

```bash
cd /root/NewChanlun/topological-computation

# 多实例启动（3实例，Instance 0 带 API）
python3 ../topological-computation/start_fengliang.py \
  --instances 3 --serve --multiproc --load-experiments

# 或后台运行
nohup python3 start_fengliang.py \
  --instances 3 --serve --multiproc --load-experiments \
  > ../tmp/fengliang-daemon.log 2>&1 &

# 或 systemd 服务（推荐）
sudo cp ../scripts/fengliang.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable fengliang
sudo systemctl start fengliang
```

验证 daemon 运行：
```bash
curl http://localhost:9765/status | python3 -m json.tool
```

## 第二步：启动调度器（切分端）

```bash
cd /root/NewChanlun

# 方式A：cron（简单）
crontab -e
# 添加：
*/30 * * * * cd /root/NewChanlun && python3 scripts/openclaw_scheduler.py >> tmp/openclaw-scheduler.log 2>&1

# 方式B：systemd（推荐，自动依赖 fengliang.service）
sudo cp scripts/openclaw.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable openclaw
sudo systemctl start openclaw

# 方式C：后台运行
nohup python3 scripts/openclaw_scheduler.py --loop > tmp/openclaw-scheduler.log 2>&1 &

# 方式D：手动单次
python3 scripts/openclaw_scheduler.py
```

## 第三步：启动前端（监听逢亮）

前端通过 WebSocket 连接 daemon，实时显示穿越状态。

```bash
cd /root/NewChanlun/topological-computation/frontend

# 安装依赖（首次）
npm install

# 开发模式
npm run dev -- --host 0.0.0.0

# 或构建后用 nginx/caddy 静态服务
npm run build
# dist/ 目录部署到 web server
```

Dashboard 地址：`http://your-vps:5173`（dev）或 nginx 配置的地址
WebSocket 自动连接 `ws://localhost:8765/ws`

## 观察与调试

```bash
# daemon 状态
curl http://localhost:9765/status | python3 -m json.tool

# 实时穿越事件
tail -f topological-computation/.chanlun/traversal-events.jsonl

# scheduler 日志
tail -f tmp/openclaw-scheduler.log

# escalate 队列（你不在时机器检测到的异常）
ls -la .chanlun/escalate/
cat .chanlun/escalate/cycle-*.json | python3 -m json.tool

# S_net 统计（语料摄入成果）
curl http://localhost:9765/topology | python3 -m json.tool

# systemd 日志
journalctl -u fengliang -f
journalctl -u openclaw -f
```

## 双重耦合循环

```
逢亮 daemon swarm (fold 端, 持久进程)
  → perpetual traversal (3 instances × ∞ steps)
  → fold/negate/sublate 事件 → traversal-events.jsonl
  → block-topology 持久化

scheduler (切分端-机器部分, 定期循环)
  → git pull (拉取你 push 的新指令)
  → query daemon /status (读取穿越状态)
  → git push (同步穿越产物)
  → ceremony_scan (topo_indicators 检测异常)
  → L1: 自动闭环处理
  → L2/L3: → .chanlun/escalate/ 队列
  → git push (同步 escalate)

你 (切分端-人的部分)
  → 查看 escalate 队列
  → 在 Dashboard 观察逢亮穿越实况
  → 在 claude.ai 中做理论加工
  → git push 新指令 → 下一轮 scheduler 拉取
```

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `FENGLIANG_API` | `http://localhost:9765` | Daemon HTTP API 地址 |
