// #1467/#1468: trusted host adapter. The model receives only packet JSON,
// never this adapter, its source arguments, evaluator, paths, or manifest.
import {featuresAt, cutoffAt} from '../../book_grid_features.mjs';
export const PROFILE = 'asof-consumer-packet/1';
const need = (ok, why) => { if (!ok) throw Error(why); };
const fields = (o, keys) => Object.fromEntries(keys.filter(k => Object.hasOwn(o, k)).map(k => [k, o[k]]));
const qkeys = ['bid_price','ask_price','bid_quantity','ask_quantity'];
const q = x => x === null || x === undefined ? null : fields(x,qkeys);
const fkeys = ['last_trade_price','seen_trade_count','since_last_trade_ns','observed_trade_count_1s','observed_trade_quantity_1s','bid_price','ask_price','bid_quantity','ask_quantity','spread','queue_imbalance','published_l1_changes_1s','published_mid2_delta_1s'];
const nkeys = ['application_index','type','reason','quote_status','native_sequence','native_type','native_event_at','captured_at','disposition','normalization','buy','price','signed_delta','level_before','level_after','before_status','after_status'];

// Inputs are trusted, validated Stage44 projections; not arbitrary hostile JSON.
// t and the observation watermark are host-side capabilities, not model requests.
export function packetAt(input, native, raw, t) {
  const {publication_end: end} = cutoffAt(input,t);
  const pubs = input.publications.slice(0,end);
  const receipts = raw.filter(x => x.source_line <= end && BigInt(x.capture_ns) < t);
  need(receipts.length === end && pubs.length === end, 'receipt/publication prefix mismatch');
  const applications = native.filter(x => x.available_line <= end && BigInt(x.available_capture_ns) < t);
  for(const a of applications) need(a.type === 'anchor' || (a.captured_line <= a.available_line && a.captured_line <= end), 'application references future receipt');
  const history = {
    receipts: receipts.map(x => ({event_seq:x.source_line, capture_ns:x.capture_ns,raw_message:x.raw_message})),
    publications: pubs.map(x => ({event_seq:x.source_line,capture_ns:x.capture_ns,quote:q(x.quote),barrier:x.barrier,status:x.status,transition:x.transition})),
    applications: applications.map(x => ({...fields(x,nkeys),available_event_seq:x.available_line,available_capture_ns:x.available_capture_ns,
      ...(x.captured_line === undefined ? {} : {captured_event_seq:x.captured_line}),
      ...(Object.hasOwn(x,'quote') ? {quote:q(x.quote)} : {}),
      ...(Object.hasOwn(x,'before_quote') ? {before_quote:q(x.before_quote)} : {}),
      ...(Object.hasOwn(x,'after_quote') ? {after_quote:q(x.after_quote)} : {})})),
    trades:input.trades.filter(x=>x.capture<t).map(x=>({capture_ns:String(x.capture),price:String(x.price),quantity:String(x.quantity)}))
  };
  // Only known origin and current prefix identity are exposed. No total rows,
  // final watermark, source SHA, future grid length, split or outcome fields.
  return JSON.stringify({profile:PROFILE,product_id:input.product,decision_ns:String(t),
    constants:{price_quantity_scale:100000000,grid_ns:'100000000',window_ns:'1000000000'},
    as_of:{exclusive_capture_ns:String(t),inclusive_event_seq:end},
    initialization:{history_origin_ns:String(input.origin),rule:'replay observed prefix from empty state; snapshots only when received; no prehistory',
      market_completeness:'not_established'},
    features:fields(featuresAt(input,t),fkeys),history});
}

// Evaluation-side only. resolved_at is not necessarily label knowledge time:
// a strictly later capture is needed to close the witness/deadline bucket.
export function partitionLabels(labels, publications, intervals) {
  need(intervals.length > 0,'empty partition');
  intervals.forEach((s,i)=>{
    need(BigInt(s.start_ns)<BigInt(s.end_ns),'invalid interval');
    if(i) need(intervals[i-1].end_ns===s.start_ns,'partitions must be contiguous');
  });
  const captures = [...new Set(publications.map(x=>x.capture_ns))].map(BigInt);
  need(captures.every((x,i)=>!i || captures[i-1]<x),'capture order');
  const firstAfter = t => {let lo=0,hi=captures.length;while(lo<hi){const m=(lo+hi)>>1;if(captures[m]<=t)lo=m+1;else hi=m;}return captures[lo]??null;};
  const out = labels.map(l=>{
    const t=BigInt(l.decision_ns), s=intervals.find(x=>BigInt(x.start_ns)<=t&&t<BigInt(x.end_ns));
    need(s,'decision outside partition');
    const observed=['observed_change','observed_no_change'].includes(l.disposition);
    need(!observed || (['up','down','none'].includes(l.label)&&l.resolved_at_ns!==null),'malformed observed label');
    const known=observed ? firstAfter(BigInt(l.resolved_at_ns)) : null;
    const loss=observed&&known!==null&&known<BigInt(s.end_ns);
    return {product_id:l.product_id,decision_ns:l.decision_ns,partition:s.name,label:l.label,
      disposition:l.disposition,label_event_ns:l.resolved_at_ns,label_known_ns:known===null?null:String(known),
      event_group:l.disposition==='observed_change'?`${l.product_id}:witness:${l.witness_line}`:null,
      loss_eligible:loss,exclusion:loss?null:observed?'cross_boundary_or_unsealed':l.disposition};
  });
  const groups = new Map();
  for(const x of out.filter(x=>x.loss_eligible&&x.event_group)) {
    if(groups.has(x.event_group)) need(groups.get(x.event_group)===x.partition,'shared future event split across losses');
    groups.set(x.event_group,x.partition);
  }
  return out;
}
