import { readFile } from "node:fs/promises";

import typescript from "typescript";

export function resolve(specifier, context, defaultResolve) {
  return defaultResolve(specifier, context, defaultResolve);
}

export async function load(url, context, defaultLoad) {
  if (!url.startsWith("file:") || !new URL(url).pathname.endsWith(".ts")) {
    return defaultLoad(url, context, defaultLoad);
  }

  const source = await readFile(new URL(url), "utf8");
  const { outputText } = typescript.transpileModule(source, {
    fileName: new URL(url).pathname,
    compilerOptions: {
      module: typescript.ModuleKind.ESNext,
      target: typescript.ScriptTarget.ES2022,
    },
  });

  return {
    format: "module",
    shortCircuit: true,
    source: outputText,
  };
}
