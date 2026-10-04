import gzip,hashlib,io,json,pathlib,tarfile,re,sys
root=pathlib.Path(__file__).resolve().parent
child=pathlib.Path(sys.argv[1]).resolve() if len(sys.argv)>1 else root.parents[2]
output=root/"qualification.tar.gz"
blocked={"reference-venv","wheels","ruff-cache","__pycache__",".ruff_cache","reference-worker"}
items={}
for p in root.rglob("*"):
    rel=p.relative_to(root)
    if not p.is_file() or p.is_symlink() or any(x in blocked or x.startswith("target") for x in rel.parts): continue
    if rel.name in {"qualification.tar.gz","archive-build-receipt.json","artifact-inventory.json","SHA256SUMS","archive-reproduction.json"}:continue
    if p.suffix not in {".json",".jsonl",".log",".py",".md",".lock"}: continue
    items["proof/"+rel.as_posix()]=p.read_bytes()
for prefix in ["conformance/c41","conductor/design/calibration"]:
    for p in (child/prefix).rglob("*"):
        if not p.is_file() or p.is_symlink(): continue
        if prefix.endswith("calibration") and not p.name.startswith("c4."):continue
        if "__pycache__" in p.parts or ".ruff_cache" in p.parts:continue
        if p.suffix not in {".json",".py",".md",".lock"}:continue
        items["source/"+p.relative_to(child).as_posix()]=p.read_bytes()
for relative in ["crates/kairo-ecs-calibration/src/lib.rs","crates/kairo-ecs-calibration/src/metrics.rs","crates/kairo-ecs-calibration/src/residuals.rs","crates/kairo-ecs-calibration/src/metric_cohorts.rs","crates/kairo-ecs-calibration/tests/metrics_c41.rs","crates/kairo-ecs-calibration/tests/metrics_c42.rs","crates/kairo-ecs-calibration/tests/cohorts_c42.rs","crates/kairo-ecs-calibration/tests/residuals_c42.rs","crates/kairo-ecs-calibration/tests/distance_c42.rs"]:
 p=child/relative;items["source/"+relative]=p.read_bytes()
assert sum(map(len,items.values()))<10*1024*1024,"archive exceeds reviewed budget"
for name,data in items.items():
    assert not name.startswith("/") and ".." not in pathlib.PurePosixPath(name).parts
    assert not re.search(rb'"token"\s*:\s*"[0-9a-fA-F]{32}"',data),"private lease token forbidden"
    assert not re.search(rb"--token\s+[0-9a-fA-F]{32}(?![0-9a-fA-F])",data),"private lease argument forbidden"
with output.open("wb") as raw:
    with gzip.GzipFile(filename="",mode="wb",fileobj=raw,mtime=0,compresslevel=9) as compressed:
        with tarfile.open(fileobj=compressed,mode="w") as tar:
            for name,data in sorted(items.items()):
                info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o644;info.mtime=0;info.uid=info.gid=0
                tar.addfile(info,io.BytesIO(data))
inventory=[{"path":n,"bytes":len(d),"sha256":hashlib.sha256(d).hexdigest()} for n,d in sorted(items.items())]
(root/"artifact-inventory.json").write_text(json.dumps(inventory,indent=2)+"\n")
with tarfile.open(output,"r:gz") as tar:
    members=tar.getmembers();assert len(members)==len(inventory)
    for m,row in zip(members,inventory):
        assert m.isfile() and m.name==row["path"] and m.size==row["bytes"]
        assert hashlib.sha256(tar.extractfile(m).read()).hexdigest()==row["sha256"]
receipt={"artifact":output.name,"sha256":hashlib.sha256(output.read_bytes()).hexdigest(),"compressed_bytes":output.stat().st_size,"member_count":len(inventory),"uncompressed_bytes":sum(x["bytes"] for x in inventory),"every_member_independently_read_back":True,"deterministic_tar_gzip_metadata":True,"excluded":"build caches, wheels, venv, private claims/tokens"}
(root/"archive-build-receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
(root/"SHA256SUMS").write_text(receipt["sha256"]+"  "+output.name+"\n")
print(json.dumps(receipt,indent=2))
