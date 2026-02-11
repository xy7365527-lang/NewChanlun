/**
 * ChanTheoryPrimitive — 缠论多级别可视化 Primitive
 *
 * 在 candleSeries 上绘制：
 * - 笔线（蓝色，实线=confirmed / 虚线=延伸）
 * - 段线（橙色，更粗）
 * - 多级别中枢矩形框（级别越高颜色越醒目，L* 金色高亮）
 *
 * 级别是市场"长"出来的（递归涌现），不是预设的。
 * 颜色按级别自动分配：Level 1 橙 → Level 2 紫 → Level 3+ 红。
 */
import type {
  Coordinate,
  IPrimitivePaneRenderer,
  IPrimitivePaneView,
  Time,
} from "lightweight-charts";
import type { CanvasRenderingTarget2D } from "fancy-canvas";

import { PluginBase } from "./plugin-base";
import type { OverlayResponse } from "../types/overlay";

// ── 级别色阶配置 ──

interface LevelColorScheme {
  fill: string;
  border: string;
  candidateFill: string;
  candidateBorder: string;
}

/** 每个级别的颜色。级别越高越醒目。 */
const LEVEL_COLORS: LevelColorScheme[] = [
  // Level 1: 橙色（基础级别）
  {
    fill: "rgba(255,159,67,0.10)",
    border: "rgba(255,159,67,0.45)",
    candidateFill: "rgba(255,159,67,0.04)",
    candidateBorder: "rgba(255,159,67,0.2)",
  },
  // Level 2: 紫色
  {
    fill: "rgba(140,90,255,0.13)",
    border: "rgba(140,90,255,0.55)",
    candidateFill: "rgba(140,90,255,0.05)",
    candidateBorder: "rgba(140,90,255,0.25)",
  },
  // Level 3: 红色
  {
    fill: "rgba(255,60,90,0.15)",
    border: "rgba(255,60,90,0.6)",
    candidateFill: "rgba(255,60,90,0.06)",
    candidateBorder: "rgba(255,60,90,0.3)",
  },
  // Level 4+: 金色
  {
    fill: "rgba(255,200,40,0.16)",
    border: "rgba(255,200,40,0.65)",
    candidateFill: "rgba(255,200,40,0.06)",
    candidateBorder: "rgba(255,200,40,0.3)",
  },
];

/** L* 裁决中枢的高亮描边 */
const LSTAR_BORDER_COLOR = "rgba(255,215,0,0.9)";
const LSTAR_BORDER_WIDTH = 2.5;

// ── 配置 ──

export interface ChanDrawingSettings {
  strokeVisible: boolean;
  segmentVisible: boolean;
  centerBoxVisible: boolean;
  strokeColor: string;
  segmentColor: string;
  /** 向后兼容：Level-1 settled 中枢颜色 */
  centerSettledFill: string;
  centerSettledBorder: string;
  centerCandidateFill: string;
  centerCandidateBorder: string;
}

const DEFAULT_SETTINGS: ChanDrawingSettings = {
  strokeVisible: true,
  segmentVisible: true,
  centerBoxVisible: true,
  strokeColor: "rgba(80,160,255,0.8)",
  segmentColor: "rgba(255,159,67,0.9)",
  centerSettledFill: LEVEL_COLORS[0].fill,
  centerSettledBorder: LEVEL_COLORS[0].border,
  centerCandidateFill: LEVEL_COLORS[0].candidateFill,
  centerCandidateBorder: LEVEL_COLORS[0].candidateBorder,
};

// ── 预计算的屏幕坐标 ──

interface LineSegment {
  x0: Coordinate | null;
  y0: Coordinate | null;
  x1: Coordinate | null;
  y1: Coordinate | null;
  confirmed: boolean;
}

interface CenterBox {
  x0: Coordinate | null;
  y0: Coordinate | null;
  x1: Coordinate | null;
  y1: Coordinate | null;
  level: number;
  settled: boolean;
  isLStar: boolean;
}

// ── Renderer ──

