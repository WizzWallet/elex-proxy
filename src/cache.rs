use std::hash::{DefaultHasher, Hash, Hasher};

use serde_json::Value;

pub fn to_cache_key(method: &str, params: &[Value]) -> u64 {
    let mut hasher = DefaultHasher::new();
    method.hash(&mut hasher);
    for x in params.iter() {
        if x.is_string() {
            x.as_str().unwrap().hash(&mut hasher);
        } else if x.is_number() {
            // never use f64 as a key
            x.as_i64().unwrap().hash(&mut hasher);
        } else if x.is_boolean() {
            x.as_bool().unwrap().hash(&mut hasher);
        } else {
            x.to_string().hash(&mut hasher);
        }
    }
    hasher.finish()
}
