/**
 * PluginBase — 简化版 (from LW Charts plugin-examples)
 * 提供 chart/series 引用 + requestUpdate()
 */
import type {
  IChartApi,
  ISeriesApi,
  ISeriesPrimitive,
  SeriesAttachedParameter,
  SeriesType,
  Time,
} from "lightweight-charts";

export abstract class PluginBase implements ISeriesPrimitive<Time> {
  private _chart: IChartApi | undefined;
  private _series: ISeriesApi<SeriesType> | undefined;
  private _requestUpdate?: () => void;

  public attached({ chart, series, requestUpdate }: SeriesAttachedParameter<Time>) {
    this._chart = chart;
    this._series = series as ISeriesApi<SeriesType>;
    this._requestUpdate = requestUpdate;
    this.requestUpdate();
  }

  public detached() {
    this._chart = undefined;
    this._series = undefined;
    this._requestUpdate = undefined;
  }

  protected requestUpdate(): void {
    this._requestUpdate?.();
  }

  public get chart(): IChartApi {
    if (!this._chart) throw new Error("Plugin not attached");
    return this._chart;
  }

  public get series(): ISeriesApi<SeriesType> {
    if (!this._series) throw new Error("Plugin not attached");
    return this._series;
  }
}
