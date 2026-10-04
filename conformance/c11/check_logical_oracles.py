#!/opt/homebrew/opt/python@3.14/bin/python3.14
"""Check C1.1 synthetic logical fixture integrity, never a production mapper."""
from __future__ import annotations
import argparse, copy, hashlib, importlib.metadata, json, pathlib, re
from datetime import date, datetime
from pathlib import Path
from jsonschema import Draft202012Validator, FormatChecker

EXPECTED_JSONSCHEMA="4.26.0"
EXPECTED_SCHEMA_SHA256="8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842"
REQUIRED_CASES={"shape_wide_equivalent","shape_long_equivalent","three_clock_roles","unit_nanosecond","unit_microsecond","unit_millisecond","unit_second","explicit_offset_positive","minute_only_profile_excluded","date_only_excluded","preresolved_minute_helper_limit","naive_local_missing_zone","ny_fold_preclassified","ny_gap_preclassified","subnanosecond_raw","pre_origin_raw","u128_overflow_raw_ns","duplicate_source_event_key_dataset_failure","missing_source_event_key_dataset_failure","stable_equal_time_order","reversed_interval_invalid","open_interval","boarding_after_episode_end","missing_triage_denominator","right_censor_retained","knowledge_cutoff_equal_eligible","knowledge_after_cutoff_ineligible","knowledge_unknown_ineligible","fhir_meta_lastupdated_not_source_recorded","hl7_msh7_not_occurrence","a08_not_physical_movement","omop_visit_end_requires_lineage"}
UNIT_EXPECTATIONS={"unit_nanosecond":("2024-01-01T00:00:00.000000007Z",7,"nanosecond"),"unit_microsecond":("2024-01-01T00:00:00.000123Z",123000,"microsecond"),"unit_millisecond":("2024-01-01T00:00:00.456Z",456000000,"millisecond"),"unit_second":("2024-01-01T00:00:09Z",9000000000,"second")}
EXCLUSION_REASONS={"minute_only_profile_excluded":"date_only_or_coarse_precision","date_only_excluded":"date_only_or_coarse_precision","naive_local_missing_zone":"missing_timezone","ny_fold_preclassified":"ambiguous_dst_fold","ny_gap_preclassified":"nonexistent_dst_gap","subnanosecond_raw":"sub_nanosecond","pre_origin_raw":"pre_origin","u128_overflow_raw_ns":"overflow"}
SEMANTIC_DECISIONS={"fhir_meta_lastupdated_not_source_recorded":"meta.lastUpdated is resource update time, neither source_recorded_time nor occurrence_time","hl7_msh7_not_occurrence":"MSH-7 is message creation time, not occurrence","a08_not_physical_movement":"A08 code alone does not prove physical movement","omop_visit_end_requires_lineage":"OMOP visit end without ETL lineage is not observed physical departure"}
TIME_RE=re.compile(r"^(\d{4}-\d{2}-\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(Z|[+-]\d{2}:\d{2})$")
COUNT_FIELDS=("source_rows","candidate_units","accepted_units","excluded_units","failed_units","unresolved_units")
U128_MAX=340282366920938463463374607431768211455

def no_duplicate_object(pairs):
    result={}
    for key,value in pairs:
        if key in result: raise ValueError(f"duplicate JSON key: {key}")
        result[key]=value
    return result

def load_json(path): return json.loads(Path(path).read_text(),object_pairs_hook=no_duplicate_object)

def strict_rfc3339(value):
    if not isinstance(value,str): return False
    match=TIME_RE.fullmatch(value)
    if not match: return False
    day,hh,mm,ss,fraction,zone=match.groups()
    if zone!="Z":
        offset_hours,offset_minutes=(int(part) for part in zone[1:].split(":"))
        if offset_hours>23 or offset_minutes>59: return False
    if fraction and len(fraction)>9 and any(c!="0" for c in fraction[9:]): return False
    try: parsed=datetime.fromisoformat(f"{day}T{hh}:{mm}:{ss}{'.'+fraction if fraction else ''}{'+00:00' if zone=='Z' else zone}")
    except ValueError: return False
    return parsed.tzinfo is not None

