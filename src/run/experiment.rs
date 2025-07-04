use serde::{Serialize, Deserialize};

use super::benchmark_suite::BenchmarkSuite;
use super::status::JobStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Experiment {
    pub benchmark_suites: Vec<BenchmarkSuite>
}

impl Experiment {
    pub fn keep_tasks(&mut self, statuses: &[JobStatus]) {
        for benchmark_suite in &mut self.benchmark_suites{
            benchmark_suite.keep_tasks(statuses);
        }
        self.benchmark_suites.retain(|benchmark_suite| benchmark_suite.simulator_parameters.len() > 0);
    }

    pub fn change_state_benchmarks(&mut self, old_state: &Option<JobStatus>, new_state: &JobStatus){
        for benchmark_suite in &mut self.benchmark_suites{
            benchmark_suite.change_state_benchmarks(old_state, new_state);
        }
    }
}


