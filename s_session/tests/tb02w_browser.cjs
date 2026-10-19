// #1420（由 #1414 tb02d_browser.cjs 改）：既有真实观察页读第二种候选三态终点。只读本地正式Q，不注入结构结果。
'use strict';
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const args = Object.fromEntries(process.argv.slice(2).reduce((a,v,i,x) => i%2 ? a : [...a,[v.slice(2),x[i+1]]], []));
// 只观察原Client的已验证提交及正式渲染，不发请求、不注入状态。
function committedCutReady({cut, mode}) {
  if (typeof TB01C === 'undefined' || typeof v2Client === 'undefined' || !v2Client ||
      typeof currentState === 'undefined' || typeof v2Busy === 'undefined' || v2Busy) return false;
  const candidate = v2Client.committed;
  if (!candidate || currentState !== candidate.state) return false;
  const fixed = candidate.projection.fixed_cut, snapshot = currentState.snapshot;
  const target = 'cut-' + cut;
  if (fixed.cut_generation !== cut || fixed.structure_cut !== target || fixed.history_mode !== mode ||
      candidate.cursor.base_cut !== target || candidate.cursor.after_generation !== cut ||
      snapshot.structure_cut !== target || snapshot.generation !== cut || snapshot.history_mode !== mode) return false;
  if (mode === 'AsKnown' && snapshot.as_of_generation !== cut) return false;
  if (String(candidate.projection.rows.length) !== candidate.projection.counts.total ||
      candidate.pages !== Math.max(1, Math.ceil(candidate.projection.rows.length / v2Client.limits.pageSize))) return false;
  const meta = document.querySelector('#meta-kv'), cursor = document.querySelector('#stream-cursor');
  const result = document.querySelector('#stream-result .ok-box');
  return !!meta && Array.from(meta.querySelectorAll('dd')).some(node => node.textContent === target) &&
    !!cursor && cursor.textContent.includes('v2 已验证 ' + target + ' ·') &&
    !!result && result.textContent.includes('完整固定切面分页已验证') &&
    !document.querySelector('#status-section .err-box, #stream-result .err-box');
}
async function main() {
  const base = new URL(args['base-url']); assert.equal(base.hostname, '127.0.0.1'); assert.equal(base.protocol, 'http:');
  const {chromium} = require(args['playwright-module']);
  fs.mkdirSync(args.output);
  let browser, context, failure;
  const cleanupErrors = [];
  const began = performance.now(), events = [], counts = {request:0,response:0,finished:0,failed:0,pageerror:0};
  let dropped = 0;
  const record = (kind, fields = {}) => {
    if (events.length < 128) events.push({ms:Math.round(performance.now() - began),kind,...fields});
    else dropped++;
  };
  const route = request => {
    try {
      const url = new URL(request.url());
      return url.origin === base.origin && ['/', '/tb01c-client.js', '/api/discover', '/api/v2/snapshot', '/api/v2/watch', '/favicon.ico'].includes(url.pathname)
        ? url.pathname : '[other]';
    } catch (_) { return '[invalid-url]'; }
  };
  try {
  browser = await chromium.launch({headless:true,
    executablePath:process.env.TB02W_CHROMIUM_EXECUTABLE || process.env.TB02D_CHROMIUM_EXECUTABLE || process.env.TB02B_CHROMIUM_EXECUTABLE});
  context = await browser.newContext({recordHar:{path:path.join(args.output,'network.har'),content:'embed'}});
  const page = await context.newPage(), errors = [];
  page.setDefaultTimeout(60000); page.setDefaultNavigationTimeout(60000);  // #1420：WSL 无头壳渲染 40 代对象较慢，放宽到 60s（只是观察等待，不改 S/Q 资源）
  page.on('pageerror', e => { errors.push(String(e)); counts.pageerror++; record('pageerror'); });
  page.on('domcontentloaded', () => record('domcontentloaded'));
  page.on('load', () => record('load'));
  page.on('request', request => { counts.request++; record('request',{route:route(request)}); });
  page.on('response', response => { counts.response++; record('response',{route:route(response.request()),status:response.status()}); });
  page.on('requestfinished', request => { counts.finished++; record('finished',{route:route(request)}); });
  page.on('requestfailed', request => { counts.failed++; record('failed',{route:route(request)}); });
    record('navigation-start');
    const navigation = await page.goto(base.href, {waitUntil:'load', timeout:60000});
    assert.ok(navigation && navigation.status() === 200 && new URL(navigation.url()).origin === base.origin, '正式页面导航未完成');
    record('navigation-loaded');
    const killed = args.design === 'killed';
    const views = [
      {cut: args['final-cut'], mode: 'RecomputedWithRevision', label: 'final',
       want: {'CC-012': ['SECOND_SEQUENCE', killed ? 'KILLED_BY_NEW_EXTREME' : 'CONFIRMED', 'semantic_state'],
              'CC-013': killed ? ['KILLED_CASE_TWO', 'second_candidate_killed_by_new_extreme'] : ['CASE_TWO', 'geometric_end'], 'CC-014': ['fact_vector']}},
      {cut: args['killed-cut'], mode: 'AsKnown', label: 'decided',
       want: killed ? {'CC-012': ['KILLED_BY_NEW_EXTREME', 'killed_by'], 'CC-013': ['KILLED_CASE_TWO', 'second_candidate_killed_by_new_extreme']}
                    : {'CC-012': ['CONFIRMED', 'matched'], 'CC-013': ['CASE_TWO']}},
      {cut: args['open-cut'], mode: 'AsKnown', label: 'open',
       want: {'CC-012': ['OPEN_AT_PREFIX_END'], 'CC-013': ['PENDING_CASE_TWO']}},
    ];
    const seen = {};
    for (const view of views) {
      if (view.mode === 'AsKnown') { await page.locator('#asof-input').fill(view.cut); await page.locator('#btn-asof').click(); }
      await page.waitForFunction(committedCutReady, {cut:view.cut,mode:view.mode}, {timeout:60000});
      record('cut-committed-' + view.label);
      for (const [axis, needles] of Object.entries(view.want)) {
        await page.locator(`[data-catalog-id="${axis}"]`).click();
        await page.waitForFunction(a => document.querySelector('#view-selection').textContent.includes(a), axis);
        const text = await page.locator('#objects').innerText();
        for (const needle of needles) assert.ok(text.includes(needle), `${view.label} cut ${view.cut} ${axis} 页面缺 ${needle}`);
        if (view.label !== 'final') assert.ok(!(killed && view.label === 'decided' && /"CONFIRMED"|: CONFIRMED/.test(text) && axis === 'CC-012'), '失效 cut 上第二序列被显示为确认');
        const stem = view.label + '-' + axis;
        fs.writeFileSync(path.join(args.output, stem + '.txt'), text);
        await page.screenshot({path:path.join(args.output, stem + '.png'), fullPage:false});
        seen[stem] = needles;
      }
    }
    assert.deepEqual(errors,[]);
    fs.writeFileSync(path.join(args.output,'RESULT.json'),JSON.stringify({status:'passed',design:args.design,cuts:[args['open-cut'],args['killed-cut'],args['final-cut']],checked:seen,page_errors:errors,browser_version:browser.version()}));
  } catch (error) { failure = error; } finally {
    for (const resource of [context, browser]) {
      if (resource) { try { await resource.close(); } catch (error) { cleanupErrors.push(error); } }
    }
    // 不读payload/全DOM，不增加HTTP或等待；128事件与64KiB硬界只约束新诊断。
    try {
      const summary = JSON.stringify({schema:'tb1420-browser-metadata/1',events,counts,dropped,
        failure:!!failure,cleanup_errors:cleanupErrors.length,elapsed_ms:Math.round(performance.now()-began)});
      assert.ok(Buffer.byteLength(summary,'utf8') <= 65536, 'browser元数据超过64KiB界限');
      fs.writeFileSync(path.join(args.output,'READINESS-DIAGNOSTICS.json'),summary);
    } catch (error) { cleanupErrors.push(error); }
  }
  if (failure || cleanupErrors.length) {
    throw new AggregateError([...(failure ? [failure] : []), ...cleanupErrors], 'Browser validation or cleanup failed');
  }
}
main().catch(e=>{console.error(e);process.exitCode=1;});