def exact_ns_from_origin(raw):
    if not strict_rfc3339(raw): raise ValueError(f"invalid RFC3339 or nonzero sub-ns fraction: {raw!r}")
    day,hh,mm,ss,fraction,zone=TIME_RE.fullmatch(raw).groups()
    local=datetime.fromisoformat(f"{day}T{hh}:{mm}:{ss}{'.'+fraction if fraction else ''}{'+00:00' if zone=='Z' else zone}")
    offset=local.utcoffset()
    seconds=(local.date().toordinal()-date(1970,1,1).toordinal())*86400+int(hh)*3600+int(mm)*60+int(ss)-(offset.days*86400+offset.seconds)
    origin=(date(2024,1,1).toordinal()-date(1970,1,1).toordinal())*86400
    fraction_ns=int(((fraction or "")+"000000000")[:9])
    return (seconds-origin)*1_000_000_000+fraction_ns

def strict_equal(left,right):
    if type(left) is not type(right): return False
    if isinstance(left,dict): return left.keys()==right.keys() and all(strict_equal(left[k],right[k]) for k in left)
    if isinstance(left,list): return len(left)==len(right) and all(strict_equal(a,b) for a,b in zip(left,right))
    return left==right

def expected_consumer_output(doc):
    rows=[]
    for case in doc["cases"]:
        item={"case_id":case["case_id"],"classification":case["expected"]["classification"],"records":case["expected"]["records"],"accounting":case["accounting"],"oracle":case["expected"].get("oracle",{})}
        if "exclusion_reason" in case["expected"]: item["exclusion_reason"]=case["expected"]["exclusion_reason"]
        rows.append(item)
    return {"cases":rows,"companion_diagnostics":doc["companion_diagnostics"]}

def compare_actual(doc,actual):
    expected=expected_consumer_output(doc); errors=[]
    if not isinstance(actual,dict) or set(actual)!={"cases","companion_diagnostics"}: return ["actual output must contain exactly cases and companion_diagnostics"]
    rows=actual.get("cases")
    if not isinstance(rows,list): return ["actual cases must be a list"]
    ids=[r.get("case_id") for r in rows if isinstance(r,dict)]
    strings=[v for v in ids if isinstance(v,str)]
    if len(ids)!=len(rows) or len(strings)!=len(ids) or len(strings)!=len(set(strings)): errors.append("actual case IDs missing, malformed or duplicated")
    want={r["case_id"]:r for r in expected["cases"]}; got={r["case_id"]:r for r in rows if isinstance(r,dict) and isinstance(r.get("case_id"),str)}
    if got.keys()!=want.keys(): errors.append("actual case coverage mismatch")
    for cid in got.keys()&want.keys():
        if not strict_equal(got[cid],want[cid]): errors.append(f"actual case records/exclusion/reason/count/oracle mismatch: {cid}")
    if not strict_equal(actual.get("companion_diagnostics"),expected["companion_diagnostics"]): errors.append("actual companion diagnostics/count mismatch")
    return errors

def time_value_error(value,label):
    ticks=value.get("relative_ticks")
    if not isinstance(ticks,str) or not ticks.isascii() or not ticks.isdigit() or (len(ticks)>1 and ticks[0]=="0"): return f"{label}: noncanonical ticks"
    integer=int(ticks)
    if integer>U128_MAX: return f"{label}: ticks exceed u128"
    computed=exact_ns_from_origin(value.get("utc",""))
    if computed<0 or computed!=integer: return f"{label}: UTC/tick mismatch or pre-origin"
    # This one helper control receives a resolved instant, not parsed raw minute text.
    if label.startswith("preresolved_minute_helper_limit/"):
        if value.get("raw") != "2024-01-01T00:03Z" or value.get("utc") != "2024-01-01T00:03:00Z" or value.get("source_precision") != "minute":
            return f"{label}: pre-resolved minute control changed"
    elif exact_ns_from_origin(value.get("raw", "")) != integer:
        return f"{label}: raw/UTC/tick mismatch"
    return None

