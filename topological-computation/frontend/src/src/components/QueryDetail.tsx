import { T, FONT } from "../tokens";
import type { QueryResponse } from "../types";
import { fToColor } from "../tokens";

interface Props {
  result: QueryResponse;
  onSelectNeighbor?: (concept: string) => void;
}

export function QueryDetail({ result, onSelectNeighbor }: Props) {
  if (!result.found || !result.vertex) {
    return (
      <div style={{
        padding: 16, fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
      }}>
        概念未找到。
      </div>
    );
  }

  const v = result.vertex;

  return (
    <div style={{
      padding: "12px 14px", fontFamily: FONT.mono, fontSize: 11, color: T.textDim,
      overflow: "auto", flex: 1,
    }}>
      {/* Header */}
      <div style={{ marginBottom: 12 }}>
        <span style={{ color: fToColor(v.f_avg), fontSize: 13, fontWeight: 600 }}>
          {v.label}
        </span>
        {v.settled && (
          <span style={{ color: T.settled, marginLeft: 10, fontSize: 9, letterSpacing: "0.1em" }}>
            ● SETTLED
          </span>
        )}
      </div>

      {/* Stats row */}
      <div style={{ display: "flex", gap: 16, marginBottom: 12, fontSize: 10 }}>
        <span>f_avg <span style={{ color: fToColor(v.f_avg) }}>{v.f_avg.toFixed(1)}</span></span>
        <span>degree <span style={{ color: T.text }}>{v.degree}</span></span>
        <span>rings <span style={{ color: T.text }}>{v.rings}</span></span>
        <span>settled_rings <span style={{ color: T.settled }}>{v.settled_rings}</span></span>
      </div>

      {/* f-terrain zones */}
      {(result.f_terrain.fold_zone.length > 0 ||
        result.f_terrain.gray_zone.length > 0 ||
        result.f_terrain.negate_zone.length > 0) && (
        <div style={{ marginBottom: 12 }}>
          <div style={{ color: T.textMuted, marginBottom: 4, letterSpacing: "0.1em", fontSize: 9 }}>
            F-TERRAIN
          </div>
          {result.f_terrain.fold_zone.length > 0 && (
            <div style={{ marginBottom: 3 }}>
              <span style={{ color: T.fLow }}>fold </span>
              <span style={{ color: T.textDim }}>{result.f_terrain.fold_zone.join(", ")}</span>
            </div>
          )}
          {result.f_terrain.gray_zone.length > 0 && (
            <div style={{ marginBottom: 3 }}>
              <span style={{ color: T.fMid }}>gray </span>
              <span style={{ color: T.textDim }}>{result.f_terrain.gray_zone.join(", ")}</span>
            </div>
          )}
          {result.f_terrain.negate_zone.length > 0 && (
            <div style={{ marginBottom: 3 }}>
              <span style={{ color: T.fHigh }}>negate </span>
              <span style={{ color: T.textDim }}>{result.f_terrain.negate_zone.join(", ")}</span>
            </div>
          )}
        </div>
      )}

      {/* Neighbors */}
      {result.neighbors.length > 0 && (
        <div>
          <div style={{ color: T.textMuted, marginBottom: 4, letterSpacing: "0.1em", fontSize: 9 }}>
            邻居 ({result.neighbors.length})
          </div>
          {result.neighbors.slice(0, 12).map((nb, i) => (
            <div
              key={i}
              onClick={() => onSelectNeighbor?.(nb.label)}
              style={{
                display: "flex", gap: 8, padding: "3px 4px",
                cursor: "pointer", borderRadius: 3,
                transition: "background 0.1s",
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLDivElement).style.background = T.bgHover;
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLDivElement).style.background = "transparent";
              }}
            >
              <span style={{ color: fToColor(nb.f), width: 30, flexShrink: 0 }}>
                {nb.f.toFixed(0)}
              </span>
              <span style={{
                flex: 1, color: T.textDim,
                overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
              }}>
                {nb.label}
              </span>
              <span style={{
                color: nb.edge_type === "negation" ? T.fHigh :
                       nb.edge_type === "defines" ? T.accent : T.textMuted,
                fontSize: 9,
              }}>
                {nb.edge_type}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
