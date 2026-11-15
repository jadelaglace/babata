import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { research, resolveResearchRun } from './research.mjs';
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

test('workspace resolution is explicit, cwd-independent and does not mutate configuration', () => {
 const root=temp(),workspace=path.join(root,'中文 空格'),assistant=path.join(root,'assistant');
 const env={BABATA_ASSISTANT_HOME:assistant};
 assert.equal(resolveResearchRun({'workspace':workspace,'run-id':'topic-v1'},env),path.join(workspace,'topic-v1'));
 assert.equal(resolveResearchRun({'run-id':'topic-v1'},env),path.join(assistant,'solutions','research','workspaces','topic-v1'));
 assert.equal(resolveResearchRun({run:path.join(root,'exact')},{BABATA_ASSISTANT_HOME:'invalid'}),path.join(root,'exact'));
 assert.deepEqual(env,{BABATA_ASSISTANT_HOME:assistant});
 assert.equal(fs.existsSync(assistant),false);
 for (const id of ['../escape','..','a/b','a\\b','', '中文','CON','nul','COM1','LPT9','x'.repeat(81)])
   assert.throws(()=>resolveResearchRun({'workspace':workspace,'run-id':id},{}),/portable ID/);
 for (const options of [{run:root,workspace}, {run:root,'run-id':'x'}])
   assert.throws(()=>resolveResearchRun(options,{}),/cannot be combined/);
 assert.throws(()=>resolveResearchRun({'run-id':'x'},{}),/Assistant default unavailable/);
 assert.throws(()=>resolveResearchRun({'run-id':'x'},{BABATA_ASSISTANT_HOME:'relative'}),/Assistant default unavailable/);
 assert.throws(()=>resolveResearchRun({workspace:'relative','run-id':'x'},env),/absolute/);
 assert.throws(()=>resolveResearchRun({run:'relative'},env),/absolute/);
});

test('invalid command/options and read-only missing runs have no directory side effects', () => {
 const root=temp(),run=path.join(root,'must-not-exist');
 for (const [command,options,pattern] of [
  ['bad',{run},/unknown research command/],
  ['status',{run,workspce:root},/unknown option/],
  ['init',{run},/request required/],
  ['status',{run},/does not exist/],
  ['verify',{run},/does not exist/]
 ]) {assert.throws(()=>research(command,options),pattern);assert.equal(fs.existsSync(run),false);}
});

test('workspace outputs reject Git roots, linked worktrees and junctions into Git', () => {
 const root=temp();
 for (const linked of [false,true]) {
  const repo=path.join(root,linked?'linked':'repo'); fs.mkdirSync(repo);
  if(linked) fs.writeFileSync(path.join(repo,'.git'),'gitdir: elsewhere');
  else fs.mkdirSync(path.join(repo,'.git'));
  const options={workspace:path.join(repo,'data'),'run-id':'test',request:write(root,`request-${linked}.json`,request)};
  assert.throws(()=>research('init',options),/outside Git/);
  assert.equal(fs.existsSync(options.workspace),false);
  const junction=path.join(root,linked?'linked-junction':'junction');
  fs.symlinkSync(repo,junction,process.platform==='win32'?'junction':'dir');
  assert.throws(()=>research('init',{...options,workspace:path.join(junction,'data')}),/outside Git/);
  assert.equal(fs.existsSync(options.workspace),false);
 }
});

test('detached release runs complete lifecycle through custom/default/exact paths from another project', () => {
 const root=temp(),project=path.join(root,'other-project'),outside=path.join(root,'another cwd');
 fs.mkdirSync(project);fs.mkdirSync(path.join(project,'.git'));fs.mkdirSync(outside);
 const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
 const skill=path.join(root,'独立 技能','babata-research');
 fs.cpSync(path.join(repo,'02_skills','babata-research'),skill,{recursive:true});
 const script=path.join(skill,'scripts','research.mjs');
 const env=Object.fromEntries(Object.entries(process.env).filter(([k])=>!k.startsWith('BABATA_')&&!['NODE_PATH','NODE_OPTIONS'].includes(k)));
 const invoke=(command,options,cwd=project,config=env)=>spawnSync(process.execPath,[script,command,...Object.entries(options).flatMap(([k,v])=>[`--${k}`,v])],{cwd,env:config,encoding:'utf8',timeout:10000});
 const ok=(command,options,cwd,config)=>{const result=invoke(command,options,cwd,config);assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);};
 const req=write(root,'request.json',request),event=write(root,'body.json',body);
 const conclusion=write(root,'conclusion.json',{producer:'test-host',markdown:'# 独立测试\n\n仅路径 fixture，不冒充网络获取。',limitations:['fixture only'],claims:[{text:'fixture',kind:'evidence',sources:['B1']}]});
 const assistant=path.join(root,'默认 Assistant');
 const modes=[
  {location:{workspace:path.join(root,'中文 工作区'),'run-id':'custom-v1'},env},
  {location:{'run-id':'default-v1'},env:{...env,BABATA_ASSISTANT_HOME:assistant}},
  {location:{run:path.join(root,'精确 run')},env:{...env,BABATA_ASSISTANT_HOME:'invalid'}}
 ];
 const requestHashes=[];
 for (const mode of modes) {
  const init=ok('init',{...mode.location,request:req},project,mode.env);
  assert.equal(init.state,'working');assert.equal(path.isAbsolute(init.run_directory),true);
  assert.equal(init.run_directory,fs.realpathSync(resolveResearchRun(mode.location,mode.env)));
  requestHashes.push(init.request_sha256);
  assert.equal(ok('init',{run:init.run_directory,request:req},outside,env).replayed,true);
  ok('record',{run:init.run_directory,event},outside,env);
  assert.equal(ok('record',{...mode.location,event},project,mode.env).replayed,true);
  assert.equal(ok('finish',{...mode.location,conclusion},project,mode.env).state,'finalized');
  const status=ok('verify',{run:init.run_directory},outside,env);
  assert.equal(status.integrity,'verified');assert.equal(status.events.length,1);
  assert.equal(fs.readFileSync(path.join(init.run_directory,'report.md'),'utf8'),JSON.parse(fs.readFileSync(conclusion)).markdown);
  fs.appendFileSync(path.join(init.run_directory,'report.md'),'changed');
  const damaged=invoke('verify',{...mode.location},project,mode.env);
  assert.equal(damaged.status,1);assert.match(damaged.stderr,/hash mismatch/);
 }
 assert.equal(new Set(requestHashes).size,1,'storage choice must not alter request identity');
 const missing=invoke('init',{'run-id':'missing',request:req});
 assert.equal(missing.status,1);assert.match(missing.stderr,/Assistant default unavailable/);
 const conflict=invoke('status',{run:root,workspace:root});
 assert.equal(conflict.status,1);assert.match(conflict.stderr,/cannot be combined/);
 assert.equal(fs.existsSync(path.join(skill,'manifest.json')),false);
 const help=spawnSync(process.execPath,[script,'--help'],{cwd:project,env,encoding:'utf8'});
 assert.equal(help.status,0);assert.match(help.stdout,/--workspace/);
});
