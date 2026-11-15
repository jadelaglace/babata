import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { research } from './research.mjs';
import { jsonBytes, sha } from './common.mjs';

const temp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'babata-research-'));
const write = (root, name, data) => {const file=path.join(root,name);fs.writeFileSync(file,jsonBytes(data));return file;};
const request = { question:'如何查证官方技术资料？', scope:'public web, bounded test',producer:'test-host',depth:'research' };
const body = {id:'B1',kind:'body',provider:'test',observed_at:'2026-11-14T00:00:00Z',status:'succeeded',url:'https://example.com/doc',coverage:'relevant-excerpt',body:'Explicit fixture text, not real acquisition.'};

test('manifest omissions, mismatched event identity and changed conclusion references reject', () => {
 const root=temp(),run=path.join(root,'run');
 research('init',{run,request:write(root,'request.json',request)});
 research('record',{run,event:write(root,'event.json',body)});
 const file=path.join(run,'manifest.json'),good=fs.readFileSync(file);
 const omitted=JSON.parse(good);omitted.files=omitted.files.filter(f=>f.role!=='observation');fs.writeFileSync(file,jsonBytes(omitted));
 assert.throws(()=>research('verify',{run}),/completeness/);fs.writeFileSync(file,good);
 research('finish',{run,conclusion:write(root,'conclusion.json',{producer:'test',markdown:'# Test',limitations:[],claims:[{text:'test',kind:'evidence',sources:['B1']}]})});
 const conclusion=JSON.parse(fs.readFileSync(path.join(run,'conclusion.json')));conclusion.claims[0].sources=['unknown'];
 const bytes=jsonBytes(conclusion);fs.writeFileSync(path.join(run,'conclusion.json'),bytes);
 const forged=JSON.parse(fs.readFileSync(file));forged.files.find(f=>f.role==='conclusion').sha256=sha(bytes);fs.writeFileSync(file,jsonBytes(forged));
 assert.throws(()=>research('verify',{run}),/acquired body/);
});

test('research replays request and event, finalizes with evidence, detects source tampering', () => {
 const root=temp(),run=path.join(root,'run');
 const req=write(root,'request.json',request);
 assert.equal(research('init',{run,request:req}).state,'working');
 assert.equal(research('init',{run,request:req}).replayed,true);
 assert.throws(()=>research('init',{run,request:write(root,'other.json',{...request,question:'different'})}),/different request/);
 const event=write(root,'event.json',body);
 assert.equal(research('record',{run,event}).replayed,false);
 assert.equal(research('record',{run,event}).replayed,true);
 assert.throws(()=>research('record',{run,event:write(root,'conflict.json',{...body,body:'changed'})}),/conflicting replay/);
 const conclusion=write(root,'conclusion.json',{producer:'test-host',markdown:'# 结论：有证据的测试结论。',limitations:['fixture only'],claims:[{text:'fixture claim',kind:'evidence',sources:['B1']}]});
 assert.equal(research('finish',{run,conclusion}).state,'finalized');
 assert.equal(research('finish',{run,conclusion}).state,'finalized');
 assert.equal(research('verify',{run}).integrity,'verified');
 assert.throws(()=>research('record',{run,event:write(root,'new.json',{...body,id:'B2'})}),/finalized/);
 fs.writeFileSync(path.join(run,'event-B1.json'),'tampered');
 assert.throws(()=>research('verify',{run}),/hash mismatch/);
});

test('unreadable bodies and unsupported claims cannot certify research', () => {
 const root=temp(),run=path.join(root,'run');
 research('init',{run,request:write(root,'request.json',request)});
 assert.throws(()=>research('record',{run,event:write(root,'fake.json',{...body,status:'inaccessible',coverage:'none',reason:'login required'})}),/cannot claim/);
 research('record',{run,event:write(root,'failed.json',{...body,status:'inaccessible',coverage:'none',body:'',reason:'login required'})});
 assert.throws(()=>research('finish',{run,conclusion:write(root,'conclusion.json',{producer:'test',markdown:'unsupported',limitations:[],claims:[{text:'unsupported',kind:'evidence',sources:['B1']}]})}),/acquired body/);
 assert.equal(research('status',{run}).state,'working');
});

test('interrupted event write is reconciled by exact bytes; active lock and path escapes reject',()=>{
 const root=temp(),run=path.join(root,'run');
 research('init',{run,request:write(root,'request.json',request)});
 fs.writeFileSync(path.join(run,'event-B1.json'),jsonBytes(Object.fromEntries(Object.entries(body).sort(([a],[b])=>a.localeCompare(b)))));
 research('record',{run,event:write(root,'event.json',body)});
 fs.writeFileSync(path.join(run,'.operation.lock'),'active');
 assert.throws(()=>research('status',{run}),/EEXIST/);
 fs.unlinkSync(path.join(run,'.operation.lock'));
 const manifest=JSON.parse(fs.readFileSync(path.join(run,'manifest.json')));
 manifest.files.push({path:'../request.json',sha256:'0'.repeat(64),role:'observation'});
 fs.writeFileSync(path.join(run,'manifest.json'),jsonBytes(manifest));
 assert.throws(()=>research('verify',{run}),/escaped/);
});
