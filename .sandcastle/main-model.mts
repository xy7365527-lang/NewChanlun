/** 当前主控与全部本机受管工蜂共用的唯一模型（用户 2026-10-25，#1617）。 */
export const MAIN_MODEL = "gpt-6.1-sol";

/** 可配置入口只接受主控同型；其它值 fail-loud，不回退旧模型。 */
export const SUPPORTED_MAIN_MODELS: readonly string[] = [MAIN_MODEL];