def check_document(doc,schema,validator):
    errors=[]
    if doc.get("corpus_version")!="c11.logical-oracles.v1" or doc.get("schema_uri")!=schema.get("$id"): errors.append("corpus/schema identity")
    if doc.get("case_ids")!=[c.get("case_id") for c in doc.get("cases",[])]: errors.append("case_ids index mismatch")
    if not isinstance(doc.get("accounting_unit"),str) or "source_rows" not in doc["accounting_unit"] or "candidate_units" not in doc["accounting_unit"]: errors.append("counting unit definition")
    for key in ("synthetic_only","logical_records_only"):
        if doc.get(key) is not True: errors.append(f"{key} boundary")
    for key in ("runtime_mapper_executed","physical_arrow_schema_claim","source_conformance_claim"):
        if doc.get(key) is not False: errors.append(f"{key} boundary")
    if doc.get("profile",{}).get("final_manifest_emitted") is not False or doc.get("companion_diagnostics",{}).get("final_manifest_emitted") is not False: errors.append("no final manifest boundary")
    cases=doc.get("cases")
    if not isinstance(cases,list): return errors+["cases must be a list"]
    ids=[c.get("case_id") for c in cases if isinstance(c,dict)]; strings=[x for x in ids if isinstance(x,str)]
    if len(ids)!=len(cases) or len(strings)!=len(ids) or len(strings)!=len(set(strings)): errors.append("case IDs missing/malformed/duplicated")
    if set(strings)!=REQUIRED_CASES: errors.append("required case coverage mismatch")
    by_id={c["case_id"]:c for c in cases if isinstance(c,dict) and isinstance(c.get("case_id"),str)}
    schema_errors=[]
    for case in cases:
        cid=case.get("case_id"); records=case.get("expected",{}).get("records",[])
        if not isinstance(records,list): schema_errors.append(f"{cid}: records must be a list"); continue
        for ix,record in enumerate(records):
            schema_errors.extend(f"{cid}/{ix}: {err.message}" for err in validator.iter_errors(record))
    if schema_errors: return errors+schema_errors
    all_time_values=[]
    def walk_times(node,label):
        found=[]
        if isinstance(node,dict):
            if {"raw","utc","relative_ticks","source_precision","lineage"}.issubset(node): found.append((label,node))
            for key,value in node.items(): found.extend(walk_times(value,f"{label}/{key}"))
        elif isinstance(node,list):
            for ix,value in enumerate(node): found.extend(walk_times(value,f"{label}/{ix}"))
        return found
    for case in cases:
        cid=case.get("case_id"); expected=case.get("expected",{}); records=expected.get("records",[]); account=case.get("accounting",{})
        for ix,record in enumerate(records): all_time_values.extend(walk_times(record,f"{cid}/{ix}"))
        if set(account)!=set(COUNT_FIELDS) or any(type(v) is not int or v<0 for v in account.values()): errors.append(f"{cid}: invalid companion accounting")
        else:
            if sum(account[k] for k in COUNT_FIELDS[2:])!=account["candidate_units"]: errors.append(f"{cid}: candidate unit conservation")
            raw_rows=case.get("raw_inputs",{}).get("rows")
            if isinstance(raw_rows,list) and len(raw_rows)!=account["source_rows"]: errors.append(f"{cid}: physical source row count")
        if expected.get("classification")=="excluded":
            exclusions=[r for r in records if r.get("record_type")=="trace_exclusion.v1"]
            reason=EXCLUSION_REASONS.get(cid)
            if len(exclusions)!=1 or expected.get("exclusion_reason")!=reason or exclusions[0].get("exclusion_reason")!=reason: errors.append(f"{cid}: fixed negative reason/exclusion record")
            else:
                raw=case.get("raw_inputs",{}).get("raw_timestamp",case.get("raw_inputs",{}).get("raw_occurrence"))
                if raw is None or raw not in exclusions[0].get("raw_time_values",{}).values(): errors.append(f"{cid}: raw source time not retained")
        if expected.get("classification")=="dataset_failure" and records: errors.append(f"{cid}: failure fabricated normalized/exclusion record")
    for label,value in all_time_values:
        try: failure=time_value_error(value,label)
        except Exception as exc: failure=f"{label}: time arithmetic error {exc}"
        if failure: errors.append(failure)
    for case in cases:
        for record in case.get("expected",{}).get("records",[]):
            if record.get("record_type")=="trace_event.v1":
                top=record.get("relative_ticks"); occurrence=record.get("occurrence_time",{}).get("relative_ticks")
                if top!=occurrence: errors.append(f"{case.get('case_id')}: top-level/occurrence tick mismatch")
                if isinstance(top,str) and top.isdigit() and int(top)>U128_MAX: errors.append(f"{case.get('case_id')}: top-level tick exceeds u128")
    if errors: return errors
    totals={key:0 for key in COUNT_FIELDS}
    for case in cases:
        for key in COUNT_FIELDS: totals[key]+=case["accounting"][key]
    if not strict_equal(doc.get("companion_diagnostics",{}).get("aggregate_counts"),totals): errors.append("companion aggregate count mismatch")
    for cid,(raw,ticks,precision) in UNIT_EXPECTATIONS.items():
        case=by_id[cid]; record=case["expected"]["records"][0]
        if case["raw_inputs"].get("raw_timestamp")!=raw or exact_ns_from_origin(raw)!=ticks or case["raw_inputs"].get("expected_delta_ns")!=str(ticks): errors.append(f"{cid}: exact raw/integer delta")
        if record.get("relative_ticks")!=str(ticks) or record["occurrence_time"].get("relative_ticks")!=str(ticks) or record["occurrence_time"].get("source_precision")!=precision: errors.append(f"{cid}: expected time literal")
    offset=by_id["explicit_offset_positive"]; offset_raw=offset["raw_inputs"]["raw_timestamp"]; offset_time=offset["expected"]["records"][0]["occurrence_time"]
    if exact_ns_from_origin(offset_raw)!=0 or offset_time.get("utc")!="2024-01-01T00:00:00Z" or offset_time.get("source_offset_or_zone")!="+01:00": errors.append("offset normalization/raw retention")
    if by_id["shape_wide_equivalent"]["expected"]["records"]!=by_id["shape_long_equivalent"]["expected"]["records"]: errors.append("wide/long expected logical rows differ")
    wide=by_id["shape_wide_equivalent"]; long=by_id["shape_long_equivalent"]
    if len(wide["raw_inputs"].get("rows",[]))!=1 or set(wide["raw_inputs"]["rows"][0])!={"event_a","event_b"} or len(long["raw_inputs"].get("rows",[]))!=8 or len({r.get("source_event_key") for r in long["raw_inputs"]["rows"]})!=2: errors.append("wide/long physical shape mismatch")
    if any(wide["accounting"][k]!=2 for k in ("candidate_units","accepted_units")) or any(long["accounting"][k]!=2 for k in ("candidate_units","accepted_units")): errors.append("wide/long candidate counts")
    wide_events=wide["raw_inputs"]["rows"][0]
    wide_by_key={event["source_event_key"]:event for event in wide_events.values()}
    long_by_key={}
    for row in long["raw_inputs"]["rows"]: long_by_key.setdefault(row["source_event_key"],{})[row["field"]]=row["value"]
    for source,records in ((wide_by_key,wide["expected"]["records"]),(long_by_key,long["expected"]["records"])):
        if set(source)!= {record["source_event_key"] for record in records}: errors.append("wide/long raw event key coverage")
        for record in records:
            raw=source.get(record["source_event_key"],{})
            kind=raw.get("kind",raw.get("kind"))
            occurrence=raw.get("occurrence")
            rank=raw.get("rank")
            source_order=raw.get("source_order")
            try: rank_value=int(rank); source_order_value=int(source_order)
            except (TypeError,ValueError): rank_value=None; source_order_value=None
            if kind is None or occurrence is None or source_order_value is None: errors.append("wide/long raw event fields missing")
            elif kind!=record["event_kind"] or occurrence!=record["occurrence_time"]["raw"] or rank_value!=record["event_kind_rank"] or source_order_value!=record["source_order"]: errors.append("wide/long raw event mapping mismatch")
    clocks=by_id["three_clock_roles"]; clock_record=clocks["expected"]["records"][0]
    for name,input_name,lineage in (("occurrence_time","occurrence","occurrence"),("source_recorded_time","source_recorded","source_recorded"),("message_created_time","message_created","message_created")):
        value=clock_record.get(name); raw=clocks["raw_inputs"].get(input_name)
        if not isinstance(value,dict) or value.get("raw")!=raw or clock_record["time_lineage"][lineage].get("status")!="observed": errors.append(f"three clocks: {name} raw/lineage")
    if len({clock_record[n]["relative_ticks"] for n in ("occurrence_time","source_recorded_time","message_created_time")})!=3: errors.append("three clock instants not distinct")
    minute=by_id["minute_only_profile_excluded"]["expected"]
    if minute.get("classification")!="excluded" or minute.get("exclusion_reason")!="date_only_or_coarse_precision": errors.append("minute-only profile exclusion")
    minute_raw=by_id["minute_only_profile_excluded"]["raw_inputs"]
    minute_fields=minute["records"][0]["raw_event"]["source_fields"]
    if minute_raw.get("declared_precision")!="minute" or minute_fields.get("declared_precision")!="minute": errors.append("minute declared precision raw/source retention")
    helper=by_id["preresolved_minute_helper_limit"]["expected"]["records"][0]
    if helper["occurrence_time"]["source_precision"]!="minute" or helper["relative_ticks"]!="180000000000": errors.append("pre-resolved helper minute limit")
    duplicate=by_id["duplicate_source_event_key_dataset_failure"]
    if duplicate["raw_inputs"]["rows"][0]["source_event_key"]!=duplicate["raw_inputs"]["rows"][1]["source_event_key"] or duplicate["accounting"]["failed_units"]!=2: errors.append("duplicate identity input/failure count")
    missing=by_id["missing_source_event_key_dataset_failure"]
    if "source_event_key" in missing["raw_inputs"]["rows"][0] or missing["accounting"]["failed_units"]!=1: errors.append("missing identity fabricated key/count")
    order_case=by_id["stable_equal_time_order"]; order=order_case["expected"]["records"]
    raw_by_key={row["source_event_key"]:row for row in order_case["raw_inputs"]["rows"]}
    for record in order:
        raw=raw_by_key.get(record["source_event_key"],{})
        try: raw_rank=int(raw.get("rank"))
        except (TypeError,ValueError): raw_rank=None
        if raw_rank!=record["event_kind_rank"] or raw.get("source_order")!=record.get("source_order"): errors.append("raw rank/source order/expected binding mismatch")
    keys=[(int(r["relative_ticks"]),r["case_key"],r["occurrence"],r["event_kind_rank"],r["source_event_key"],r["source_order"]) for r in order]
    if keys!=sorted(keys) or not any(r["event_kind_rank"]>2**64-1 for r in order): errors.append("stable sort/unbounded rank control")
    interval=by_id["reversed_interval_invalid"]["expected"]["records"][0]
    start=int(interval["start"]["time"]["relative_ticks"]); end=int(interval["end"]["time"]["relative_ticks"])
    if not start>end or interval["status"]!="invalid": errors.append("reversed interval control")
    open_record=by_id["open_interval"]["expected"]["records"][0]
    if open_record["end"] is not None or open_record["status"]!="open_right": errors.append("open interval null endpoint control")
    boarding=by_id["boarding_after_episode_end"]
    timeline={r["event_kind"]:int(r["relative_ticks"]) for r in boarding["expected"]["records"]}
    if not (timeline.get("episode_end",0)<timeline.get("boarding",0)<timeline.get("physical_departure",0)): errors.append("episode/boarding/departure order")
    if boarding["accounting"]["source_rows"]!=3 or boarding["accounting"]["candidate_units"]!=3: errors.append("boarding candidate count")
    raw_board={row["source_event_key"]:row for row in boarding["raw_inputs"].get("rows",[])}
    for record in boarding["expected"]["records"]:
        raw=raw_board.get(record["source_event_key"],{})
        if raw.get("event_kind")!=record.get("event_kind") or raw.get("occurrence")!=record.get("occurrence_time",{}).get("raw") or raw.get("source_order")!=record.get("source_order"): errors.append("boarding raw/expected event chronology mismatch")
    triage=by_id["missing_triage_denominator"]
    if triage["expected"]["oracle"].get("eligible_denominator",0) is not None or triage["expected"]["oracle"].get("final_manifest_emitted") is not False: errors.append("missing denominator/final manifest control")
    right=by_id["right_censor_retained"]["expected"]["records"][0]
    right_case=by_id["right_censor_retained"]; right_raw=right_case["raw_inputs"]
    if not (right["censor_status"]=="right" and right["event_observed"] is False and right["event_time"] is None and right["last_observed"] is not None): errors.append("right censor semantics")
    if right_raw.get("endpoint_observed") is not False or right_raw.get("risk_start")!=right["risk_start"].get("raw") or right_raw.get("last_observed")!=right["last_observed"].get("raw") or int(right["risk_start"]["relative_ticks"])>int(right["last_observed"]["relative_ticks"]): errors.append("right censor raw interval/time binding")
    cutoff="2024-01-01T00:00:05Z"; cutoff_ns=exact_ns_from_origin(cutoff)
    for cid,status,eligible,at in (("knowledge_cutoff_equal_eligible","known",True,cutoff),("knowledge_after_cutoff_ineligible","not_yet_known",False,"2024-01-01T00:00:06Z"),("knowledge_unknown_ineligible","unknown",False,None)):
        case=by_id[cid]; record=case["expected"]["records"][0]; knowledge=record["knowledge_availability"]; time=knowledge.get("available_at")
        time_utc=time.get("utc") if isinstance(time,dict) else None
        actual_eligible=(knowledge.get("status")=="known" and time_utc is not None and exact_ns_from_origin(time_utc)<=cutoff_ns)
        if case["raw_inputs"].get("prediction_cutoff")!=cutoff or case["raw_inputs"].get("availability_status")!=status or case["raw_inputs"].get("available_at")!=at: errors.append(f"{cid}: raw cutoff input")
        if knowledge.get("status")!=status or time_utc!=at or actual_eligible is not eligible or case["expected"]["oracle"].get("feature_eligible") is not eligible: errors.append(f"{cid}: cutoff/null arithmetic")
    for cid,decision in SEMANTIC_DECISIONS.items():
        case=by_id[cid]
        if case["expected"]["oracle"].get("decision")!=decision or case["expected"]["records"] or case["raw_inputs"].get("local_mapping_claim") is not False: errors.append(f"{cid}: source semantic counterexample")
    fhir=by_id["fhir_meta_lastupdated_not_source_recorded"]["raw_inputs"].get("synthetic_fields",{})
    if fhir.get("meta.lastUpdated")!="2024-01-01T00:00:04Z" or fhir.get("mapping_candidate")!="source_recorded_time" or fhir.get("synthetic_observation_time")!="2024-01-01T00:00:02Z": errors.append("FHIR resource update counterexample fields")
    hl7=by_id["hl7_msh7_not_occurrence"]["raw_inputs"].get("synthetic_fields",{})
    if hl7.get("MSH-7")!="2024-01-01T00:00:04Z" or hl7.get("synthetic_occurrence")!="2024-01-01T00:00:02Z": errors.append("HL7 clock-role counterexample fields")
    a08=by_id["a08_not_physical_movement"]["raw_inputs"].get("synthetic_fields",{})
    if a08.get("message")!="ADT^A08" or a08.get("movement_evidence") is not None: errors.append("A08 movement counterexample fields")
    omop=by_id["omop_visit_end_requires_lineage"]["raw_inputs"].get("synthetic_fields",{})
    if omop.get("visit_end_datetime")!="2024-01-01T00:00:04Z" or omop.get("etl_lineage") is not None: errors.append("OMOP lineage counterexample fields")
    if triage["raw_inputs"].get("cohort_rows")!=triage["accounting"]["source_rows"] or triage["raw_inputs"].get("triage_rows",0)>=triage["raw_inputs"].get("cohort_rows",0): errors.append("triage denominator raw counts")
    overflow=by_id["u128_overflow_raw_ns"]; overflow_raw=overflow["raw_inputs"]
    if overflow_raw.get("raw_encoding")!="decimal relative nanoseconds from origin" or overflow_raw.get("origin")!="2024-01-01T00:00:00Z" or overflow_raw.get("raw_timestamp")!=str(U128_MAX+1) or int(overflow["expected"]["records"][0]["raw_time_values"]["occurrence"])<=U128_MAX: errors.append("relative-u128 overflow literal/basis")
    return errors

