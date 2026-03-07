import { T, FONT } from "../tokens";
import type { OperationStats } from "../types";

interface Props {
  operations: OperationStats | null;
}

const OP_COLORS: Record<string, string> = {
  fold: T.fLow,
  negate: T.fHigh,
  sublate: T.accent,
  walk: T.textMuted,
};

export function OperationsSummary({ operations }: Props) {
  if (!operations) return null;

  const entries = Object.entries(operations) as Array<
    [string, { count: number; blocked: number }]
  >;

  return (
    <div style={{
      display: "flex", gap: 12, padding: "8px 12px",
      fontFamily: FONT.mono, fontSize: 10, color: T.textDim,
      borderTop: `1px solid ${T.border}`,
      flexShrink: 0,
    }}>
      {entries.map(([op, stats]) => (
        <div key={op} style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <span style={{ color: OP_COLORS[op] ?? T.textDim }}>{op}</span>
          <span style={{ color: T.text }}>{stats.count.toLocaleString()}</span>
          {stats.blocked > 0 && (
            <span style={{ color: T.fMid }}>({stats.blocked} blocked)</span>
          )}
        </div>
      ))}
    </div>
  );
}
