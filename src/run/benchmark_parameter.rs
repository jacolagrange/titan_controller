use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::status::JobStatus;

/*
 * A Benchmark is the based on the Experiment, all the inputs it needs to be run with
 */ 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkParameter{
    pub arguments: HashMap<String, String>,
    pub benchmark_name: String,
    pub run_idx: usize,

    pub task_idx: Option<usize>,
    pub status: JobStatus
}

impl BenchmarkParameter {
    pub fn set_up_benchmark_host_dir(&self, dst_path: &PathBuf) -> std::io::Result<()> {
        let benchmark_path = dst_path.join(&self.benchmark_name).join(&self.run_idx.to_string());
        if ! (benchmark_path.exists() && benchmark_path.is_dir()) {
            std::fs::create_dir_all(&benchmark_path)?;
        }
        Ok(())
    }
}
