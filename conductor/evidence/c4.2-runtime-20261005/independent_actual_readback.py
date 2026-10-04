import hashlib,importlib.util,json,pathlib,subprocess,sys
child=pathlib.Path(sys.argv[1]); report=pathlib.Path(sys.argv[2]); output=pathlib.Path(sys.argv[3])
spec=importlib.util.spec_from_file_location("c41_independent",child/"conformance/c41/test_reference.py")
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
data=json.loads(report.read_text());m.compare_candidate(data,m.FIXTURES)
head=subprocess.check_output(["git","rev-parse","HEAD"],cwd=child,text=True).strip()
assert data["provenance"]["producer_commit"]==head
source=b"".join((child/p).read_bytes() for p in ["crates/kairo-ecs-calibration/tests/metrics_c42.rs","crates/kairo-ecs-calibration/src/metrics.rs"])
assert data["provenance"]["producer_source_sha256"]==hashlib.sha256(source).hexdigest()
for p in ["crates/kairo-ecs-calibration/tests/metrics_c42.rs","crates/kairo-ecs-calibration/src/metrics.rs"]:
 assert subprocess.check_output(["git","show",f"HEAD:{p}"],cwd=child)==(child/p).read_bytes()
cases={x["id"]:x["result"] for x in data["cases"]}
assert float(cases["tail-180"]["w1"])==180.0 and float(cases["tail-180"]["ks_d"])==0.2
assert abs(float(cases["fractional"]["w1"])-1/3)<1e-12 and float(cases["fractional"]["ks_d"])==0.5
result={"status":"pass","head":head,"actual_case_count":len(cases),"producer_source_sha256":hashlib.sha256(source).hexdigest(),"actual_report_sha256":hashlib.sha256(report.read_bytes()).hexdigest(),"independent_comparator":"unchanged C4.1 Python exact Fraction oracle","manual_tail":"CDF gap1/5 over900 units:W1=180,D=0.2","manual_fractional":"two equally weighted quantile distances1/2 and1/6: W1=1/3,D=1/2"}
output.write_text(json.dumps(result,indent=2)+"\n");print(json.dumps(result,indent=2))
