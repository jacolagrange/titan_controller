use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::config_parse::ParsedArgs;
use crate::sniper_config::SniperConfig;

pub fn get_hash_sniper_config(exp_args: &str) -> u64 {
    let p_args = ParsedArgs::new(exp_args);
    let final_conf = p_args.get_final_config();
    //println!("Final_conf {:#?}", final_conf);
    let hash = hash_config_normalized(&final_conf);
    println!("HASH is {hash}");
    hash
}

fn hash_config_normalized(config: &SniperConfig) -> u64 {
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    hasher.finish()
}
