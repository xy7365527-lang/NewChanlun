/** CLIProxyLocal 的无第三方依赖接线；wayfinder 可在无 node_modules 的宿主导入。 */
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export const CLIPROXY_BASE_URL = "http://127.0.0.1:8318";
export const CLIPROXY_CONFIG = join(homedir(), "Library/Application Support/CLIProxyLocal/config.yaml");

function unquote(value: string): string {
  return value.replace(/\s+#.*$/, "").replace(/^["']|["']$/g, "");
}

/** 从 config.yaml 文本取 access.api-keys 首项，不依赖 yaml 包。 */
export function extractFirstApiKey(yamlText: string): string | null {
  let inAccess = false;
  let keysIndent = -1;
  for (const raw of yamlText.split(/\r?\n/)) {
    if (/^\s*(#.*)?$/.test(raw)) continue;
    const indent = raw.length - raw.trimStart().length;
    const line = raw.trim();
    if (indent === 0) {
      inAccess = /^access:\s*$/.test(line);
      keysIndent = -1;
      continue;
    }
    if (!inAccess) continue;
    if (keysIndent < 0) {
      const match = /^api-keys:\s*(.*)$/.exec(line);
      if (!match) continue;
      const inline = match[1].trim();
      if (inline.startsWith("[")) {
        const first = inline.replace(/^\[|\]$/g, "").split(",")[0]?.trim();
        return first ? unquote(first) : null;
      }
      keysIndent = indent;
      continue;
    }
    if (indent <= keysIndent && !line.startsWith("-")) return null;
    if (line.startsWith("-")) {
      const value = unquote(line.slice(1).trim());
      return value || null;
    }
  }
  return null;
}

/** 密钥只在内存中返回；错误与日志不得包含配置内容。 */
export function readCliProxyAccessKey(configPath: string = CLIPROXY_CONFIG): string {
  let text: string;
  try {
    text = readFileSync(configPath, "utf8");
  } catch (error) {
    throw new Error(`[FAIL-loud #1547] 读不到 CLIProxyLocal 配置 ${configPath}：${String(error).slice(0, 200)}`);
  }
  const key = extractFirstApiKey(text);
  if (!key) throw new Error(`[FAIL-loud #1547] ${configPath} 里没有 access.api-keys 首项`);
  return key;
}

export function cliProxyEnv(
  configPath: string = CLIPROXY_CONFIG,
  baseUrl: string = CLIPROXY_BASE_URL,
): Record<string, string> {
  return {
    ANTHROPIC_BASE_URL: baseUrl,
    ANTHROPIC_AUTH_TOKEN: readCliProxyAccessKey(configPath),
  };
}
