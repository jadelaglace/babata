import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { render } from './render.mjs';

const temp=()=>fs.mkdtempSync(path.join(os.tmpdir(),'babata-render-'));
const input=(root,text,name='input.md')=>{const file=path.join(root,name);fs.writeFileSync(file,text);return file;};

test('real browser: offline Markdown renders, replays, verifies detached bundle and catches tampering',async()=>{
 const root=temp(),source=input(root,['# 中文渲染验证','','已有内容，不调用搜索。','','| 项目 | 状态 |','|---|---|','| 独立输出 | 可复用 |', ...Array.from({length:25},(_,i)=>`\n长内容第 ${i+1} 段：超过视窗不等于正文被裁切。`)].join(String.fromCharCode(10)));
 const out=path.join(root,'out');
 const first=await render('render',{input:source,out});
 assert.equal(first.outputs[0].width,1120);assert.ok(first.outputs[0].height>900);
 assert.equal((await render('render',{input:source,out})).replayed,true);
 fs.unlinkSync(source);
 assert.equal((await render('verify',{out})).integrity,'verified');
 fs.appendFileSync(path.join(out,'output.png'),'tamper');
 await assert.rejects(render('verify',{out}),/hash mismatch/);
});

test('real browser: explicit pages retain semantic boundaries',async()=>{
 const root=temp();const source=input(root,'<section class="page"><h1>第一页</h1><p>完整内容。</p></section><section class="page"><h2>第二页</h2><table><tr><td>行保持完整。</td></tr></table></section>','input.html');
 const result=await render('render',{input:source,out:path.join(root,'out'),mode:'paged'});
 assert.deepEqual(result.outputs.map(x=>x.path),['page-001.png','page-002.png']);
 assert.ok(result.outputs.every(x=>x.width>0&&x.height>0));
});

test('real browser: local assets are frozen and changed asset replay rejects',async()=>{
 const root=temp();const asset=path.join(root,'pixel.png');
 fs.writeFileSync(asset,Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aD1sAAAAASUVORK5CYII=','base64'));
 const source=input(root,['# 资产','','![本地图片](pixel.png)'].join(String.fromCharCode(10)));const out=path.join(root,'out');
 const result=await render('render',{input:source,out});assert.equal(result.source_assets.length,1);
 const relocated=path.join(root,'relocated');fs.mkdirSync(relocated);const relocatedInput=input(relocated,fs.readFileSync(source,'utf8'));
 await assert.rejects(render('render',{input:relocatedInput,out}),/different input location/);
 fs.appendFileSync(asset,'changed');
 await assert.rejects(render('render',{input:source,out}),/asset changed/);
 assert.equal((await render('verify',{out})).integrity,'verified');
});

for(const [name,content,mode] of [
 ['missing image','<h1>缺图</h1><img src="missing.png">','long'],
 ['remote resource','<h1>Remote</h1><img src="https://example.com/not-allowed.png">','long'],
 ['script','<h1>Unsafe</h1><script>window.bad=true</script>','long'],
 ['overflow','<h1>Overflow</h1><table style="width:5000px"><tr><td>Cannot clip</td></tr></table>','long'],
 ['hidden text','<h1>Visible</h1><p style="display:none">Must not silently disappear</p>','long'],
 ['hidden root','<style>body{opacity:0}</style><h1>Cannot disappear</h1>','long'],
 ['clipped text','<h1>Visible</h1><div style="height:1px;overflow:hidden">Must not be clipped</div>','long'],
 ['positioned text','<h1>Visible</h1><p style="position:fixed;top:99999px">Cannot omit</p>','long'],
 ['unstructured pagination','<h1>No explicit page</h1>','paged']
])test('real browser rejects '+name,async()=>{
 const root=temp();const source=input(root,content,'input.html');const out=path.join(root,'out');
 const reason = {
  'missing image': /asset.render errors|missing images/,
  'remote resource': /asset.render errors|missing images/,
  'script': /unsafe.unsupported HTML/,
  'overflow': /content overflow/,
  'hidden text': /hidden.positioned/,
  'hidden root': /hidden.positioned/,
  'clipped text': /content overflow/,
  'positioned text': /hidden.positioned|content overflow/,
  'unstructured pagination': /explicit main-level/
 }[name];
 await assert.rejects(render('render',{input:source,out,mode}),reason);assert.equal(fs.existsSync(path.join(out,'manifest.json')),false);
});

test('unsafe output root and unrelated existing files reject without overwrite',async()=>{
 const root=temp();const source=input(root,'# safe');const out=path.join(root,'out');fs.mkdirSync(out);fs.writeFileSync(path.join(out,'keep.txt'),'preserve');
 await assert.rejects(render('render',{input:source,out}),/unrelated/);
 assert.equal(fs.readFileSync(path.join(out,'keep.txt'),'utf8'),'preserve');
 await assert.rejects(render('render',{input:source,out:path.resolve('tmp-render-should-not-exist')}),/outside Git/);
});
