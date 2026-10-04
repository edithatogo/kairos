import datetime, hashlib, json, os, pathlib, subprocess, sys, time
root=pathlib.Path(__file__).resolve().parent
spec=json.loads(pathlib.Path(sys.argv[1]).read_text())
receipts=[]
for c in spec:
    cwd=pathlib.Path(c["cwd"]); env=os.environ.copy(); env.update(c.get("env",{}))
    head=subprocess.check_output(["git","rev-parse","HEAD"],cwd=cwd,text=True).strip()
    started=datetime.datetime.now(datetime.timezone.utc).isoformat(); begin=time.monotonic()
    p=subprocess.run(c["argv"],cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    log=root/(c["id"]+".log"); log.write_bytes(p.stdout)
    hashes={x:hashlib.sha256((cwd/x).read_bytes()).hexdigest() for x in c.get("inputs",[])}
    item={"id":c["id"],"argv":c["argv"],"cwd":str(cwd),"head":head,"env_overrides":c.get("env",{}),"started_utc":started,"ended_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"duration_seconds":time.monotonic()-begin,"exit_status":p.returncode,"input_sha256":hashes,"log":log.name,"log_sha256":hashlib.sha256(p.stdout).hexdigest(),"interpretation":c.get("interpretation","ordinary gate")}
    receipts.append(item); (root/(pathlib.Path(sys.argv[1]).stem+"-receipts.json")).write_text(json.dumps(receipts,indent=2)+"\n")
    print(c["id"],p.returncode,p.stdout.decode(errors="replace")[-1800:],flush=True)
    if p.returncode != c.get("expected_exit",0): raise SystemExit(p.returncode or 1)
