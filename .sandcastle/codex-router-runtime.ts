// #1448：仅为直接 Codex 工蜂读取本机 Router 的两个配置值。
import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join } from "node:path";

export const MODEL = "deepseek/deepseek-v4.1-flash";
export const EFFORT = "max";

export function readRouterConfiguration(env: NodeJS.ProcessEnv): { baseUrl: string; catalog: string } {
  const config = join(env.CODEX_HOME ?? join(env.HOME ?? homedir(), ".codex"), "config.toml");
  let values: unknown;
  try {
    // -I 隔离 Python 路径；只取两个值，不复制或加载其他 Codex 设置、认证或插件。
    const output = execFileSync("python3", ["-I", "-c", [
      "import json, sys, tomllib",
      "with open(sys.argv[1], 'rb') as f: c = tomllib.load(f)",
      "print(json.dumps([c.get('openai_base_url'), c.get('model_catalog_json')]))",
    ].join("\n"), config], {
      env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], timeout: 5000, maxBuffer: 65536,
    });
    values = JSON.parse(output);
  } catch {
    throw new Error("无法读取 Router 配置；需要 Python 3.11+ 的 tomllib 和有效的用户 config.toml");
  }
  if (!Array.isArray(values) || values.length !== 2) throw new Error("Router 配置格式无效");
  const [baseUrl, catalog] = values;
  // 只支持本机当前的无秘密 /v1 地址。带 caller capability 的地址不可放进 argv。
  const match = typeof baseUrl === "string" && /^http:\/\/127\.0\.0\.1:([1-9]\d{0,4})\/v1$/.exec(baseUrl);
  if (!match || Number(match[1]) > 65535) {
    throw new Error("Router 地址必须是本机 http://127.0.0.1:<port>/v1；带认证路径的配置须另行适配，拒绝传入命令");
  }
  if (typeof catalog !== "string" || !isAbsolute(catalog) || /[\r\n\0]/.test(catalog)) {
    throw new Error("Router 模型目录必须是绝对文件路径");
  }
  try {
    const stat = lstatSync(catalog);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 16 * 1024 * 1024) throw new Error();
    const data = JSON.parse(readFileSync(catalog, "utf8"));
    const models = Array.isArray(data.models) ? data.models.filter((x: { slug?: string } | null) => x?.slug === MODEL) : [];
    if (models.length !== 1 || !Array.isArray(models[0].supported_reasoning_levels)
      || !models[0].supported_reasoning_levels.some((x: { effort?: string } | null) => x?.effort === EFFORT)) throw new Error();
  } catch {
    throw new Error("Router 模型目录必须包含唯一的 DeepSeek V4.1 Flash 且支持 max");
  }
  return { baseUrl, catalog };
}