class ChanPaneRenderer implements IPrimitivePaneRenderer {
  private _strokeLines: LineSegment[];
  private _segmentLines: LineSegment[];
  private _centerBoxes: CenterBox[];
  private _settings: ChanDrawingSettings;
  private _lastDrawDebugSignature = "";

  constructor(
    strokeLines: LineSegment[],
    segmentLines: LineSegment[],
    centerBoxes: CenterBox[],
    settings: ChanDrawingSettings
  ) {
    this._strokeLines = strokeLines;
    this._segmentLines = segmentLines;
    this._centerBoxes = centerBoxes;
    this._settings = settings;
  }

  draw(target: CanvasRenderingTarget2D) {
    target.useMediaCoordinateSpace((scope) => {
      const ctx = scope.context;
      const width = scope.mediaSize.width;
      const height = scope.mediaSize.height;
      const finiteStrokeCount = this._strokeLines.filter(
        (s) =>
          Number.isFinite(s.x0) &&
          Number.isFinite(s.y0) &&
          Number.isFinite(s.x1) &&
          Number.isFinite(s.y1)
      ).length;
      const finiteSegmentCount = this._segmentLines.filter(
        (s) =>
          Number.isFinite(s.x0) &&
          Number.isFinite(s.y0) &&
          Number.isFinite(s.x1) &&
          Number.isFinite(s.y1)
      ).length;
      const onScreenStrokeCount = this._strokeLines.filter(
        (s) =>
          Number.isFinite(s.x0) &&
          Number.isFinite(s.y0) &&
          Number.isFinite(s.x1) &&
          Number.isFinite(s.y1) &&
          (Math.min(s.x0 as number, s.x1 as number) <= width &&
            Math.max(s.x0 as number, s.x1 as number) >= 0) &&
          (Math.min(s.y0 as number, s.y1 as number) <= height &&
            Math.max(s.y0 as number, s.y1 as number) >= 0)
      ).length;
      const onScreenSegmentCount = this._segmentLines.filter(
        (s) =>
          Number.isFinite(s.x0) &&
          Number.isFinite(s.y0) &&
          Number.isFinite(s.x1) &&
          Number.isFinite(s.y1) &&
          (Math.min(s.x0 as number, s.x1 as number) <= width &&
            Math.max(s.x0 as number, s.x1 as number) >= 0) &&
          (Math.min(s.y0 as number, s.y1 as number) <= height &&
            Math.max(s.y0 as number, s.y1 as number) >= 0)
      ).length;
      const drawSig = [
        width,
        height,
        finiteStrokeCount,
        finiteSegmentCount,
        onScreenStrokeCount,
        onScreenSegmentCount,
      ].join("|");
      if (drawSig !== this._lastDrawDebugSignature) {
        this._lastDrawDebugSignature = drawSig;
      }
      ctx.save();
      try {
        // 1. 中枢矩形框（最底层，低级别先画，高级别后画）
        if (this._settings.centerBoxVisible) {
          this._drawCenterBoxes(ctx);
        }
        // 2. 段线（比笔线更粗，表示更高层结构）
        if (this._settings.segmentVisible) {
          this._drawLines(
            ctx,
            this._segmentLines,
            this._settings.segmentColor,
            3
          );
        }
        // 3. 笔线（最上层）
        if (this._settings.strokeVisible) {
          this._drawLines(
            ctx,
            this._strokeLines,
            this._settings.strokeColor,
            1
          );
        }
      } finally {
        ctx.restore();
      }
    });
  }

  private _drawLines(
    ctx: CanvasRenderingContext2D,
    lines: LineSegment[],
    color: string,
    width: number
  ) {
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.lineJoin = "round";
    ctx.lineCap = "round";

    for (const seg of lines) {
      if (
        seg.x0 === null ||
        seg.y0 === null ||
        seg.x1 === null ||
        seg.y1 === null
      )
        continue;
      ctx.setLineDash(seg.confirmed ? [] : [4, 3]);
      ctx.beginPath();
      ctx.moveTo(seg.x0, seg.y0);
      ctx.lineTo(seg.x1, seg.y1);
      ctx.stroke();
    }
    ctx.setLineDash([]);
  }

