import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { retireLegacyShadow } from './decision-claimer.mts';

test('旧影子迁移交错：另一个处理者不得迁走新活动目录或其锁', () => {
  const root=fs.mkdtempSync(join(tmpdir(),'shadow-race-')), dir=join(root,'9');
  fs.mkdirSync(dir); fs.writeFileSync(join(dir,'summary.json'),JSON.stringify({phase:'shadow_p'}));
  const original=fs.existsSync;
  let nested=false, inner: string|null=null;
  // B 已读旧状态并检查旧锁；A 在 B 使用检查结果前完成退役并开始处理。
  fs.existsSync=((path: fs.PathLike) => {
    const result=original(path);
    if(String(path)===join(dir,'.lock') && !nested){
      nested=true;
      inner=retireLegacyShadow(root,9);
      if(inner){fs.mkdirSync(dir);fs.writeFileSync(join(dir,'.lock'),'active-A');}
    }
    return result;
  }) as typeof fs.existsSync;
  syncBuiltinESMExports();
  try {
    const outer=retireLegacyShadow(root,9);
    assert.equal([inner,outer].filter(Boolean).length,1,'同票交错只能有一个迁移 owner');
    if(inner) assert.equal(fs.readFileSync(join(dir,'.lock'),'utf8'),'active-A','活动锁不可被另一迁移者搬走');
    assert.equal(original(`${dir}.shadow-p-retired-2`),false);
  } finally {fs.existsSync=original;syncBuiltinESMExports();fs.rmSync(root,{recursive:true,force:true});}
});
