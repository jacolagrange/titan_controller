use crate::config_parse::ParsedArgs;
use config::Config;
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use serde_json::Value;

pub fn get_hash_sniper_config(exp_args: &str) -> u64 {
    let p_args = ParsedArgs::new(exp_args);
    let hash = if let Ok(final_conf) = p_args.get_final_config(){
    // println!("{:#?}", final_conf);
        hash_config_normalized(&final_conf)
    } else {0};
    println!("HASH is {hash}");
    hash
}

fn hash_config_normalized(config: &Config) -> u64 {
    // Convert to Value, then to BTreeMap to ensure sorted keys
    let value: Value = config.clone().try_deserialize().unwrap();
    let normalized = normalize_json_value(value);
    println!("normalized is {:#?}", normalized);
    
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let btree: BTreeMap<String, Value> = map.into_iter()
                .map(|(k, v)| (k, normalize_json_value(v)))
                .collect();
            Value::Object(btree.into_iter().collect())
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(normalize_json_value).collect())
        }
        other => other,
    }
}