  private _drawCenterBoxes(ctx: CanvasRenderingContext2D) {
    // 按级别升序排列：低级别先画（底层），高级别后画（上层）
    const sorted = [...this._centerBoxes].sort((a, b) => a.level - b.level);

    for (const box of sorted) {
      if (
        box.x0 === null ||
        box.y0 === null ||
        box.x1 === null ||
        box.y1 === null
      )
        continue;

      const x = Math.min(box.x0, box.x1);
      const y = Math.min(box.y0, box.y1);
      const w = Math.abs(box.x1 - box.x0);
      const h = Math.abs(box.y1 - box.y0);

      // 级别色阶
      const colorIdx = Math.min(box.level - 1, LEVEL_COLORS.length - 1);
      const colors = LEVEL_COLORS[Math.max(0, colorIdx)];

      // 填充
      ctx.fillStyle = box.settled ? colors.fill : colors.candidateFill;
      ctx.fillRect(x, y, w, h);

      // 边框
      if (box.isLStar) {
        // L* 裁决中枢：金色高亮
        ctx.strokeStyle = LSTAR_BORDER_COLOR;
        ctx.lineWidth = LSTAR_BORDER_WIDTH;
      } else {
        ctx.strokeStyle = box.settled
          ? colors.border
          : colors.candidateBorder;
        ctx.lineWidth = 1 + Math.min(box.level - 1, 3) * 0.5;
      }
      ctx.setLineDash([]);
      ctx.strokeRect(x, y, w, h);
    }
  }

}

// ── PaneView ──

class ChanPaneView implements IPrimitivePaneView {
  private _source: ChanTheoryPrimitive;
  private _strokeLines: LineSegment[] = [];
  private _segmentLines: LineSegment[] = [];
  private _centerBoxes: CenterBox[] = [];

  constructor(source: ChanTheoryPrimitive) {
    this._source = source;
  }

