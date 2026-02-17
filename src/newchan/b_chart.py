"""B 系统 — TradingView 风格前端 HTML 应用

生成完整的单页应用 HTML，通过 /api/* 与 Python bottle 后端交互。
支持：品种搜索/切换、动态指标添加删除、多图布局、合成品种。
"""

from __future__ import annotations

from pathlib import Path

# lightweight-charts JS
try:
    import lightweight_charts as _lwc
    _JS_DIR = Path(_lwc.__file__).parent / "js"
except ImportError:
    _JS_DIR = None


def _load_js() -> str:
    if _JS_DIR:
        return (_JS_DIR / "lightweight-charts.js").read_text(encoding="utf-8")
    raise RuntimeError("lightweight-charts 未安装")


def build_app_html() -> str:
    """构建完整的图表 SPA HTML。"""
    lw_js = _load_js()
    return _HTML_TEMPLATE.replace("/* __LW_CHARTS_JS__ */", lw_js)


# ------------------------------------------------------------------
# 完整 HTML 模板
# ------------------------------------------------------------------

_HTML_TEMPLATE = r"""<!DOCTYPE html>
<html lang="zh"><head><meta charset="UTF-8">
<title>NewChan Chart</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#131722;color:#d1d4dc;font-family:'Segoe UI',system-ui,sans-serif;overflow:hidden;height:100vh;display:flex;flex-direction:column}
/* 顶部工具栏 */
#toolbar{display:flex;align-items:center;gap:6px;padding:4px 10px;background:#1e222d;border-bottom:1px solid #2a2e39;flex-shrink:0;height:36px;z-index:100}
#toolbar input,#toolbar select,#toolbar button{font-size:12px;height:26px}
#search-wrap{position:relative}
#sym-input{width:160px;background:#2a2e39;border:1px solid #363a45;color:#d1d4dc;padding:0 6px;border-radius:3px}
#sym-input:focus{outline:none;border-color:#2962FF}
#search-dropdown{display:none;position:absolute;top:28px;left:0;width:320px;max-height:300px;overflow-y:auto;background:#1e222d;border:1px solid #363a45;border-radius:4px;z-index:300;box-shadow:0 4px 12px rgba(0,0,0,0.5)}
#search-dropdown.show{display:block}
.sr-item{display:flex;justify-content:space-between;align-items:center;padding:6px 10px;cursor:pointer;font-size:12px;border-bottom:1px solid #2a2e39}
.sr-item:hover{background:#2a2e39}
.sr-sym{font-weight:600;color:#d1d4dc}
.sr-desc{color:#787b86;font-size:11px;max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.sr-badge{font-size:10px;padding:1px 5px;border-radius:2px;background:#2a2e39;color:#787b86}
.sr-badge.cache{color:#26a69a}
.sr-badge.ibkr{color:#2962FF}
#sym-list{background:#2a2e39;border:1px solid #363a45;color:#d1d4dc;border-radius:3px;max-width:160px}
#conn-status{width:8px;height:8px;border-radius:50%;display:inline-block;margin-right:4px}
#conn-status.on{background:#26a69a}
#conn-status.off{background:#ef5350}
#conn-status.unknown{background:#787b86}
.tb-btn{background:#2a2e39;color:#787b86;border:1px solid #363a45;padding:0 8px;border-radius:3px;cursor:pointer}
.tb-btn:hover{background:#363a45;color:#d1d4dc}
.tb-btn.active{background:#2962FF;color:#fff;border-color:#2962FF}
.tb-sep{width:1px;height:20px;background:#363a45}
#status{margin-left:auto;font-size:11px;color:#787b86}
/* 布局容器 */
#layout{flex:1;display:grid;gap:1px;background:#2a2e39;overflow:hidden}
.chart-cell{background:#131722;position:relative;overflow:hidden;display:flex;flex-direction:column}
.cell-toolbar{display:flex;align-items:center;gap:3px;padding:2px 6px;background:#1a1e2b;flex-shrink:0;height:24px}
.cell-toolbar .tf-btn{font-size:11px;background:transparent;color:#787b86;border:none;padding:1px 5px;border-radius:2px;cursor:pointer}
.cell-toolbar .tf-btn:hover{color:#d1d4dc}
.cell-toolbar .tf-btn.active{color:#2962FF;font-weight:600}
.cell-sym{font-size:11px;font-weight:600;color:#d1d4dc;margin-right:6px}
.cell-body{flex:1;display:flex;flex-direction:column;overflow:hidden}
.main-chart-wrap{flex:1;min-height:0}
.subchart-wrap{height:120px;border-top:1px solid #2a2e39;flex-shrink:0;position:relative}
.subchart-label{position:absolute;top:2px;left:6px;font-size:10px;color:#787b86;z-index:2}
.subchart-close{position:absolute;top:1px;right:6px;font-size:12px;color:#787b86;cursor:pointer;z-index:2}
.subchart-close:hover{color:#ef5350}
/* NewChan overlay 状态角标 */
.newchan-status{position:absolute;left:8px;top:8px;z-index:50;padding:4px 6px;border-radius:4px;background:rgba(0,0,0,.45);color:#d1d4dc;font-size:12px;line-height:1.2;pointer-events:none;white-space:pre}
/* 指标面板（弹窗） */
.modal-bg{display:none;position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.5);z-index:200}
.modal-bg.show{display:flex;align-items:center;justify-content:center}
.modal{background:#1e222d;border:1px solid #363a45;border-radius:6px;padding:16px;min-width:320px;max-width:500px}
.modal h3{font-size:14px;margin-bottom:10px;color:#d1d4dc}
.modal .ind-item{display:flex;align-items:center;justify-content:space-between;padding:6px 8px;border-radius:3px;cursor:pointer;font-size:13px}
.modal .ind-item:hover{background:#2a2e39}
.modal .ind-item .badge{font-size:10px;color:#787b86;background:#2a2e39;padding:1px 5px;border-radius:2px}
.modal .close-btn{float:right;cursor:pointer;color:#787b86;font-size:16px}
.modal .close-btn:hover{color:#ef5350}
/* 合成品种面板 */
.synth-row{display:flex;gap:6px;margin-top:8px;align-items:center}
.synth-row input,.synth-row select{background:#2a2e39;border:1px solid #363a45;color:#d1d4dc;padding:4px 6px;border-radius:3px;font-size:12px}
.synth-row button{background:#2962FF;color:#fff;border:none;padding:4px 12px;border-radius:3px;cursor:pointer;font-size:12px}
</style>
</head><body>

<!-- 顶部工具栏 -->
<div id="toolbar">
  <span id="conn-status" class="unknown" title="IBKR 连接状态"></span>
  <div id="search-wrap">
    <input id="sym-input" placeholder="搜索品种 (如 crude, CL, gold)..." autocomplete="off">
    <div id="search-dropdown"></div>
  </div>
  <select id="sym-list"><option value="">-- 已缓存 --</option></select>
  <div class="tb-sep"></div>
  <button class="tb-btn" onclick="showIndicatorModal()">+ 指标</button>
  <button class="tb-btn" onclick="showSyntheticModal()">+ 合成</button>
  <div class="tb-sep"></div>
  <span style="font-size:11px;color:#787b86">布局:</span>
  <button class="tb-btn layout-btn" data-layout="1x1" title="1x1">▣</button>
  <button class="tb-btn layout-btn" data-layout="2x1" title="2列">▥</button>
  <button class="tb-btn layout-btn" data-layout="1x2" title="2行">▤</button>
  <button class="tb-btn layout-btn" data-layout="2x2" title="2x2">▦</button>
  <button class="tb-btn" id="conn-btn" onclick="reconnectIBKR()" title="重新连接 IBKR">连接</button>
  <span id="status"></span>
</div>

<!-- 布局容器 -->
<div id="layout"></div>

<!-- 指标选择弹窗 -->
<div id="indicator-modal" class="modal-bg" onclick="if(event.target===this)this.classList.remove('show')">
  <div class="modal">
    <span class="close-btn" onclick="document.getElementById('indicator-modal').classList.remove('show')">&times;</span>
    <h3>添加指标</h3>
    <div id="indicator-list"></div>
  </div>
</div>

<!-- 合成品种弹窗 -->
<div id="synthetic-modal" class="modal-bg" onclick="if(event.target===this)this.classList.remove('show')">
  <div class="modal">
    <span class="close-btn" onclick="document.getElementById('synthetic-modal').classList.remove('show')">&times;</span>
    <h3>合成品种</h3>
    <div class="synth-row">
      <input id="synth-a" placeholder="品种A" style="width:70px">
      <select id="synth-op"><option value="spread">A - B</option><option value="ratio">A / B</option></select>
      <input id="synth-b" placeholder="品种B" style="width:70px">
      <button onclick="createSynthetic()">生成</button>
    </div>
    <div id="synth-status" style="font-size:11px;color:#787b86;margin-top:6px"></div>
  </div>
</div>

<script>/* __LW_CHARTS_JS__ */</script>
<script>
// =====================================================================
// 全局状态
// =====================================================================
const API = '';
let panels = [];           // ChartPanel 实例数组
let activePanel = null;    // 当前激活的面板
let indicatorDefs = [];    // 从服务器获取的指标定义
let currentLayout = '1x1';
const DEFAULT_TF = '1m';
const TF_LIST = [];

// =====================================================================
// 工具函数
// =====================================================================
function parseTime(t){ return Math.floor(new Date(t+'Z').getTime()/1000); }
function convertArr(arr){ return arr.map(d=>({...d, time:parseTime(d.time)})); }
async function api(path, opts){ const r=await fetch(API+path,opts); return r.json(); }
function setStatus(msg){ document.getElementById('status').textContent=msg; }
function getTFSeconds(tf){
  const map={'1m':60,'5m':300,'15m':900,'30m':1800,'1h':3600,'4h':14400,'1d':86400,'1w':604800};
  return map[tf]||60;
}

// =====================================================================
// ChartPanel 类 — 每个格子一个独立图表
// =====================================================================
class ChartPanel {
  constructor(container){
    this.container = container;
    this.symbol = '';
    this.interval = '1min';
    this.tf = DEFAULT_TF;
    this.indicators = [];  // [{id, name, params, series:[], subchart?, subEl?, subChart?}]
    this._idCounter = 0;
    this._build();
  }

  _build(){
    this.container.innerHTML = '';
    this.container.classList.add('chart-cell');
    // 单元格工具栏
    const tb = document.createElement('div'); tb.className='cell-toolbar';
    this.symLabel = document.createElement('span'); this.symLabel.className='cell-sym'; this.symLabel.textContent='--';
    tb.appendChild(this.symLabel);
    TF_LIST.forEach(tf=>{
      const b=document.createElement('button'); b.className='tf-btn'; b.textContent=tf; b.dataset.tf=tf;
      if(tf===this.tf) b.classList.add('active');
      b.onclick=()=>this.setTF(tf);
      tb.appendChild(b);
    });
    this.container.appendChild(tb);
    this.tbEl = tb;
    // 主体
    const body = document.createElement('div'); body.className='cell-body';
    this.mainWrap = document.createElement('div'); this.mainWrap.className='main-chart-wrap';
    body.appendChild(this.mainWrap);
    this.subContainer = body;  // subchart 追加到 body 里
    this.container.appendChild(body);
    // 创建 lightweight chart
    this.chart = LightweightCharts.createChart(this.mainWrap, {
      layout:{background:{color:'#131722'},textColor:'#d1d4dc'},
      grid:{vertLines:{color:'#1e222d'},horzLines:{color:'#1e222d'}},
      crosshair:{mode:LightweightCharts.CrosshairMode.Normal},
      rightPriceScale:{borderColor:'#2a2e39'},
      timeScale:{borderColor:'#2a2e39',timeVisible:true,secondsVisible:false},
    });
    this.candleSeries = this.chart.addCandlestickSeries({
      upColor:'#26a69a',downColor:'#ef5350',borderUpColor:'#26a69a',borderDownColor:'#ef5350',
      wickUpColor:'#26a69a',wickDownColor:'#ef5350',
    });
    this.volumeSeries = this.chart.addHistogramSeries({priceFormat:{type:'volume'},priceScaleId:'vol'});
    this.chart.priceScale('vol').applyOptions({scaleMargins:{top:0.82,bottom:0}});
    // NewChan overlay 状态角标 + Layer Registry
    this.newchan = { enabled: true, layers: { strokeLine:null, segmentLine:null, trendLine:null }, lastPayload: null };
    this.newchan.statusEl = document.createElement('div');
    this.newchan.statusEl.className = 'newchan-status';
    this.newchan.statusEl.textContent = 'NewChan: (not loaded)';
    this.mainWrap.style.position = 'relative';
    this.mainWrap.appendChild(this.newchan.statusEl);
    // 点击激活
    this.container.addEventListener('click',()=>this.activate());
  }

  activate(){
    if(activePanel) activePanel.container.style.outline='none';
    activePanel = this;
    this.container.style.outline='1px solid #2962FF';
  }

  resize(){
    this.chart.applyOptions({width:this.mainWrap.clientWidth,height:this.mainWrap.clientHeight});
    this.indicators.forEach(ind=>{
      if(ind.subChart) ind.subChart.applyOptions({width:ind.subEl.clientWidth,height:ind.subEl.clientHeight});
    });
    // NewChan MACD 子图 resize
    if(this.newchan?.macdChart&&this.newchan.macdWrap){
      this.newchan.macdChart.applyOptions({width:this.newchan.macdWrap.clientWidth,height:this.newchan.macdWrap.clientHeight});
    }
  }

  async setSymbol(symbol, interval='1min'){
    this.symbol=symbol; this.interval=interval;
    this.symLabel.textContent=symbol;
    await this.loadData();
  }

  async setTF(tf){
    this.tf=tf;
    this.tbEl.querySelectorAll('.tf-btn').forEach(b=>b.classList.toggle('active',b.dataset.tf===tf));
    await this.loadData();
  }

  async loadData(){
    if(!this.symbol) return;
    setStatus(`加载 ${this.symbol} ${this.tf}...`);
    const res = await api(`/api/ohlcv?symbol=${this.symbol}&interval=${this.interval}&tf=${this.tf}`);
    if(res.error){ setStatus(res.error); return; }
    const data = convertArr(res.data);
    this.candleSeries.setData(data);
    this.volumeSeries.setData(data.map(d=>({time:d.time,value:d.volume||0,color:d.close>=d.open?'rgba(38,166,154,0.3)':'rgba(239,83,80,0.3)'})));
    this.chart.timeScale().fitContent();
    // 刷新所有指标
    for(const ind of this.indicators) await this._loadIndicator(ind);
    // 刷新 NewChan overlay（中枢线 + L* 状态 + MACD 子图）
    this.refreshNewChanOverlay('full');
    setStatus(`${this.symbol} ${this.tf} — ${res.count} 条`);
    // 盘中定时刷新：每 60 秒重新加载 K线 + overlay（Live feeder 持续写缓存）
    if(this._liveTimer) clearInterval(this._liveTimer);
    this._liveTimer=setInterval(()=>this.loadData(), 60000);
  }

  async addIndicator(name, params={}){
    const def = indicatorDefs.find(d=>d.name===name);
    if(!def) return;
    const id = `ind_${++this._idCounter}`;
    const paramStr = Object.entries(params).map(([k,v])=>k+'='+v).join(',');
    const ind = {id, name, params, paramStr, def, series:[]};

    if(def.display==='subchart'){
      // 创建子图面板
      const wrap = document.createElement('div'); wrap.className='subchart-wrap';
      const label = document.createElement('span'); label.className='subchart-label'; label.textContent=name;
      const closeBtn = document.createElement('span'); closeBtn.className='subchart-close'; closeBtn.innerHTML='&times;';
      closeBtn.onclick=()=>this.removeIndicator(id);
      wrap.appendChild(label); wrap.appendChild(closeBtn);
      this.subContainer.appendChild(wrap);
      ind.subEl = wrap;
      ind.subChart = LightweightCharts.createChart(wrap,{
        layout:{background:{color:'#131722'},textColor:'#787b86'},
        grid:{vertLines:{color:'#1e222d'},horzLines:{color:'#1e222d'}},
        rightPriceScale:{borderColor:'#2a2e39'},
        timeScale:{borderColor:'#2a2e39',timeVisible:true,secondsVisible:false,visible:false},
      });
      // 同步时间轴
      this.chart.timeScale().subscribeVisibleLogicalRangeChange(range=>{
        if(range && ind.subChart) ind.subChart.timeScale().setVisibleLogicalRange(range);
      });
    }

    this.indicators.push(ind);
    await this._loadIndicator(ind);
    this.resize();
  }

  async _loadIndicator(ind){
    if(!this.symbol) return;
    const res = await api(`/api/indicator?symbol=${this.symbol}&interval=${this.interval}&tf=${this.tf}&name=${ind.name}&params=${ind.paramStr||''}`);
    if(res.error) return;
    const data = convertArr(res.data);
    // 清除旧 series
    ind.series.forEach(s=>{
      try{
        if(ind.def.display==='subchart'&&ind.subChart) ind.subChart.removeSeries(s);
        else this.chart.removeSeries(s);
      }catch(e){}
    });
    ind.series=[];
    // 创建新 series
    const targetChart = ind.def.display==='subchart' ? ind.subChart : this.chart;
    for(const sDef of ind.def.series){
      let s;
      if(sDef.type==='histogram'){
        s = targetChart.addHistogramSeries({priceLineVisible:false,priceScaleId:ind.id});
        const hData = data.map(d=>({time:d.time,value:d[sDef.key]||0,color:(d[sDef.key]||0)>=0?'rgba(38,166,154,0.6)':'rgba(239,83,80,0.6)'}));
        s.setData(hData);
      } else {
        s = targetChart.addLineSeries({color:sDef.color||'#888',lineWidth:1,priceLineVisible:false,priceScaleId:ind.def.display==='overlay'?'right':ind.id});
        s.setData(data.map(d=>({time:d.time,value:d[sDef.key]})).filter(d=>d.value!=null&&!isNaN(d.value)));
      }
      ind.series.push(s);
    }
  }

  removeIndicator(id){
    const idx = this.indicators.findIndex(i=>i.id===id);
    if(idx<0) return;
    const ind = this.indicators[idx];
    ind.series.forEach(s=>{
      try{
        if(ind.def.display==='subchart'&&ind.subChart) ind.subChart.removeSeries(s);
        else this.chart.removeSeries(s);
      }catch(e){}
    });
    if(ind.subEl){
      if(ind.subChart) ind.subChart.remove();
      ind.subEl.remove();
    }
    this.indicators.splice(idx,1);
    this.resize();
  }

  // ═══════════════════════════════════════════════════════════════════
  // NewChan overlay（A→B 桥接：中枢线 + L* 状态 + MACD 子图）
  // ═══════════════════════════════════════════════════════════════════

  async refreshNewChanOverlay(detail='full'){
    if(!this.newchan?.enabled) return;
    try{
      const payload = await this.fetchNewChanOverlay(detail);
      this.applyNewChanOverlay(payload);
    }catch(e){
      console.error('[newchan] overlay fetch failed',e);
      if(this.newchan?.statusEl) this.newchan.statusEl.textContent=`NewChan: ERROR\n${String(e).slice(0,120)}`;
    }
  }

  async fetchNewChanOverlay(detail='full'){
    const symbol=this.symbol;
    const tf=this.tf||'1d';
    const qs=new URLSearchParams({
      symbol:String(symbol), interval:this.interval||'1min',
      tf:String(tf), detail:detail,
      segment_algo:'v1', stroke_mode:'wide',
      min_strict_sep:'5', center_sustain_m:'2',
    });
    const r=await fetch(`/api/newchan/overlay?${qs.toString()}`,{method:'GET'});
    if(!r.ok) throw new Error(`HTTP ${r.status}`);
    return await r.json();
  }

  applyNewChanOverlay(payload){
    if(!payload||payload.schema_version!=='newchan_overlay_v1') throw new Error('bad schema_version');

    // --- 1) 状态文本 ---
    const lstar=payload.lstar;
    if(!lstar){
      this.newchan.statusEl.textContent='NewChan: L* = none';
    }else{
      const alive=lstar.is_alive?'ALIVE':'DEAD';
      const death=lstar.death_reason?`\n${lstar.death_reason}`:'';
      const anc=lstar.anchors;
      let ancTxt='';
      if(anc){
        const rs=anc.run_exit_side??'';
        const re=anc.run_exit_extreme??'';
        const pb=anc.event_seen_pullback?'PB':'';
        ancTxt=`\n${rs} ${re} ${pb}`.trimEnd();
      }
      this.newchan.statusEl.textContent=`NewChan: ${alive}\n${lstar.regime}${death}${ancTxt}`;
    }

    // --- 2) 主图：中枢核 ZG/ZD 线 ---
    this.ensureNewChanCenterLineSeries();
    const centers=Array.isArray(payload.centers)?payload.centers:[];
    const settled=centers.filter(c=>c.kind==='settled');
    const candidate=centers.filter(c=>c.kind==='candidate');
    const buildLD=(items,field)=>{
      const out=[];
      for(const c of items){
        if(!c.t0||!c.t1||c[field]==null) continue;
        out.push({time:c.t0,value:c[field]});
        out.push({time:c.t1,value:c[field]});
      }
      return out;
    };
    this.newchan.zgSettled.setData(buildLD(settled,'ZG'));
    this.newchan.zdSettled.setData(buildLD(settled,'ZD'));
    this.newchan.zgCand.setData(buildLD(candidate,'ZG'));
    this.newchan.zdCand.setData(buildLD(candidate,'ZD'));

    // --- 3) 子图：MACD hist ---
    this.ensureNewChanMacdSubchart();
    const series=payload.macd?.series||[];
    const histData=series.map(p=>({
      time:p.time, value:p.hist||0,
      color:(p.hist||0)>=0?'rgba(38,166,154,0.6)':'rgba(239,83,80,0.6)',
    }));
    this.newchan.macdHist.setData(histData);

    // --- 4) 笔/段/趋势折线（暂时禁用：lightweight-charts lineSeries 不接受重复时间戳）---
    this.newchan.lastPayload = payload;
    // TODO: 待升级 lightweight-charts 或改用 markers 后再启用
    // this.renderStrokeObjects(payload);
    // this.renderSegmentObjects(payload);
    // this.renderTrendObjects(payload);

    // --- 5) 子图创建后必须 resize 主图 ---
    setTimeout(()=>this.resize(), 50);
  }

  renderStrokeObjects(payload){
    if(!this.newchan.layers.strokeLine){
      this.newchan.layers.strokeLine=this.chart.addLineSeries({
        lineWidth:1, color:'rgba(80,160,255,0.65)',
        priceLineVisible:false, lastValueVisible:false, crosshairMarkerVisible:false,
      });
    }
    const strokes=payload.strokes||[];
    const data=[];
    for(const s of strokes){
      if(s.p0==null||s.p1==null) continue;
      data.push({time:s.t0,value:s.p0});
      data.push({time:s.t1,value:s.p1});
    }
    this.newchan.layers.strokeLine.setData(data);
  }

  renderSegmentObjects(payload){
    if(!this.newchan.layers.segmentLine){
      this.newchan.layers.segmentLine=this.chart.addLineSeries({
        lineWidth:2, color:'rgba(255,159,67,0.75)',
        priceLineVisible:false, lastValueVisible:false, crosshairMarkerVisible:false,
      });
    }
    const segs=payload.segments||[];
    const data=[];
    for(const s of segs){
      if(s.p0==null||s.p1==null) continue;
      data.push({time:s.t0,value:s.p0});
      data.push({time:s.t1,value:s.p1});
    }
    this.newchan.layers.segmentLine.setData(data);
  }

  renderTrendObjects(payload){
    if(!this.newchan.layers.trendLine){
      this.newchan.layers.trendLine=this.chart.addLineSeries({
        lineWidth:3, color:'rgba(180,90,255,0.65)',
        priceLineVisible:false, lastValueVisible:false, crosshairMarkerVisible:false,
      });
    }
    const trends=payload.trends||[];
    const data=[];
    for(const tr of trends){
      if(tr.p0==null||tr.p1==null) continue;
      data.push({time:tr.t0,value:tr.p0});
      data.push({time:tr.t1,value:tr.p1});
    }
    this.newchan.layers.trendLine.setData(data);
  }

  ensureNewChanCenterLineSeries(){
    if(this.newchan.zgSettled) return;
    const sc='rgba(255,159,67,0.85)', cc='rgba(255,159,67,0.25)';
    const opts={priceLineVisible:false,lastValueVisible:false,crosshairMarkerVisible:false};
    this.newchan.zgSettled=this.chart.addLineSeries({lineWidth:2,color:sc,...opts});
    this.newchan.zdSettled=this.chart.addLineSeries({lineWidth:2,color:sc,...opts});
    this.newchan.zgCand=this.chart.addLineSeries({lineWidth:1,color:cc,...opts});
    this.newchan.zdCand=this.chart.addLineSeries({lineWidth:1,color:cc,...opts});
  }

  ensureNewChanMacdSubchart(){
    if(this.newchan.macdChart) return;
    const wrap=document.createElement('div');
    wrap.className='subchart-wrap'; wrap.style.height='140px';
    const label=document.createElement('span'); label.className='subchart-label'; label.textContent='MACD (NewChan)';
    wrap.appendChild(label);
    this.subContainer.appendChild(wrap);
    this.newchan.macdWrap=wrap;
    this.newchan.macdChart=LightweightCharts.createChart(wrap,{
      layout:{background:{color:'#131722'},textColor:'#787b86'},
      grid:{vertLines:{color:'#1f2430'},horzLines:{color:'#1f2430'}},
      rightPriceScale:{borderColor:'#2a2e39'},
      timeScale:{borderColor:'#2a2e39',timeVisible:true,secondsVisible:false,visible:false},
      crosshair:{mode:0},
    });
    this.chart.timeScale().subscribeVisibleLogicalRangeChange(range=>{
      if(range&&this.newchan.macdChart) this.newchan.macdChart.timeScale().setVisibleLogicalRange(range);
    });
    this.newchan.macdHist=this.newchan.macdChart.addHistogramSeries({
      priceFormat:{type:'price',precision:4,minMove:0.0001}, base:0,
    });
  }
}

// =====================================================================
// 布局管理
// =====================================================================
const LAYOUTS = {
  '1x1':{cols:'1fr',rows:'1fr',count:1},
  '2x1':{cols:'1fr 1fr',rows:'1fr',count:2},
  '1x2':{cols:'1fr',rows:'1fr 1fr',count:2},
  '2x2':{cols:'1fr 1fr',rows:'1fr 1fr',count:4},
};

function setLayout(name){
  currentLayout=name;
  const cfg=LAYOUTS[name]; if(!cfg) return;
  const el=document.getElementById('layout');
  el.style.gridTemplateColumns=cfg.cols;
  el.style.gridTemplateRows=cfg.rows;
  // 保留已有面板的状态，增减格子
  while(panels.length<cfg.count){
    const div=document.createElement('div');
    el.appendChild(div);
    const p=new ChartPanel(div);
    panels.push(p);
    if(!activePanel) p.activate();
  }
  // 隐藏多余的面板
  panels.forEach((p,i)=>{
    p.container.style.display = i<cfg.count ? '' : 'none';
  });
  document.querySelectorAll('.layout-btn').forEach(b=>b.classList.toggle('active',b.dataset.layout===name));
  setTimeout(()=>panels.forEach(p=>p.resize()),50);
}

// =====================================================================
// 指标弹窗
// =====================================================================
function showIndicatorModal(){
  const list=document.getElementById('indicator-list');
  list.innerHTML='';
  indicatorDefs.forEach(def=>{
    const div=document.createElement('div'); div.className='ind-item';
    div.innerHTML=`<span>${def.name}</span><span class="badge">${def.display}</span>`;
    div.onclick=()=>{
      if(activePanel){
        const params={};
        def.params.forEach(p=>{params[p.name]=p.default});
        activePanel.addIndicator(def.name,params);
      }
      document.getElementById('indicator-modal').classList.remove('show');
    };
    list.appendChild(div);
  });
  document.getElementById('indicator-modal').classList.add('show');
}

// =====================================================================
// 合成品种弹窗
// =====================================================================
function showSyntheticModal(){
  document.getElementById('synth-status').textContent='';
  document.getElementById('synthetic-modal').classList.add('show');
}
async function createSynthetic(){
  const a=document.getElementById('synth-a').value.toUpperCase();
  const b=document.getElementById('synth-b').value.toUpperCase();
  const op=document.getElementById('synth-op').value;
  if(!a||!b){document.getElementById('synth-status').textContent='请填写品种';return;}
  document.getElementById('synth-status').textContent='正在生成...';
  const res=await api('/api/synthetic',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({a,b,op,interval:'1min'})});
  if(res.error){document.getElementById('synth-status').textContent=res.error;return;}
  document.getElementById('synth-status').textContent=`已生成 ${res.name}（${res.count} 条）`;
  await refreshSymbolList();
  document.getElementById('synthetic-modal').classList.remove('show');
  if(activePanel) activePanel.setSymbol(res.name,'1min');
}

// =====================================================================
// 连接状态
// =====================================================================
let ibConnected = false;

async function checkConnection(){
  try{
    const res=await api('/api/connection');
    ibConnected=res.connected;
    const el=document.getElementById('conn-status');
    el.className=ibConnected?'on':'off';
    el.title=ibConnected?'IBKR 已连接':'IBKR 未连接';
    document.getElementById('conn-btn').textContent=ibConnected?'已连接':'连接';
  }catch(e){ document.getElementById('conn-status').className='off'; }
}

async function reconnectIBKR(){
  setStatus('正在连接 IBKR...');
  try{
    const res=await api('/api/connection/connect',{method:'POST'});
    if(res.connected){ setStatus('IBKR 连接成功'); }
    else{ setStatus('IBKR 连接失败: '+(res.error||'')); }
  }catch(e){ setStatus('连接错误: '+e.message); }
  await checkConnection();
}

// =====================================================================
// 品种搜索（防抖下拉建议）
// =====================================================================
let searchTimer=null;
const symInput=document.getElementById('sym-input');
const dropdown=document.getElementById('search-dropdown');

symInput.addEventListener('input',()=>{
  clearTimeout(searchTimer);
  const q=symInput.value.trim();
  if(q.length<1){ dropdown.classList.remove('show'); return; }
  searchTimer=setTimeout(()=>doSearch(q), 400);
});

symInput.addEventListener('keydown',async e=>{
  if(e.key==='Enter'){
    e.preventDefault();
    dropdown.classList.remove('show');
    const sym=symInput.value.trim().toUpperCase();
    if(sym && activePanel) await loadSymbolSmart(sym);
  }
  if(e.key==='Escape') dropdown.classList.remove('show');
});

// 点击外部关闭下拉
document.addEventListener('click',e=>{
  if(!document.getElementById('search-wrap').contains(e.target)) dropdown.classList.remove('show');
});

async function doSearch(q){
  try{
    let results=await api(`/api/search?q=${encodeURIComponent(q)}`);
    // 兼容后端返回 {data:[...]} 或直接 [...]
    if(results && !Array.isArray(results) && results.data) results=results.data;
    if(!Array.isArray(results)||results.length===0){
      dropdown.innerHTML=`<div class="sr-item" style="color:${ibConnected?'#787b86':'#ef9a9a'}">${ibConnected?'未找到匹配品种':'IBKR 未连接，仅能匹配已缓存代码'}</div>`;
      dropdown.classList.add('show');
      return;
    }
    dropdown.innerHTML='';
    results.forEach(r=>{
      const div=document.createElement('div'); div.className='sr-item';
      const src=r.source==='cache'?'cache':'ibkr';
      const desc = r.secType==='CACHED' ? r.description : `${r.secType||''} ${r.currency||''}`.trim();
      div.innerHTML=`<span><span class="sr-sym">${r.symbol}</span> <span class="sr-desc">${desc}</span></span><span class="sr-badge ${src}">${src==='cache'?'缓存':'IB'} ${r.exchange||''}</span>`;
      div.onclick=async()=>{
        dropdown.classList.remove('show');
        symInput.value=r.symbol;
        if(activePanel) await loadSymbolSmart(r.symbol);
      };
      dropdown.appendChild(div);
    });
    dropdown.classList.add('show');
  }catch(e){
    dropdown.innerHTML='<div class="sr-item" style="color:#ef5350">搜索失败</div>';
    dropdown.classList.add('show');
  }
}

async function loadSymbolSmart(symbol){
  if(!activePanel) return;
  symbol=symbol.toUpperCase();
  setStatus(`加载 ${symbol}（从 IBKR 拉取最新）...`);
  // 后端自动从 IBKR 拉最新历史 -> 写缓存 -> 返回
  const res=await api(`/api/ohlcv?symbol=${symbol}&interval=1min&tf=${activePanel.tf}`);
  if(res.error){
    setStatus(`加载 ${symbol} 失败: ${res.error}`);
    return;
  }
  await activePanel.setSymbol(symbol,'1min');
  await refreshSymbolList();
  startRealtimePolling(activePanel);
}

// =====================================================================
// 品种列表（已缓存）
// =====================================================================
async function refreshSymbolList(){
  const syms=await api('/api/symbols');
  const sel=document.getElementById('sym-list');
  sel.innerHTML='<option value="">-- 已缓存 --</option>';
  syms.forEach(s=>{
    const opt=document.createElement('option');
    opt.value=JSON.stringify(s);
    opt.textContent=`${s.symbol} (${s.interval})`;
    sel.appendChild(opt);
  });
}

document.getElementById('sym-list').addEventListener('change',async e=>{
  if(!e.target.value||!activePanel) return;
  const s=JSON.parse(e.target.value);
  await activePanel.setSymbol(s.symbol, s.interval);
  startRealtimePolling(activePanel);
});

// =====================================================================
// 实时数据轮询
// =====================================================================
const rtPollers = new Map();  // panelId -> {timer, symbol, since}

function startRealtimePolling(panel){
  if(!panel||!panel.symbol) return;
  // 停掉旧的
  stopRealtimePolling(panel);
  const panelId = panels.indexOf(panel);

  // 订阅成功后再开始轮询，避免无效请求
  api('/api/subscribe',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({symbol:panel.symbol})})
    .then((res)=>{
      if(!res || !res.subscribed){
        setStatus(`实时订阅失败: ${(res&&res.error)?res.error:'未知错误'}`);
        return;
      }

      const state = {symbol:panel.symbol, since:0};
      const timer = setInterval(async()=>{
        if(panel.symbol!==state.symbol){ stopRealtimePolling(panel); return; }
        try{
          const res=await api(`/api/realtime?symbol=${state.symbol}&since=${state.since}`);
          if(res.bars && res.bars.length>0){
            state.since=res.next_since;
            for(const b of res.bars){
              // 将 5 秒 bar 时间对齐到当前显示周期（如 1 分钟）
              let rawTime=parseTime(b.time);
              const tfSecs=getTFSeconds(panel.tf);
              if(tfSecs>0) rawTime=Math.floor(rawTime/tfSecs)*tfSecs;
              const bar={time:rawTime,open:b.open,high:b.high,low:b.low,close:b.close};
              panel.candleSeries.update(bar);
              panel.volumeSeries.update({time:rawTime,value:b.volume||0,color:b.close>=b.open?'rgba(38,166,154,0.3)':'rgba(239,83,80,0.3)'});
            }
            // 自动滚动到最新
            panel.chart.timeScale().scrollToRealTime();
          }
        }catch(e){}
      }, 2000);

      rtPollers.set(panelId, {timer, symbol:state.symbol});
    })
    .catch(()=>{});
}

function stopRealtimePolling(panel){
  const panelId = panels.indexOf(panel);
  const old = rtPollers.get(panelId);
  if(old){
    clearInterval(old.timer);
    api('/api/unsubscribe',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({symbol:old.symbol})}).catch(()=>{});
    rtPollers.delete(panelId);
  }
}

// 布局按钮
document.querySelectorAll('.layout-btn').forEach(b=>b.addEventListener('click',()=>setLayout(b.dataset.layout)));

// =====================================================================
// 初始化
// =====================================================================
(async function init(){
  // 获取配置 + 连接状态
  const [inds, tfs] = await Promise.all([api('/api/indicators'), api('/api/timeframes'), checkConnection()]);
  indicatorDefs = inds;
  TF_LIST.push(...tfs);
  // 初始布局
  setLayout('1x1');
  await refreshSymbolList();
  // 如果有缓存数据，自动加载第一个
  const syms = await api('/api/symbols');
  if(syms.length>0 && activePanel){
    await activePanel.setSymbol(syms[0].symbol, syms[0].interval);
    startRealtimePolling(activePanel);
  }
  setStatus('就绪');
  // 定时刷新连接状态
  setInterval(checkConnection, 10000);
})();

// 响应式
window.addEventListener('resize',()=>panels.forEach(p=>p.resize()));
</script>
</body></html>"""
