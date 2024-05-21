use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;

use crate::experiments::ExperimentArgument;

#[derive(Serialize, Deserialize, Debug)]
pub struct JobDataPoint{
    job_nr: String,
    benchmark_suite: String,
    arguments: ExperimentArgument
}

struct JobData {
    result_path: PathBuf
}

impl JobData {
    pub fn new(result_path: &Path) -> Self {
        JobData {result_path: result_path.to_path_buf()}
    }

    pub fn write_data(&self, job_data_points: Vec<JobDataPoint>) {
        let result_file = File::create(&self.result_path).unwrap();
        let _ = serde_json::to_writer_pretty(&result_file, &job_data_points);
        drop(result_file);
    }

    pub fn get_data(&self) -> Option<Vec<JobDataPoint>> {
        let result_file = File::open(&self.result_path).unwrap();
        let data = serde_json::from_reader(&result_file);
        drop(result_file);
        return data.ok();
    }
}
