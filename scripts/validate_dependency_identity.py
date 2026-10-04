"""Reject high dependency findings; correct only the pinned private alias identity."""
import hashlib
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]
PINS={
 'scripts/bootstrap-node-tools/package-lock.json':'3928f3049db0d21170bbf8fb564715eeb50d59f14b0a27ebcaa42381071b999e',
 'vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz':'fbd36545bda6d9cd7da805cff6967f96ca2f7f9c59b45e79f97a3e129eec7485',
 'vendor/http-cache-semantics-kairos-prototype/index.js':'ed6c1faabbe21f7bfef09ce258392cf181678149237a67ce492308a46ca6620c',
}
KNOWN={'GHSA-rc47-6667-2j5j','GHSA-ch52-4w7c-c8xp'}
def private_identity(root):
 for name,expected in PINS.items():
  p=root/name
  if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest()!=expected:raise ValueError('private dependency proof drift: '+name)
 lock=json.loads((root/'scripts/bootstrap-node-tools/package-lock.json').read_text())
 p=lock['packages']['node_modules/http-cache-semantics']
 if p.get('name')!='@careops/http-cache-semantics-kairos-prototype' or p.get('version')!='0.1.0':raise ValueError('private package identity drift')

def validate(pages,root=ROOT):
 if not isinstance(pages,list) or not pages or any(not isinstance(page,list) for page in pages):raise ValueError('missing paginated dependency response')
 rows=[row for page in pages for row in page];corrected=[]
 for row in rows:
  if not isinstance(row,dict) or row.get('change_type') not in ('added','removed'):raise ValueError('unknown dependency row')
  if row['change_type']=='removed':continue
  findings=row.get('vulnerabilities')
  if not isinstance(findings,list):raise ValueError('missing vulnerability array')
  for v in findings:
   if not isinstance(v,dict) or v.get('severity') not in ('low','moderate','high','critical'):raise ValueError('unknown vulnerability schema')
   if v['severity'] not in ('high','critical'):continue
   exact=all(row.get(k)==value for k,value in {'manifest':'scripts/bootstrap-node-tools/package-lock.json','ecosystem':'npm','name':'http-cache-semantics','version':'0.1.0','package_url':'pkg:npm/http-cache-semantics@0.1.0','license':'BSD-2-Clause','source_repository_url':None,'scope':'runtime'}.items())
   if not exact or v.get('severity')!='high' or v.get('advisory_ghsa_id') not in KNOWN:raise ValueError('high dependency finding: '+str(v))
   private_identity(root);corrected.append({'reported_purl':row['package_url'],'actual_name':'@careops/http-cache-semantics-kairos-prototype','advisory':v['advisory_ghsa_id'],'classification':'verified_alias_identity_mismatch'})
 return {'threshold':'high','corrected_identity_findings':corrected,'rows':len(rows)}
if __name__=='__main__':
 try:print(json.dumps(validate(json.loads(Path(sys.argv[1]).read_text())),indent=2))
 except (ValueError,KeyError,TypeError,OSError,IndexError) as e:raise SystemExit(str(e))