def mutation_probes(doc,schema,validator):
    probes=[]
    def fixture_probe(name,mutate):
        candidate=copy.deepcopy(doc); mutate(candidate)
        if not check_document(candidate,schema,validator): raise AssertionError(f"fixture mutation not detected: {name}")
        probes.append({"name":name,"detected":True})
    for name,mutate in (
        ("retained_raw_normalized_time_mismatch",lambda d:next(c for c in d["cases"] if c["case_id"]=="stable_equal_time_order")["expected"]["records"][0]["occurrence_time"].__setitem__("raw","2024-01-01T00:00:09Z")),
        ("changed_expected_tick",lambda d:d["cases"][3]["expected"]["records"][0].__setitem__("relative_ticks","8")),
        ("top_occurrence_tick_mismatch",lambda d:d["cases"][3]["expected"]["records"][0]["occurrence_time"].__setitem__("relative_ticks","8")),
        ("removed_raw_exclusion_evidence",lambda d:d["cases"][8]["expected"]["records"][0]["raw_time_values"].clear()),
        ("missing_case_coverage",lambda d:d["cases"].pop()),
        ("count_conservation_drift",lambda d:d["companion_diagnostics"]["aggregate_counts"].__setitem__("excluded_units",999)),
        ("schema_required_field_missing",lambda d:d["cases"][3]["expected"]["records"][0].pop("dataset_id")),
        ("malformed_utc_format",lambda d:d["cases"][3]["expected"]["records"][0]["occurrence_time"].__setitem__("utc","not-a-date")),
        ("detail_must_remain_string",lambda d:d["cases"][8]["expected"]["records"][0].__setitem__("detail",{})),
        ("cutoff_changed",lambda d:d["cases"][25]["expected"]["records"][0]["knowledge_availability"]["available_at"].__setitem__("utc","2024-01-01T00:00:06Z")),
        ("interval_status_changed",lambda d:d["cases"][20]["expected"]["records"][0].__setitem__("status","complete")),
        ("open_interval_filled",lambda d:d["cases"][21]["expected"]["records"][0].__setitem__("end",{})),
        ("censor_status_changed",lambda d:d["cases"][24]["expected"]["records"][0].__setitem__("censor_status","not_censored")),
        ("negative_reason_changed_both",lambda d:(d["cases"][8]["expected"].__setitem__("exclusion_reason","other"),d["cases"][8]["expected"]["records"][0].__setitem__("exclusion_reason","other"))),
        ("missing_denominator_fabricated",lambda d:d["cases"][23]["expected"]["oracle"].__setitem__("eligible_denominator",0)),
        ("semantic_counterexample_changed",lambda d:d["cases"][28]["expected"]["oracle"].__setitem__("decision","substituted")),
        ("raw_rank_mismatch",lambda d:d["cases"][19]["raw_inputs"]["rows"][0].__setitem__("rank","2")),
        ("boarding_raw_chronology_changed",lambda d:d["cases"][22]["raw_inputs"]["rows"][1].__setitem__("occurrence","2024-01-01T00:00:14Z")),
        ("fhir_update_substitution_changed",lambda d:d["cases"][28]["raw_inputs"]["synthetic_fields"].__setitem__("meta.lastUpdated","2024-01-01T00:00:02Z")),
        ("wide_raw_event_occurrence_changed",lambda d:next(c for c in d["cases"] if c["case_id"]=="shape_wide_equivalent")["raw_inputs"]["rows"][0]["event_a"].__setitem__("occurrence","2024-01-01T00:00:09Z")),
        ("long_raw_event_rank_changed",lambda d:next(r for r in next(c for c in d["cases"] if c["case_id"]=="shape_long_equivalent")["raw_inputs"]["rows"] if r["field"]=="rank" and r["source_event_key"]=="wide-a").__setitem__("value","99")),
        ("stable_raw_source_order_changed",lambda d:next(r for r in next(c for c in d["cases"] if c["case_id"]=="stable_equal_time_order")["raw_inputs"]["rows"] if r["source_event_key"]=="event-c").__setitem__("source_order",99)),
        ("relative_overflow_basis_changed",lambda d:next(c for c in d["cases"] if c["case_id"]=="u128_overflow_raw_ns")["raw_inputs"].__setitem__("raw_encoding","absolute timestamp nanoseconds")),
        ("minute_precision_source_retention_changed",lambda d:next(c for c in d["cases"] if c["case_id"]=="minute_only_profile_excluded")["expected"]["records"][0]["raw_event"]["source_fields"].pop("declared_precision")),
        ("censor_last_observed_binding_changed",lambda d:next(c for c in d["cases"] if c["case_id"]=="right_censor_retained")["raw_inputs"].__setitem__("last_observed","2024-01-01T00:00:06Z")),
        ("invalid_offset_minute",lambda d:d["cases"][3]["expected"]["records"][0]["occurrence_time"].__setitem__("utc","2024-01-01T00:00:00+01:60")),
        ("invalid_offset_hour",lambda d:d["cases"][3]["expected"]["records"][0]["occurrence_time"].__setitem__("utc","2024-01-01T00:00:00+24:00")),
    ):
        fixture_probe(name,mutate)
    exact=expected_consumer_output(doc)
    for name,mutate in (
        ("actual_missing_case",lambda a:a["cases"].pop()),
        ("actual_changed_record",lambda a:a["cases"][3]["records"][0].__setitem__("relative_ticks","8")),
        ("actual_changed_counts",lambda a:a["companion_diagnostics"]["aggregate_counts"].__setitem__("candidate_units",-1)),
        ("actual_malformed_case_id",lambda a:a["cases"][0].__setitem__("case_id",{})),
        ("actual_bool_not_integer",lambda a:a["cases"][0]["accounting"].__setitem__("source_rows",True)),
        ("actual_oracle_diagnostic",lambda a:a["cases"][0]["oracle"].__setitem__("expected_records_must_match",False)),
    ):
        candidate=copy.deepcopy(exact); mutate(candidate)
        if not compare_actual(doc,candidate): raise AssertionError(f"actual comparator mutation not detected: {name}")
        probes.append({"name":name,"detected":True,"scope":"candidate comparison only"})
    if compare_actual(doc,exact): raise AssertionError("exact expected projection rejected")
    try: no_duplicate_object([("x",1),("x",2)])
    except ValueError: probes.append({"name":"duplicate_json_key_rejected","detected":True})
    else: raise AssertionError("duplicate JSON object keys accepted")
    return probes

