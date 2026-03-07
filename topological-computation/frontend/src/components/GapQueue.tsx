import { T, FONT } from "../tokens";
import type { GapEntry } from "../types";

interface Props {
  gaps: GapEntry[];
}

function statusColor(status: string): string {
  switch (status) {
    case "searching": return T.feeding;
    case "unfed": return T.fHigh;
    default: return T.textMuted;
  }
}

export function GapQueue({ gaps }: Props) {
  return (
    <div style={{
      padding: "8px 12px", fontFamily: FONT.mono, fontSize: 10,
      borderTop: `1px solid ${T.border}`,
      flexShrink: 0,
    }}>
      <div style={{ color: T.textDim, marginBottom: 4, letterSpacing: "0.1em" }}>
        GAP QUEUE {gaps.length > 0 && <span style={{ color: T.fMid }}>({gaps.length})</span>}
      </div>
      {gaps.length === 0 ? (
        <div style={{ color: T.textMuted, fontSize: 9 }}>无 gap</div>
      ) : (
        gaps.slice(0, 8).map((g, i) => (
          <div key={i} style={{
            display: "flex", alignItems: "center", gap: 8, padding: "3px 0",
            color: statusColor(g.status),
          }}>
            <span style={{
              width: 6, height: 6, borderRadius: "50%",
              background: statusColor(g.status),
              flexShrink: 0,
            }} />
            <span style={{ flex: 1, color: T.textDim, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {g.concept}
            </span>
            <span>deg={g.degree}</span>
            <span style={{ color: T.textMuted }}>{g.status}</span>
          </div>
        ))
      )}
    </div>
  );
}
