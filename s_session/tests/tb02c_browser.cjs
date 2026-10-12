// #1404：既有真实观察页。只读取本地正式Q，不注入结构结果。
'use strict';
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const args = Object.fromEntries(process.argv.slice(2).reduce((a,v,i,x) => i%2 ? a : [...a,[v.slice(2),x[i+1]]], []));
async function main() {
  const base = new URL(args['base-url']); assert.equal(base.hostname, '127.0.0.1'); assert.equal(base.protocol, 'http:');
  const {chromium} = require(args['playwright-module']);
  fs.mkdirSync(args.output);
  const browser = await chromium.launch({headless:true,
    executablePath:process.env.TB02C_CHROMIUM_EXECUTABLE || process.env.TB02B_CHROMIUM_EXECUTABLE});
  const context = await browser.newContext({recordHar:{path:path.join(args.output,'network.har'),content:'embed'}});
  const page = await context.newPage(), errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  try {
    await page.goto(base.href, {waitUntil:'networkidle'});
    await page.waitForFunction(() => document.querySelector('#meta-kv').textContent.includes('cut-28'));
    for (const axis of ['CC-011','CC-012','CC-013']) {
      await page.locator(`[data-catalog-id="${axis}"]`).click();
      await page.waitForFunction(a => document.querySelector('#view-selection').textContent.includes(a), axis);
      const text = await page.locator('#objects').innerText();
      assert.ok(text.includes(axis), '新增段对象未被既有页面呈现');
      if (axis === 'CC-013') { assert.match(text,/CASE_ONE/); assert.match(text,/geometric_end/); assert.match(text,/terminated_known_at/); }
      fs.writeFileSync(path.join(args.output,axis+'.txt'),text);
    }
    await page.locator('#asof-input').fill('16'); await page.locator('#btn-asof').click();
    await page.waitForFunction(() => document.querySelector('#meta-kv').textContent.includes('cut-16'));
    await page.locator('[data-catalog-id="CC-013"]').click();
    const developing = await page.locator('#objects').innerText();
    assert.match(developing,/NO_FIRST/); assert.doesNotMatch(developing,/CASE_ONE/);
    fs.writeFileSync(path.join(args.output,'developing.txt'),developing);
    await page.screenshot({path:path.join(args.output,'developing.png'),fullPage:false});
    assert.deepEqual(errors,[]);
    fs.writeFileSync(path.join(args.output,'RESULT.json'),JSON.stringify({status:'passed',cuts:['16','28'],page_errors:errors,browser_version:browser.version()}));
  } finally { await context.close(); await browser.close(); }
}
main().catch(e=>{console.error(e);process.exitCode=1;});
