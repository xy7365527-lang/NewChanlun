# 用 Boat 执行新 run

控制端继续持有 Wayfinder claim、受管提交、批准和 GitHub App 发布。新 run 从 `PILOT_BOAT_CONFIG` 冻结执行配置。缺配置时拒绝创建，不回退本机。历史 `run.json` 没有 `execution` 字段时仍按原位置恢复。

每个工蜂键（run、角色、目标 head、持有代次）独占一台 Boat VM。作者和独审各用自己的 30 分钟窗口，互不分摊寿命。

## 准备冻结镜像的基底 VM

每台工蜂 VM 都从一台已停止的基底 VM fork 出来。fork 复制基底的最新快照，Boat 在恢复磁盘时一并恢复已保存的 Docker 镜像；控制端等 `hydrated` 为真后才派工，监督器启动工蜂前核对镜像精确 ID，缺失即写失败回执。

1. 在一台 `--no-env` 的专用 VM 上按 `.sandcastle/Dockerfile` 构建现役基础镜像，再用 `.sandcastle/Boat.Dockerfile` 加固定版本 Bun。
2. 取得精确身份：`docker image inspect --format '{{.Id}}' newchanlun-boat1804`，填入配置的 `image`。
3. 停止该 VM 并核 `stop.status=completed`。adapter 只 fork 它，从不启动或停止它。

停止的快照保留 14 天，过期后 fork 失败。基底 owner 在快照到期或丢失时，按原运行包 manifest 的源码 head、仓内锁文件及上述 Dockerfile 重建一台新的专用基底。核对新镜像身份并停止新基底，然后为新 run 更新配置的 `base_sandbox` 与 `image`。已有 run 保留冻结配置，不能直接改写其绑定。不要用常驻 VM、心跳或自动续期维持基底。付费计划可选 saved snapshot，免费试用不提供。

## 受管配置

配置文件只放非秘密字段，不填写 key 值。

```json
{
  "endpoint": "https://YOUR-MODEL-ENDPOINT/v1",
  "image": "sha256:FULL_IMAGE_ID",
  "base_sandbox": "bx_STOPPED_BASE_VM",
  "key_source_sandbox": "bx_MODEL_GATEWAY",
  "key_source_path": "/home/user/model-gateway1804/client-key",
  "pstack_source": "/Users/silencehan/.codex/pstack-upstream-source",
  "model_rules": "/Users/silencehan/.agents/pstack-models.md",
  "ttl": 1800,
  "max_standard_seconds": 500000
}
```

`endpoint` 必须是 HTTPS `/v1` 入口。`image` 必须是完整 `sha256:` 身份，且基底 VM 内有该镜像。模型能力经受管 SSH 管道从网关 VM 流到工蜂 VM，不进入运行包、提示或日志；网关 VM 由其 owner 显式管理，adapter 不启动它。工蜂读取 key 后立即删除文件，监督器结束时再删一次，收集后控制端在停 VM 前确认删除。

## 一次派工的生命周期

`boat/<key>.json` 是该工蜂的唯一绑定，依次经过 `creating → prepared → submitting → submitted → collected`。

- `creating`：先落 `create_requested`，再 fork 基底并把新 VM 改名为该键的唯一名称。重启后按名称收养；fork 响应丢失且没有同名 VM 时报未知，不再 fork 第二台。随后上传运行包；远端已有认领时拒绝覆盖。
- `prepared`：派工前读 `archiveAfter`。剩余寿命不足 15 分钟加 2 分钟收尾余量时拒绝派工，不自动续期。监督器拿到绝对截止时间（自动停止前 2 分钟），镜像核对、依赖安装和工蜂都受它约束，到点写失败回执。
- `submitting`：提交响应丢失时，先看远端认领；没有认领再查 `boat history`。没有任何提交记录才补提交一次，远端排他认领保证模型任务最多一次。
- `submitted` 之后，只有取到远端终态回执才收集。回执和 session 取回本地后，控制端删除 key 并停止该 VM，记录 stop ID。

停止的 VM 只会让观察结果成为 `EffectUnknown`，提示显式恢复，不会自动续费或重派。恢复后先按原认领和监督进程对账：进程仍在就继续等，已结束且无回执就保持未知。

## 启动、观察和恢复

设置 `PILOT_BOAT_CONFIG` 为上述文件的绝对路径，再使用原 `python -m pilot1742.executor` 或 `python -m pilot1742 run` 入口。调用方仍调用 `run_worker(role, brief)`。发布与批准入口不变。

只读状态只读本地记录，不访问 Boat、不收集、不进入票级锁：

```sh
python -m pilot1742.executor status --run-id ORIGINAL_RUN
```

已收集的工蜂即使 VM 已停止，状态和后续派工也直接使用本地回执。需要对账在途工蜂时，先显式恢复它的 VM，再由原控制入口恢复 run：

```sh
boat resume bx_WORKER_VM --ttl 1800
python -m pilot1742.executor run --run-id ORIGINAL_RUN --private-key /absolute/app.pem --resume
```

`EffectUnknown` 保留原身份和日志。不要换 run ID 规避未知副作用。退出码缺失、输出截断、timeout、旧 generation、模型不符和 head 漂移都不能成为成功。

## 取消

控制进程退出并释放 run 锁后，v2 run 经执行器在票级动作锁下取消：

```sh
python -m pilot1742.executor cancel-worker --run-id ORIGINAL_RUN \
  --role author --head FULL_TARGET_SHA --private-key /absolute/app.pem
```

v1 run 用 `python -m pilot1742 cancel-worker --home ... --run-id ... --role ... --head ...`。取消只作用于该绑定的远端监督进程。监督器认领前到达的取消直接占住认领并写下“模型任务未启动”的回执；启动下一阶段与取消共用一把锁，取消后不会再启动新阶段。打印的是取消后观察到的事实，看到终态才算退出。

## 运行包和验收

源码导出保留原 Git SHA，只带当前及父提交所需对象，不带宿主 Git 配置。`boat push` 会省略 `.git`，因此 adapter 使用显式 tar 与 `boat scp`。pstack 与模型规则在每个 run 首次派工时冻结为一份技能包，包身份含其内容摘要，同一 run 的作者与独审使用同一份。主控在首次构建时也读取这份技能包和规则。主控项目技能与纪律从 run 的冻结 base 导出，不从实时源仓库或可变作者 checkout 读取。项目技能经同一个转换器生成内容身份，记录在主控 `boat/controller-context/context.json` 和工蜂 `package.json` 中。三者以只读挂载进入 Docker，依赖由锁文件在 Linux 重建，不复制 Darwin 缓存。

`tests/boat_live.py` 用无模型夹具工蜂在真实 Boat 上跑提交、监督、观察、收集、断连、停止恢复与取消。夹具回执带 `fixture: true`，不能替代真实模型验收。完整接入验收仍需真实模型轨迹、异家族独审、原发布链和精确 head 批准。首轮资源预算上限为 5 美元，每台 VM 上限为 1800 秒。

## 隔离测试与生产新建

`launcher` 只替换历史本机 run 的进程命令，不能为新 run 选择本机后端。测试套件在 `tests/conftest.py` 中替换新建工厂，构造本机夹具。生产工厂只产生 `BoatRun`。读取旧清单时，缺失 `execution` 或历史 `test-launcher` 记录都转成 `HistoricalLocal`，原清单保持不变。未知 backend 和无效 Boat 配置会拒绝加载。