def main():
    parser=argparse.ArgumentParser(description="C1.1 logical fixture integrity checker; no mapper is run")
    parser.add_argument("--schema",required=True); parser.add_argument("--corpus",default=str(pathlib.Path(__file__).with_name("logical-oracles-v1.json")))
    parser.add_argument("--actual",help="optional future consumer JSON in the generic comparison shape")
    args=parser.parse_args(); version=importlib.metadata.version("jsonschema")
    if version!=EXPECTED_JSONSCHEMA: raise SystemExit(f"jsonschema version {version}; expected {EXPECTED_JSONSCHEMA}")
    schema_path=pathlib.Path(args.schema); corpus_path=pathlib.Path(args.corpus)
    schema_bytes=schema_path.read_bytes(); schema_hash=hashlib.sha256(schema_bytes).hexdigest()
    if schema_hash!=EXPECTED_SCHEMA_SHA256: raise SystemExit(f"schema hash {schema_hash}; expected {EXPECTED_SCHEMA_SHA256}")
    schema=json.loads(schema_bytes,object_pairs_hook=no_duplicate_object); doc=load_json(corpus_path)
    Draft202012Validator.check_schema(schema); formats=FormatChecker(); formats.checks("date-time")(strict_rfc3339)
    validator=Draft202012Validator(schema,format_checker=formats); errors=check_document(doc,schema,validator)
    if errors: raise SystemExit("fixture integrity failed: "+"; ".join(errors))
    probes=mutation_probes(doc,schema,validator); consumer_status="not_executed"
    if args.actual:
        actual=load_json(args.actual); mismatches=compare_actual(doc,actual)
        if mismatches: raise SystemExit("consumer output mismatch: "+"; ".join(mismatches))
        consumer_status="compared_and_matched"
    print(json.dumps({"status":"PASS","scope":"fixture integrity only; no production mapper run","consumer_status":consumer_status,"jsonschema_version":version,"case_count":len(doc["cases"]),"logical_record_count":sum(len(c["expected"]["records"]) for c in doc["cases"]),"mutation_probes":probes,"schema_sha256":schema_hash,"corpus_sha256":hashlib.sha256(corpus_path.read_bytes()).hexdigest()},sort_keys=True))
    return 0
if __name__=="__main__": raise SystemExit(main())