  update() {
    const data = this._source.overlayData;
    if (!data) {
      this._strokeLines = [];
      this._segmentLines = [];
      this._centerBoxes = [];
      return;
    }

    const series = this._source.series;
    const timeScale = this._source.chart.timeScale();

    // 笔线
    this._strokeLines = data.strokes.map((s) => ({
      x0: timeScale.timeToCoordinate(s.t0 as unknown as Time),
      y0: series.priceToCoordinate(s.p0),
      x1: timeScale.timeToCoordinate(s.t1 as unknown as Time),
      y1: series.priceToCoordinate(s.p1),
      confirmed: s.confirmed,
    }));

    // 段线：按“共享笔连接点”渲染，保证连续且落在笔分型上。
    // 相邻段满足 s[i].s1 == s[i+1].s0 时，连接点取 shared stroke 的 p1/t1。
    this._segmentLines = [];
    let forcedStartOverrideCount = 0;
    let wouldForceStartCount = 0;
    let nativeStartOffFractalCount = 0;
    let nativeEndOffFractalCount = 0;
    let drawStartOffFractalCount = 0;
    let drawEndOffFractalCount = 0;
    let drawDirectionMismatchCount = 0;
    let backendDirPriceMismatchCount = 0;
    let renderVsBackendEndpointDeltaCount = 0;
    let polylineFallbackCount = 0;
    let linePathDeviationGt03Count = 0;
    let linePathDeviationMax = 0;
    let endpointTypeMismatchCount = 0;
    let endpointFieldMissingCount = 0;
    const linePathDeviationSamples: Array<Record<string, unknown>> = [];
    const segmentRenderAnchors: Array<{
      x0: Coordinate | null;
      y0: Coordinate | null;
      x1: Coordinate | null;
      y1: Coordinate | null;
    }> = [];
    const renderedEndpoints: Array<{
      startTime: number;
      startPrice: number;
      endTime: number;
      endPrice: number;
      dir: "up" | "down";
    }> = [];
    const segmentAlignSamples: Array<Record<string, unknown>> = [];
    const strokeEndpointForType = (
      stroke: OverlayResponse["strokes"][number] | undefined,
      fractalType: "top" | "bottom"
    ): { time: number; price: number } | null => {
      if (!stroke) return null;
      if (fractalType === "top") {
        return stroke.dir === "down"
          ? { time: stroke.t0, price: stroke.p0 }
          : { time: stroke.t1, price: stroke.p1 };
      }
      return stroke.dir === "down"
        ? { time: stroke.t1, price: stroke.p1 }
        : { time: stroke.t0, price: stroke.p0 };
    };
    for (let i = 0; i < data.segments.length; i++) {
      const s = data.segments[i];
      const startStroke = data.strokes[s.s0];
      const endStroke = data.strokes[s.s1];
      const hasEpFields = !!s.ep0 && !!s.ep1;
      if (!hasEpFields) endpointFieldMissingCount += 1;
      let startPrice = s.ep0?.price ?? s.p0;
      let startTime = s.ep0?.time ?? s.t0;
      let endPrice = s.ep1?.price ?? s.p1;
      let endTime = s.ep1?.time ?? s.t1;

      // 段方向语义期望：up => bottom->top; down => top->bottom
      const expectedStartType: "top" | "bottom" =
        s.dir === "up" ? "bottom" : "top";
      const expectedEndType: "top" | "bottom" =
        s.dir === "up" ? "top" : "bottom";
      const declaredStartType = s.ep0?.type ?? expectedStartType;
      const declaredEndType = s.ep1?.type ?? expectedEndType;
      if (declaredStartType !== expectedStartType || declaredEndType !== expectedEndType) {
        endpointTypeMismatchCount += 1;
      }

      const legacyStartPrice = i === 0 ? s.p0 : data.segments[i - 1].p1;
      const legacyStartTime = i === 0 ? s.t0 : data.segments[i - 1].t1;
      const pts = Array.isArray(s.stroke_points) ? s.stroke_points : [];
      const hasPoint = (t: number, p: number) =>
        pts.some((pt) => pt.time === t && Math.abs(pt.value - p) <= 1e-9);
      const forcedStart =
        i > 0 && (startTime !== s.t0 || Math.abs(startPrice - s.p0) > 1e-9);
      const wouldForceStart =
        i > 0 &&
        (legacyStartTime !== s.t0 || Math.abs(legacyStartPrice - s.p0) > 1e-9);
      const nativeStartOnFractal = hasPoint(s.t0, s.p0);
      const nativeEndOnFractal = hasPoint(s.t1, s.p1);
      const expectedStartStrokePt = strokeEndpointForType(startStroke, declaredStartType);
      const expectedEndStrokePt = strokeEndpointForType(endStroke, declaredEndType);
      const drawStartOnFractal =
        (expectedStartStrokePt &&
          startTime === expectedStartStrokePt.time &&
          Math.abs(startPrice - expectedStartStrokePt.price) <= 1e-9) ||
        hasPoint(startTime, startPrice);
      const drawEndOnFractal =
        (expectedEndStrokePt &&
          endTime === expectedEndStrokePt.time &&
          Math.abs(endPrice - expectedEndStrokePt.price) <= 1e-9) ||
        hasPoint(endTime, endPrice);
      const drawDirectionMismatch =
        (s.dir === "up" && startPrice > endPrice) ||
        (s.dir === "down" && startPrice < endPrice);
      const backendDirPriceMismatch =
        (s.dir === "up" && s.p0 >= s.p1) ||
        (s.dir === "down" && s.p0 <= s.p1);
      const renderVsBackendEndpointDelta =
        Math.abs(startPrice - s.p0) > 1e-9 ||
        Math.abs(endPrice - s.p1) > 1e-9 ||
        startTime !== s.t0 ||
        endTime !== s.t1;
      if (forcedStart) forcedStartOverrideCount += 1;
      if (wouldForceStart) wouldForceStartCount += 1;
      if (!nativeStartOnFractal) nativeStartOffFractalCount += 1;
      if (!nativeEndOnFractal) nativeEndOffFractalCount += 1;
      if (!drawStartOnFractal) drawStartOffFractalCount += 1;
      if (!drawEndOnFractal) drawEndOffFractalCount += 1;
      if (drawDirectionMismatch) drawDirectionMismatchCount += 1;
      if (backendDirPriceMismatch) backendDirPriceMismatchCount += 1;
      if (renderVsBackendEndpointDelta) renderVsBackendEndpointDeltaCount += 1;
      if (
        segmentAlignSamples.length < 3 &&
        (
          wouldForceStart ||
          !nativeStartOnFractal ||
          !drawStartOnFractal ||
          drawDirectionMismatch ||
          backendDirPriceMismatch
        )
      ) {
        segmentAlignSamples.push({
          id: s.id,
          i,
          t0: s.t0,
          p0: s.p0,
          t1: s.t1,
          p1: s.p1,
          drawStartTime: startTime,
          drawStartPrice: startPrice,
          forcedStart,
          wouldForceStart,
          legacyStartTime,
          legacyStartPrice,
          endTime,
          endPrice,
          nativeStartOnFractal,
          nativeEndOnFractal,
          drawStartOnFractal,
          drawEndOnFractal,
          drawDirectionMismatch,
          backendDirPriceMismatch,
          renderVsBackendEndpointDelta,
          expectedStartType,
          expectedEndType,
          declaredStartType,
          declaredEndType,
          hasEpFields,
          strokePointsLen: pts.length,
          firstPt: pts[0] ?? null,
          lastPt: pts[pts.length - 1] ?? null,
        });
      }
      if (pts.length >= 3 && endTime !== startTime) {
        let segMaxDev = 0;
        const dt = endTime - startTime;
        for (const pt of pts) {
          const alpha = (pt.time - startTime) / dt;
          const interp = startPrice + (endPrice - startPrice) * alpha;
          const dev = Math.abs(pt.value - interp);
          if (dev > segMaxDev) segMaxDev = dev;
        }
        if (segMaxDev > linePathDeviationMax) linePathDeviationMax = segMaxDev;
        if (segMaxDev > 0.3) {
          linePathDeviationGt03Count += 1;
          if (linePathDeviationSamples.length < 5) {
            linePathDeviationSamples.push({
              id: s.id,
              i,
              dir: s.dir,
              s0: s.s0,
              s1: s.s1,
              maxDev: segMaxDev,
              points: pts.length,
              startTime,
              endTime,
            });
          }
        }
      }
      renderedEndpoints.push({
        startTime,
        startPrice,
        endTime,
        endPrice,
        dir: s.dir,
      });
      // 段绘制模式：语义端点单线（避免与笔路径完全重叠）
      const lineAnchor = {
        x0: timeScale.timeToCoordinate(startTime as unknown as Time),
        y0: series.priceToCoordinate(startPrice),
        x1: timeScale.timeToCoordinate(endTime as unknown as Time),
        y1: series.priceToCoordinate(endPrice),
      };
      this._segmentLines.push({
        x0: lineAnchor.x0,
        y0: lineAnchor.y0,
        x1: lineAnchor.x1,
        y1: lineAnchor.y1,
        confirmed: s.confirmed,
      });
      segmentRenderAnchors.push(lineAnchor);
    }

    // 局部极值一致性：连接点序列中，top 应高于两侧，bottom 应低于两侧。
    let localExtremaViolationCount = 0;
    const localExtremaViolationSamples: Array<Record<string, unknown>> = [];
    if (renderedEndpoints.length > 0) {
      const joins: Array<{ time: number; price: number }> = [];
      const joinTypes: Array<"top" | "bottom"> = [];
      joins.push({
        time: renderedEndpoints[0].startTime,
        price: renderedEndpoints[0].startPrice,
      });
      joinTypes.push(renderedEndpoints[0].dir === "up" ? "bottom" : "top");
      for (let i = 0; i < renderedEndpoints.length; i += 1) {
        joins.push({
          time: renderedEndpoints[i].endTime,
          price: renderedEndpoints[i].endPrice,
        });
        joinTypes.push(renderedEndpoints[i].dir === "up" ? "top" : "bottom");
      }
      for (let k = 1; k < joins.length - 1; k += 1) {
        const left = joins[k - 1].price;
        const mid = joins[k].price;
        const right = joins[k + 1].price;
        const jt = joinTypes[k];
        const bad =
          (jt === "top" && (mid < left || mid < right)) ||
          (jt === "bottom" && (mid > left || mid > right));
        if (bad) {
          localExtremaViolationCount += 1;
          if (localExtremaViolationSamples.length < 3) {
            localExtremaViolationSamples.push({
              k,
              joinType: jt,
              left,
              mid,
              right,
            });
          }
        }
      }
    }

    // 多级别中枢矩形框
    this._centerBoxes = this._buildCenterBoxes(data, series, timeScale);
  }

