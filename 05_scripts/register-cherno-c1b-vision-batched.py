import hashlib, json, os, subprocess, sys, time
from pathlib import Path

STAGE=Path(r'D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-full-20260822-v1')
DATA_HOME=Path(os.environ.get('BABATA_DATA_HOME',r'D:\BabataData'))
BABATA=Path(r'C:\Users\Aiano\Babata\01_app\target\debug\babata.exe')
MANIFEST=STAGE/'manifest.json'; LEDGER=STAGE/'c1b-registration-ledger.json'

def sha(p):
    h=hashlib.sha256()
    with open(p,'rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''): h.update(b)
    return h.hexdigest()
def call(args):
    r=subprocess.run([str(BABATA),'--json']+args,capture_output=True,text=True,timeout=120)
    if r.returncode: raise RuntimeError(f'babata failed: {args}\n{r.stdout}\n{r.stderr}')
    return json.loads(r.stdout)
def verify(run_id,item,kind,provider,model,tool,out_sha):
    d=call(['process','show-run','--run',run_id]); run=d['run']; ds=d.get('derivatives',[])
    if run.get('state')!='succeeded' or run.get('pipeline_id')!='agent_import' or run.get('target_kind')!=kind or run.get('provider')!=provider or run.get('tool_or_model')!=model or ds[0].get('output_sha256')!=out_sha:
        raise RuntimeError(f'identity mismatch {run_id}')
    lp=ds[0]['logical_path']; mp=DATA_HOME/lp.replace('/',os.sep)
    if sha(mp)!=out_sha: raise RuntimeError(f'managed hash mismatch {run_id}')
    return {'run_id':run_id,'derivative_id':ds[0]['id'],'logical_path':lp,'output_sha256':out_sha,'kind':kind}
def register(item,kind,provider,model,tool,out_file,params,usage='{}',loss=''):
    return call(['process','register','--pipeline','agent_import','--revision',item['c0']['revision_id'],'--item',item['c0']['item_id'],'--kind',kind,'--provider',provider,'--model',model,'--tool-version',tool,'--input-sha256',item['c0']['input_sha256'],'--input-asset-id',item['c0']['asset_id'],'--output-file',str(out_file),'--params-json',json.dumps(params,separators=(',',':')),'--usage-json',usage,'--loss-notes',loss])['run_id']
def main():
    manifest=json.loads(MANIFEST.read_text(encoding='utf-8'))
    ledger=json.loads(LEDGER.read_text(encoding='utf-8')) if LEDGER.exists() else {'schema':'babata.cherno-course-c1b-registration/v1','status':'in_progress','scope':'full_269','items':[]}
    done={x['video_id']:x for x in ledger.get('items',[])}
    ffmpeg=subprocess.run(['ffmpeg','-version'],capture_output=True,text=True).stdout.splitlines()[0].strip()
    for item in manifest['items']:
        vid=item['video_id']
        if vid in done: continue
        try:
            runs=call(['process','list-runs','--revision',item['c0']['revision_id']])
            dec=item['essence_decision']; dp=STAGE/dec['path']; dparams={'service':'dashscope','adapter':'qianwen-vision','credential_source':'environment','provider_input_sha256':item['processing']['provider_input_sha256'],'video_input_sha256':item['processing']['video_input_sha256'],'transcript_sha256':item['processing']['transcript_sha256'],'fps':item['processing']['fps'],'pricing':{'estimated_cost_cny':item['processing']['estimated_cost_cny'],'free_tier_assumed':False}}
            dr=None
            for x in runs:
                if x.get('target_kind')=='structured_result' and x.get('provider')=='qianwen_skill' and x.get('state')=='succeeded':
                    q=call(['process','show-run','--run',x['id']]);
                    if q.get('derivatives',[{}])[0].get('output_sha256')==dec['sha256']: dr=x['id']; break
            if not dr: dr=register(item,'structured_result','qianwen_skill',item['processing']['model'],item['processing']['tool_version'],dp,dparams,json.dumps(item['processing']['usage'],separators=(',',':')),' '.join(dec.get('limitations',[])))
            regs=[verify(dr,item,'structured_result','qianwen_skill',item['processing']['model'],item['processing']['tool_version'],dec['sha256'])]
            for media in item.get('retained_derivatives',[]):
                mp=STAGE/media['path']; params={'provider_input_sha256':item['c0']['input_sha256'],'preprocessing':['ffmpeg extraction from the complete read-only C0 video'],'source_locator':media['source_locator'],'original_model_source_locator':media['original_model_source_locator'],'role':media['role'],'essence_decision_sha256':dec['sha256']}
                rid=register(item,media['kind'],'local_extract','ffmpeg',ffmpeg,mp,params,'{}',' '.join(media.get('loss_notes',[])))
                regs.append(verify(rid,item,media['kind'],'local_extract','ffmpeg',ffmpeg,media['sha256']))
            done[vid]={'video_id':vid,'course_slug':item['course_slug'],'source_item_id':item['c0']['item_id'],'source_revision_id':item['c0']['revision_id'],'source_asset_id':item['c0']['asset_id'],'source_asset_sha256':item['c0']['input_sha256'],'complete_c1':item['complete_c1'],'registrations':regs,'status':'registered'}
            ledger['items']=list(done.values()); ledger['coverage']={'items':len(done)}; LEDGER.write_text(json.dumps(ledger,ensure_ascii=False,indent=2),encoding='utf-8'); print(vid,len(done),flush=True)
        except Exception as e:
            print(f'FAILED {vid}: {e}',file=sys.stderr,flush=True); continue
    if len(done)==len(manifest['items']):
        ledger['status']='registered'; LEDGER.write_text(json.dumps(ledger,ensure_ascii=False,indent=2),encoding='utf-8')
    return 0
if __name__=='__main__': raise SystemExit(main())
