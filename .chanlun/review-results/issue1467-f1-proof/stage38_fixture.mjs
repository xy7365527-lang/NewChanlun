import fs from 'node:fs';
import assert from 'node:assert/strict';
const [input,output] = process.argv.slice(2);
const x = JSON.parse(fs.readFileSync(input,'utf8'));
assert.equal(x.prices.length,283);
assert.equal(x.strokes.length,70);
const stroke = s => `⟨.${s.up?'up':'down'},${s.start},${s.end},${s.start_price},${s.end_price}⟩`;
fs.writeFileSync(output,`import Origin.ChanlunElements
namespace SegmentLiftReach
open NewChanlun.Origin
def prices : List Int := [${x.prices.join(',')}]
def bidQuantities : List Int := [${x.bid_quantities.join(',')}]
def macroPrices : List Int := [${x.macro_prices.join(',')}]
def strokes : List Stroke := [${x.strokes.map(stroke).join(',\n')}]
end SegmentLiftReach
`,{flag:'wx'});
