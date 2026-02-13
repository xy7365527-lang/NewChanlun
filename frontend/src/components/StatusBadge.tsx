import type { OverlayLStar } from "../types/overlay";

interface Props {
  lstar: OverlayLStar | null;
  totalLevels?: number;
}

/** 级别色阶 —— 与 ChanTheoryPrimitive 中 LEVEL_COLORS 对应 */
const LEVEL_BADGE_COLORS = [
  "#ff9f43", // Level 1: 橙
  "#8c5aff", // Level 2: 紫
  "#ff3c5a", // Level 3: 红
  "#ffc828", // Level 4+: 金
];

function levelColor(level: number): string {
  return LEVEL_BADGE_COLORS[Math.min(level - 1, LEVEL_BADGE_COLORS.length - 1)] ?? "#888";
}

export function StatusBadge({ lstar, totalLevels = 0 }: Props) {
  if (!lstar) {
    const depthStr = totalLevels > 0 ? ` │ ${totalLevels}层` : "";
    return (
      <div className="status-badge">
        <span style={{ color: "#888" }}>L★—</span>
        {`  无裁决锚${depthStr}`}
      </div>
    );
  }

  const lvl = lstar.level;
  const color = levelColor(lvl);
  const depthStr = totalLevels > 0 ? ` │ ${totalLevels}层` : "";

  return (
    <div className="status-badge">
      <span style={{ color, fontWeight: "bold" }}>L★{lvl}</span>
      {`  ${lstar.regime}${depthStr}`}
    </div>
  );
}
