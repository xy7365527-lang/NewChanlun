/**
 * EventMarkerManager — 使用 Lightweight Charts SeriesMarkers API
 * 在 K线上绘制笔事件标记和线段事件标记。
 */
import type { ISeriesApi, SeriesMarker, Time } from "lightweight-charts";
import type { ChanEvent } from "../types/events";

/** epoch 秒 → LW Charts UTCTimestamp */
function epochToTime(epoch: number): Time {
  return epoch as unknown as Time;
}

export class EventMarkerManager {
  private markers: SeriesMarker<Time>[] = [];
  private series: ISeriesApi<"Candlestick"> | null = null;

  attach(series: ISeriesApi<"Candlestick">): void {
    this.series = series;
  }

  detach(): void {
    this.series = null;
    this.markers = [];
  }

  addEvent(event: ChanEvent): void {
    if (!this.series) return;

    let marker: SeriesMarker<Time> | null = null;
    const time = epochToTime(event.bar_ts);

    switch (event.event_type) {
      case "stroke_settled": {
        if (event.direction === "up") {
          marker = {
            time,
            position: "belowBar",
            color: "#26a69a",
            shape: "arrowUp",
            text: "S\u2191",
          };
        } else {
          marker = {
            time,
            position: "aboveBar",
            color: "#ef5350",
            shape: "arrowDown",
            text: "S\u2193",
          };
        }
        break;
      }

      case "stroke_invalidated": {
        marker = {
          time,
          position: "aboveBar",
          color: "#666",
          shape: "circle",
          text: "\u2715",
        };
        break;
      }

      case "stroke_candidate": {
        if (event.direction === "up") {
          marker = {
            time,
            position: "belowBar",
            color: "#42a5f5",
            shape: "circle",
            text: "C\u2191",
          };
        } else {
          marker = {
            time,
            position: "aboveBar",
            color: "#42a5f5",
            shape: "circle",
            text: "C\u2193",
          };
        }
        break;
      }

      case "stroke_extended":
        // 不需要 marker
        return;

      case "segment_settle": {
        const payload = event as unknown as Record<string, unknown>;
        const segDir = payload.direction as string;
        if (segDir === "up") {
          marker = {
            time,
            position: "aboveBar",
            color: "#ef5350",
            shape: "square",
            text: "Seg\u2191",
          };
        } else {
          marker = {
            time,
            position: "belowBar",
            color: "#26a69a",
            shape: "square",
            text: "Seg\u2193",
          };
        }
        break;
      }

      case "segment_break_pending": {
        marker = {
          time,
          position: "aboveBar",
          color: "#999",
          shape: "square",
          text: "Brk?",
        };
        break;
      }

      case "segment_invalidate":
        // 不显示 marker（避免噪音）
        return;

      case "zhongshu_candidate": {
        marker = {
          time,
          position: "belowBar",
          color: "#ff9800",
          shape: "square",
          text: "ZS?",
        };
        break;
      }

      case "zhongshu_settle": {
        const zsPayload = event as unknown as Record<string, unknown>;
        const breakDir = zsPayload.break_direction as string;
        if (breakDir === "up") {
          marker = {
            time,
            position: "aboveBar",
            color: "#ef5350",
            shape: "square",
            text: "ZS\u2191",
          };
        } else {
          marker = {
            time,
            position: "belowBar",
            color: "#26a69a",
            shape: "square",
            text: "ZS\u2193",
          };
        }
        break;
      }

      case "zhongshu_invalidate":
        // 不显示 marker（避免噪音）
        return;

      case "move_candidate": {
        const mcPayload = event as unknown as Record<string, unknown>;
        const mcKind = mcPayload.kind as string;
        const mcDir = mcPayload.direction as string;
        if (mcKind === "trend" && mcDir === "up") {
          marker = {
            time,
            position: "belowBar",
            color: "#4caf50",
            shape: "square",
            text: "M↑",
          };
        } else if (mcKind === "trend" && mcDir === "down") {
          marker = {
            time,
            position: "aboveBar",
            color: "#f44336",
            shape: "square",
            text: "M↓",
          };
        } else {
          marker = {
            time,
            position: "belowBar",
            color: "#2196f3",
            shape: "square",
            text: "M?",
          };
        }
        break;
      }

      case "move_settle": {
        const msPayload = event as unknown as Record<string, unknown>;
        const msDir = msPayload.direction as string;
        if (msDir === "up") {
          marker = {
            time,
            position: "belowBar",
            color: "#388e3c",
            shape: "circle",
            text: "M✓",
          };
        } else {
          marker = {
            time,
            position: "aboveBar",
            color: "#c62828",
            shape: "circle",
            text: "M✓",
          };
        }
        break;
      }

      case "move_invalidate":
        // 不显示 marker（避免噪音）
        return;
    }

    if (marker) {
      this.markers.push(marker);
      // markers 必须按 time 排序
      this.markers.sort((a, b) => (a.time as unknown as number) - (b.time as unknown as number));
      this.series.setMarkers(this.markers);
    }
  }

  clear(): void {
    this.markers = [];
    if (this.series) {
      this.series.setMarkers([]);
    }
  }
}
