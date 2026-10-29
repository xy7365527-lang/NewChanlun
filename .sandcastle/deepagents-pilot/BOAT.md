# 用 Boat 执行新 run

控制端继续持有 Wayfinder claim、受管提交、批准和 GitHub App 发布。新 run 从 `PILOT_BOAT_CONFIG` 冻结执行配置。缺配置时拒绝创建，不回退本机。历史 `run.json` 没有 `execution` 字段时仍按原位置恢复。

## 准备云镜像和模型入口

1. 在专用 Boat VM 按 `.sandcastle/Dockerfile` 构建现役基础镜像。
2. 用 `.sandcastle/Boat.Dockerfile` 扩展该镜像。该层只加固定版本的 Bun。
3. 用 `docker image inspect --format '{{.Id}}'` 取得精确镜像身份。
4. 在受管配置文件保存以下非秘密字段。不要填写 key 值。

```json
{
  "endpoint": "https://YOUR-MODEL-ENDPOINT/v1",
  "image": "sha256:FULL_IMAGE_ID",
  "image_source_sandbox": "bx_IMAGE_SOURCE",
  "key_source_sandbox": "bx_MODEL_GATEWAY",
  "key_source_path": "/home/user/model-gateway1804/client-key",
  "pstack_source": "/Users/silencehan/.codex/pstack-upstream-source",
  "model_rules": "/Users/silencehan/.agents/pstack-models.md",
  "ttl": 1800,
  "max_standard_seconds": 500000
}
```

`endpoint` 必须是 HTTPS `/v1` 入口。`image` 必须是完整 `sha256:` 身份。镜像与模型能力经受管 SSH 管道传输，不进入运行包、提示或日志。来源 VM 需由其 owner 显式恢复。模型客户端 key 在工蜂环境中的可见性沿用 #1804 已批风险。

## 启动、观察和恢复

设置 `PILOT_BOAT_CONFIG` 为上述文件的绝对路径，再使用原 `python -m pilot1742.executor` 或 `python -m pilot1742 run` 入口。调用方仍调用 `run_worker(role, brief)`。发布与批准入口不变。

每个 run 的 `boat/sandbox.json` 保存唯一 VM；每个工蜂的 `boat/<key>.json` 保存 generation、head、package 和原 command。`submitting` 之后再次调用只对账，不二次派工。远端排他 `claim/` 目录是执行事实，VM 状态不能代替它。

停止后的 VM 不会因状态查询自动续费。显式恢复同一 VM 后，再恢复原 run：

```sh
boat resume bx_ORIGINAL_ID --no-env --ttl 1800
python -m pilot1742 status --home /absolute/run-home --run-id ORIGINAL_RUN
```

`EffectUnknown` 保留原身份和日志。不要换 run ID 规避未知副作用。退出码缺失、输出截断、timeout、旧 generation、模型不符和 head 漂移都不能成为成功。

控制进程退出并释放 run 锁后，可取消原绑定：

```sh
python -m pilot1742 cancel-worker --home /absolute/run-home \
  --run-id ORIGINAL_RUN --role author --head FULL_TARGET_SHA
```

取消请求只作用于该 binding 的远端进程。后续观察到终态才算退出。取回回执和 session 后，由资源 owner 停止原 VM 并核 `stop.status=completed`。

## 运行包和验收

源码导出保留原 Git SHA，只带当前及父提交所需对象，不带宿主 Git 配置。`boat push` 会省略 `.git`，因此 adapter 使用显式 tar 与 `boat scp`。pstack、项目技能和规则作为只读挂载进入 Docker，依赖由锁文件在 Linux 重建，不复制 Darwin 缓存。

完整接入验收仍需真实模型轨迹、异家族独审、原发布链、断线恢复和精确 head 批准。离线恢复测试与 Docker 探针不能替代这些结果。首轮资源预算上限为 5 美元，每次 VM 上限为 1800 秒。
