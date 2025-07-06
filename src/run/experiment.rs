use serde::{Serialize, Deserialize};
use std::path::Path;

use super::benchmark_suite::BenchmarkSuite;
use super::status::JobStatus;
use super::db::ExperimentsDataBase;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Experiment {
    pub benchmark_suites: Vec<BenchmarkSuite>
}

impl Experiment {
    // pub fn keep_tasks(&mut self, statuses: &[JobStatus]) {
    //     for benchmark_suite in &mut self.benchmark_suites{
    //         benchmark_suite.keep_tasks(statuses);
    //     }
    //     self.benchmark_suites.retain(|benchmark_suite| benchmark_suite.simulator_parameters.len() > 0);
    // }
    

    pub fn keep_state(&mut self, db: &ExperimentsDataBase, statuses: &[JobStatus], include_none: &bool) -> bool {
        self.benchmark_suites.retain_mut(|bench_suite| bench_suite.keep_state(&db, &statuses, &include_none));
        self.benchmark_suites.len() > 0
    }

    pub fn for_each_run_path<F>(&self, mut f: F)
    where
        F: FnMut(&Path),
    {
        for suite in &self.benchmark_suites {
            suite.for_each_run_path(&mut f);
        }
    }

    // pub fn change_state_benchmarks(&mut self, old_state: &Option<JobStatus>, new_state: &JobStatus){
    //     for benchmark_suite in &mut self.benchmark_suites{
    //         benchmark_suite.change_state_benchmarks(old_state, new_state);
    //     }
    // }
}


