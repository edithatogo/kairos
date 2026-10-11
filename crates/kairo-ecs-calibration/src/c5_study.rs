use serde_json::Value;
pub(crate) fn plan(_: &Value)->Result<Value,String>{Err("C5 plan not implemented".into())}
pub(crate) fn summarize(_: &Value,_:&[Value])->Result<Value,String>{Err("C5 summary not implemented".into())}
