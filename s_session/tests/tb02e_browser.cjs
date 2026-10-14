// #1419 S9（根裁 D-E (B)）：既有真实观察页。只读取本地正式Q，不注入结构结果。
// 末切面点 CC-022（单元归属），AsKnown 第一个完成段切面再点 CC-022；(B) 前缀无中枢，只核归属与等待原因。
'use strict';
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const args = Object.fromEntries(process.argv.slice(2).reduce((a,v,i,x) => i%2 ? a : [...a,[v.slice(2),x[i+1]]], []));
// 与 tb02d_browser.cjs 的 committedCutReady 逐字相同：只观察原 Client 的已验证提交及正式渲染。
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
      executablePath:process.env.TB02E_CHROMIUM_EXECUTABLE || process.env.TB02D_CHROMIUM_EXECUTABLE || process.env.TB02B_CHROMIUM_EXECUTABLE});
    context = await browser.newContext({recordHar:{path:path.join(args.output,'network.har'),content:'embed'}});
    const page = await context.newPage(), errors = [];
    page.setDefaultTimeout(10000); page.setDefaultNavigationTimeout(10000);
    page.on('pageerror', e => { errors.push(String(e)); counts.pageerror++; record('pageerror'); });
    page.on('domcontentloaded', () => record('domcontentloaded'));
    page.on('load', () => record('load'));
    page.on('request', request => { counts.request++; record('request',{route:route(request)}); });
    page.on('response', response => { counts.response++; record('response',{route:route(response.request()),status:response.status()}); });
    page.on('requestfinished', request => { counts.finished++; record('finished',{route:route(request)}); });
    page.on('requestfailed', request => { counts.failed++; record('failed',{route:route(request)}); });
    record('navigation-start');
    const navigation = await page.goto(base.href, {waitUntil:'load', timeout:10000});
    assert.ok(navigation && navigation.status() === 200 && new URL(navigation.url()).origin === base.origin, '正式页面导航未完成');
    record('navigation-loaded');
    await page.waitForFunction(committedCutReady, {cut:args['final-cut'],mode:'RecomputedWithRevision'}, {timeout:10000});
    record('complete-final-cut-committed');
    await page.locator('[data-catalog-id="CC-022"]').click();
    await page.waitForFunction(() => document.querySelector('#view-selection').textContent.includes('CC-022'));
    const final = await page.locator('#objects').innerText();
    for (const s of ['NOT_YET_ASSIGNED', 'NO_LEFT_CENTER_IN_WINDOW', 'UNIT_NOT_COMPLETED', 'segment:1:1', 'segment:1:13', 'segment:1:25'])
      assert.ok(final.includes(s), '末切面 CC-022 未呈现 ' + s);
    assert.ok(!final.includes('CENTER_MEMBER') && !final.includes('CONNECTOR'), '(B) 前缀不应有中枢归属');
    fs.writeFileSync(path.join(args.output,'CC-022.txt'),final);
    await page.screenshot({path:path.join(args.output,'CC-022.png'),fullPage:false});
    await page.locator('#asof-input').fill(args['pending-cut']); await page.locator('#btn-asof').click();
    await page.waitForFunction(committedCutReady, {cut:args['pending-cut'],mode:'AsKnown'}, {timeout:10000});
    record('complete-pending-cut-committed');
    await page.locator('[data-catalog-id="CC-022"]').click();
    const early = await page.locator('#objects').innerText();
    for (const s of ['NO_LEFT_CENTER_IN_WINDOW', 'UNIT_NOT_COMPLETED', 'segment:1:1', 'segment:1:13'])
      assert.ok(early.includes(s), 'AsKnown 切面 CC-022 未呈现 ' + s);
    assert.ok(!early.includes('segment:1:25'), 'AsKnown 切面提前出现后来的单元');
    fs.writeFileSync(path.join(args.output,'CC-022-asof.txt'),early);
    await page.screenshot({path:path.join(args.output,'CC-022-asof.png'),fullPage:false});
    assert.deepEqual(errors,[]);
    fs.writeFileSync(path.join(args.output,'RESULT.json'),JSON.stringify({status:'passed',cuts:[args['pending-cut'],args['final-cut']],page_errors:errors,browser_version:browser.version()}));
  } catch (error) { failure = error; } finally {
    for (const resource of [context, browser]) {
      if (resource) { try { await resource.close(); } catch (error) { cleanupErrors.push(error); } }
    }
    try {
      const summary = JSON.stringify({schema:'tb1419-browser-metadata/1',events,counts,dropped,
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
