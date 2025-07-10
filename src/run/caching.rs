use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;

use crate::constants;
use crate::run::sniper_config::SniperConfig;
use crate::run::config_parse::ParsedArgs;

pub fn get_hash_sniper_config(exp_args: &str) -> u64 {
    let p_args = ParsedArgs::new(exp_args);
    let final_conf = p_args.get_final_config();
    //println!("Final_conf {:#?}", final_conf);
    let hash = hash_config_normalized(&final_conf);
    //println!("HASH is {hash}");
    hash
}

fn hash_config_normalized(config: &SniperConfig) -> u64 {
    let mut hasher = FxHasher::default();
    config.hash(&mut hasher);
    hasher.finish()
}

pub fn get_tools_hash_path(git_repos: &HashMap<String, String>, traces: &Option<HashMap<String, String>>) -> PathBuf {
    let mut total_hash = String::new();

    for (repo, branch) in git_repos {
        let repo_location = match repo.as_str() {
            "benchmarks" => Some(Path::new(constants::LOCAL_BENCHMARK_DIR)),
            "sniper" => Some(Path::new(constants::LOCAL_SNIPER_DIR)),
            _ => None
        };
        if let Some(repo_location_str) = repo_location {
            let repo_hash = get_hash_git(&repo_location_str.to_path_buf(), branch);
            total_hash += &repo_hash;
        }
    }

    if let Some(trace_map) = traces {
        for (trace_suite, version) in trace_map {
            let trace_hash = hash_trace_folder(trace_suite, version).to_string();
            total_hash += &trace_hash;
        }
    }
    
    //Hash the final result in a managable string
    let mut hasher = FxHasher::default();
    total_hash.hash(&mut hasher);
    let final_hash = hasher.finish();

    let hash_map = format!("{:x}", final_hash);

    let hash_path = Path::new(constants::CACHE_FOLDER_NAME).join(hash_map);
    hash_path
}

fn get_hash_git(location: &PathBuf, branch: &str) -> String {
    let repo_path = location.as_path();
    
    let _fetch_result = Command::new("git")
        .arg("fetch")
        .arg("origin")
        .arg(branch)
        .current_dir(repo_path)
        .output()
        .expect("Failed to fetch from origin");
    
    // Get the commit hash from origin/branch
    let hash_output = Command::new("git")
        .arg("log")
        .arg(&format!("origin/{}", branch))
        .arg("-n")
        .arg("1")
        .arg("--pretty=format:%H")
        .current_dir(repo_path)
        .output()
        .expect("Failed to get commit hash");
    
    let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
        .trim()
        .to_string();

    commit_hash
}

fn hash_trace_folder(suite_name: &str, version: &str) -> u64 {
    let hash_str = format!("{suite_name} {version}");
    let mut hasher = FxHasher::default();
    hash_str.hash(&mut hasher);
    hasher.finish()
}
