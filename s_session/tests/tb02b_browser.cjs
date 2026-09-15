#!/usr/bin/env node
// #1392：真实 Chromium + 正式页面/HTTP；不注入响应，不调用结构 reducer。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, all) => i % 2 ? a : [...a, [v.slice(2), all[i+1]]], []));
// 生命周期断言必须由计划显式选择：--lifecycle-asof <代际>。
// 不再用 generation>=12 隐式推定「当前有 CONFIRMED」——那是当前项目的运行痕迹，不是输入域事实。
async function main() {
  const base = new URL(args['base-url']);
  assert.equal(base.hostname, '127.0.0.1'); assert.equal(base.protocol, 'http:');
  const {chromium} = require(args['playwright-module']);
  fs.mkdirSync(args.output);
  const asof = args['lifecycle-asof'];
  if (asof !== undefined) assert.match(asof, /^(0|[1-9][0-9]{0,18})$/, 'lifecycle-asof 必须是规范十进制代际');
  const executablePath = process.env.TB02B_CHROMIUM_EXECUTABLE || chromium.executablePath();
  const browser = await chromium.launch({headless:true, executablePath});
  const context = await browser.newContext({recordHar:{path:path.join(args.output,'network.har'),content:'embed'}});
  const page = await context.newPage();
  const errors=[]; page.on('pageerror', error=>errors.push(String(error)));
  try {
    await page.goto(base.href, {waitUntil:'networkidle'});
    await page.waitForFunction(gen=>document.querySelector('#meta-kv').textContent.includes('cut-'+gen), args.generation);
    for (const axis of ['CC-008','CC-009','CC-010','CC-055','CC-056']) {
      const button=page.locator('[data-catalog-id="'+axis+'"]');
      assert.equal(await button.count(),1); await button.click();
      await page.waitForFunction(axis=>document.querySelector('#view-selection').textContent.includes(axis),axis);
      const sources=page.locator('#objects details > summary').filter({hasText:'完整来源引用与原始坐标'});
      if (await sources.count()) { await sources.first().click(); assert.ok((await page.locator('#objects').innerText()).includes('receipt_id')); }
      fs.writeFileSync(path.join(args.output,axis+'.txt'),await page.locator('#objects').innerText());
    }
    let lifecycle = null;
    if (asof !== undefined) {
      assert.notEqual(String(asof), String(args.generation), 'lifecycle-asof 不能等于当前 cut');
      await page.locator('#asof-input').fill(asof); await page.locator('#btn-asof').click();
      await page.waitForFunction(gen=>document.querySelector('#meta-kv').textContent.includes('cut-'+gen), asof);
      await page.locator('[data-catalog-id="CC-010"]').click();
      assert.match(await page.locator('#objects').innerText(), /\bFORMED_UNCONFIRMED\b/);
      await page.screenshot({path:path.join(args.output,'formed.png'),fullPage:false});
      await context.setOffline(true); await page.locator('#btn-current').click();
      await page.waitForTimeout(800); // 固定断连窗口，仅属故障调度，不参与语义比较。
      await context.setOffline(false); await page.locator('#btn-current').click();
      await page.waitForFunction(gen=>document.querySelector('#meta-kv').textContent.includes('cut-'+gen),args.generation);
      await page.locator('[data-catalog-id="CC-010"]').click();
      assert.match(await page.locator('#objects').innerText(), /\bCONFIRMED\b/);
      lifecycle = {asof_generation:asof,current_generation:String(args.generation),
        asserted:['FORMED_UNCONFIRMED@asof','CONFIRMED@current']};
    }
    // 完整事实已保存为文本/HAR；长页截图会申请数十万像素高的位图并使 Chromium 崩溃。
    await page.screenshot({path:path.join(args.output,'final.png'),fullPage:false});
    assert.deepEqual(errors,[]);
    fs.writeFileSync(path.join(args.output,'RESULT.json'),JSON.stringify({status:'passed',page_errors:errors,base_url:base.href,generation:args.generation,lifecycle_assertion:lifecycle,screenshot_scope:'viewport',executable_path:executablePath,browser_version:browser.version()}));
  } finally { await context.close(); await browser.close(); }
}
main().catch(error=>{console.error(error);process.exitCode=1;});