  /** 从 overlay 数据构建多级别中枢 box 列表 */
  private _buildCenterBoxes(
    data: OverlayResponse,
    series: ChanTheoryPrimitive["series"],
    timeScale: ReturnType<ChanTheoryPrimitive["chart"]["timeScale"]>
  ): CenterBox[] {
    const boxes: CenterBox[] = [];
    const lstarLevel = data.lstar?.level ?? -1;
    const lstarCenterId = data.lstar?.center_id ?? -1;

    // 优先使用 levels 数组（多级别）
    if (data.levels && data.levels.length > 0) {
      for (const lv of data.levels) {
        for (const c of lv.centers) {
          boxes.push({
            x0: timeScale.timeToCoordinate(c.t0 as unknown as Time),
            y0: series.priceToCoordinate(c.ZG),
            x1: timeScale.timeToCoordinate(c.t1 as unknown as Time),
            y1: series.priceToCoordinate(c.ZD),
            level: lv.level,
            settled: c.kind === "settled",
            isLStar: lv.level === lstarLevel && c.id === lstarCenterId,
          });
        }
      }
    } else {
      // 向后兼容：只有顶层 centers（视为 Level 1）
      for (const c of data.centers) {
        boxes.push({
          x0: timeScale.timeToCoordinate(c.t0 as unknown as Time),
          y0: series.priceToCoordinate(c.ZG),
          x1: timeScale.timeToCoordinate(c.t1 as unknown as Time),
          y1: series.priceToCoordinate(c.ZD),
          level: 1,
          settled: c.kind === "settled",
          isLStar: lstarLevel === 1 && c.id === lstarCenterId,
        });
      }
    }

    return boxes;
  }

  renderer() {
    return new ChanPaneRenderer(
      this._strokeLines,
      this._segmentLines,
      this._centerBoxes,
      this._source.settings
    );
  }

  zOrder(): "bottom" | "normal" | "top" {
    return "bottom";
  }
}

// ── 主 Primitive ──

export class ChanTheoryPrimitive extends PluginBase {
  private _data: OverlayResponse | null = null;
  private _settings: ChanDrawingSettings;
  private _paneView: ChanPaneView;

  constructor(settings?: Partial<ChanDrawingSettings>) {
    super();
    this._settings = { ...DEFAULT_SETTINGS, ...settings };
    this._paneView = new ChanPaneView(this);
  }

  get overlayData() {
    return this._data;
  }

  get settings() {
    return this._settings;
  }

  /** 更新 overlay 数据，触发重绘 */
  setData(overlay: OverlayResponse | null) {
    this._data = overlay;
    this.requestUpdate();
  }

  /** 更新设置 */
  applySettings(s: Partial<ChanDrawingSettings>) {
    this._settings = { ...this._settings, ...s };
    this.requestUpdate();
  }

  updateAllViews() {
    this._paneView.update();
  }

  paneViews() {
    return [this._paneView];
  }
}
