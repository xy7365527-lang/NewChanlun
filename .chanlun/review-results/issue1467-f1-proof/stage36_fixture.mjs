import fs from 'node:fs';
import assert from 'node:assert/strict';
const [input, output] = process.argv.slice(2);
const x = JSON.parse(fs.readFileSync(input, 'utf8'));
assert.equal(x.prices.length, 95);
assert.equal(x.events.length, 94);
assert.equal(x.strokes.length, 23);
const stroke = s => `⟨.${s.up ? 'up' : 'down'},${s.start},${s.end},${s.start_price},${s.end_price}⟩`;
const source = `import Origin.ChanlunElements
namespace ReferenceBase
open NewChanlun.Origin
def prices : List Int := [${x.prices.join(',')}]
def bidQuantities : List Int := [${x.bid_quantities.join(',')}]
def askQuantity : Int := ${x.ask_quantity}
def strokes : List Stroke := [${x.strokes.map(stroke).join(',\n')}]
end ReferenceBase
`;
fs.writeFileSync(output, source, {flag: 'wx'});
