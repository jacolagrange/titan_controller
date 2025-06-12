use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write as IoWrite;

use crate::constants::SNIPER_ARGUMENT_FILE_NAME;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Arguments {
    pub job_arguments: Vec<JobArgument>
}

impl Arguments {
    pub fn keep_tasks(&mut self, statuses: &[JobStatus]) {
        for job_argument in &mut self.job_arguments{
            job_argument.keep_tasks(statuses);
        }
        self.job_arguments.retain(|job_argument| job_argument.experiment_arguments.len() > 0);
    }

    pub fn change_state_benchmarks(&mut self, old_state: &Option<JobStatus>, new_state: &JobStatus){
        for job_argument in &mut self.job_arguments{
            job_argument.change_state_benchmarks(old_state, new_state);
        }
    }
}

/*
 * A Job is a the whole collections of all the different experiments (There is one job / benchmark
 * sutie, because of different mounting and git requirements.)
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobArgument {
    pub suite: String,
    pub meta_arguments: HashMap<String, String>,
    pub experiment_arguments: Vec<ExperimentArgument>,
    pub host_dst_path: PathBuf,
    pub job_nr: Option<String>
}

impl JobArgument {
    pub fn prepare_host_directories(&self) -> std::io::Result<()> {
        for experiment_argument in &self.experiment_arguments {
            experiment_argument.set_up_host_dir(&self.host_dst_path)?;
        }
        Ok(())
    }

    pub fn get_number_task(&self) -> usize {
        let mut tasks = 0;
        for experiment_argument in &self.experiment_arguments {
            tasks += experiment_argument.get_number_task();
        }
        tasks
    }

    pub fn keep_tasks(&mut self, statuses: &[JobStatus]){
        for experiment_argument in &mut self.experiment_arguments{
            experiment_argument.keep_task(statuses);
        }
        self.experiment_arguments.retain(|experiment_argument| experiment_argument.benchmarks.len() > 0);
    }

    pub fn change_state_benchmarks(&mut self, old_state: &Option<JobStatus>, new_state: &JobStatus){
        for experiment_argument in &mut self.experiment_arguments{
            experiment_argument.change_state_benchmarks(old_state, new_state);
        }
    }
}

/*
 * An experiment is defined for a given (sniper) configuration with defined parameters
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExperimentArgument{
    pub sniper_dir_name: String,
    pub variable_sniper_parameters: HashMap<String, String>,
    pub benchmarks: Vec<BenchmarkArgument>,
}

impl ExperimentArgument {
    pub fn set_up_host_dir(&self, parent_path: &PathBuf) -> std::io::Result<()> {
        let exp_meta_info_path = parent_path.join(&self.sniper_dir_name);
        if ! (exp_meta_info_path.exists() && exp_meta_info_path.is_dir()) {
            std::fs::create_dir_all(&exp_meta_info_path)?;
        }
        self.create_host_argument_file(&exp_meta_info_path)?;

        for benchmark_argument in &self.benchmarks {
            benchmark_argument.set_up_benchmark_host_dir(&exp_meta_info_path)?;
        }
        Ok(())
    }

    fn create_host_argument_file(&self, dst_path: &PathBuf) -> std::io::Result<()>{
        let file = File::create(dst_path.join(SNIPER_ARGUMENT_FILE_NAME))?;
        let mut writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(&mut writer, &self.variable_sniper_parameters)?;
        writer.flush()?;
        Ok(())
    }

    pub fn get_number_task(&self) -> usize {
        self.benchmarks.len()
    }

    pub fn keep_task(&mut self, statuses: &[JobStatus]){
        self.benchmarks.retain(|benchmark| statuses.contains(&benchmark.status));
    }

    pub fn change_state_benchmarks(&mut self, old_state: &Option<JobStatus>, new_state: &JobStatus){
        for benchmark in &mut self.benchmarks{
            if old_state.is_none() || Some(&benchmark.status) == old_state.as_ref() {
                benchmark.status = new_state.clone();
            }
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JobStatus {
    TOSUBMIT,
    SUBMITTED,
    DONE,
    FAILED,
    RETRIED
}

/*
 * A Benchmark is the based on the Experiment, all the inputs it needs to be run with
 */ 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkArgument{
    pub arguments: HashMap<String, String>,
    pub benchmark_name: String,
    pub run_idx: usize,

    pub task_idx: Option<usize>,
    pub status: JobStatus
}

impl BenchmarkArgument {
    pub fn set_up_benchmark_host_dir(&self, dst_path: &PathBuf) -> std::io::Result<()> {
        let benchmark_path = dst_path.join(&self.benchmark_name).join(&self.run_idx.to_string());
        if ! (benchmark_path.exists() && benchmark_path.is_dir()) {
            std::fs::create_dir_all(&benchmark_path)?;
        }
        Ok(())
    }
}


