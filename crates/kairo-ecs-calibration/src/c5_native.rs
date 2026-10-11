use serde_json::Value;
pub(crate) fn evaluate(_: &Value,_:&Value)->Result<Value,String>{Err("C5 native not implemented".into())}
pub(crate) fn verify_trace(_: &Value)->Result<Value,String>{Err("C5 trace not implemented".into())}
pub(crate) fn replay(_: &Value,_:&str)->Result<Value,String>{Err("C5 replay not implemented".into())}
