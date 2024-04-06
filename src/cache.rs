use std::hash::{DefaultHasher, Hash, Hasher};

use serde_json::Value;

pub fn to_cache_key(method: &str, params: &[Value]) -> u64 {
    let mut hasher = DefaultHasher::new();
    method.hash(&mut hasher);
    for param in params {
        hash_json_value(param, &mut hasher);
    }
    hasher.finish()
}

fn hash_json_value(value: &Value, hasher: &mut DefaultHasher) {
    match value {
        Value::String(s) => s.hash(hasher),
        Value::Number(n) => n.as_i64().unwrap().hash(hasher),
        Value::Bool(b) => b.hash(hasher),
        Value::Array(a) => {
            for x in a.iter() {
                hash_json_value(x, hasher);
            }
        }
        Value::Object(o) => {
            for (k, v) in o.iter() {
                k.hash(hasher);
                hash_json_value(v, hasher);
            }
        }
        Value::Null => {
            "null".hash(hasher);
        }
    }
}
