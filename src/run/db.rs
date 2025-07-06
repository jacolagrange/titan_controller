use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};

use super::experiment::Experiment;
use super::benchmark_suite::BenchmarkSuite;
use super::status::JobStatus;
use crate::constants::{EXPERIMENT_DB_NAME, CACHE_FOLDER_NAME};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskData {
    job_id: Option<String>,
    task_idx: Option<usize>,
    pub status: JobStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExperimentsDataBase {
    //Location -> TaskData
    tasks: HashMap<PathBuf, TaskData>
}

impl ExperimentsDataBase {
    pub fn new() -> Self {
        Self{tasks: HashMap::new()}
    }
    
    // pub fn from_experiment(experiment: &Experiment) -> Self {
    //     let mut tasks = HashMap::new();
    //     for bench_suite in &experiment.benchmark_suites {
    //         let job_id = &bench_suite.job_id;
    //         let suite_location = bench_suite.host_dst_path.clone();

    //         for sim_param in &bench_suite.simulator_parameters {
    //             let sim_location = suite_location.clone().join(sim_param.simulator_dir_name.clone());

    //             for bench_param in &sim_param.benchmark_parameters {
    //                 let task_idx = bench_param.task_idx;
    //                 let status = bench_param.status.clone();
    //                 let location = bench_param.get_dir(&sim_location);

    //                 tasks.insert(
    //                     location,
    //                     TaskData{job_id: job_id.clone(), task_idx, status}
    //                     );
    //             }
    //         }
    //     }
    //     ExperimentsDataBase{tasks}
    // }

    pub fn from_cache() -> Result<Self, std::io::Error> {
        let cache_path = Self::get_cache_path(); 
        Self::from_file(&cache_path)
    }

    fn from_file(file_path: &Path) -> Result<Self, std::io::Error> {
        if file_path.exists() && file_path.is_file() {
            let file = File::open(&file_path)?;
            let mut reader = std::io::BufReader::new(file);
            let db: Self = serde_json::from_reader(&mut reader)?;
            Ok(db)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No database found at: {}", file_path.to_str().unwrap()),
                ))
        }
    }

    // pub fn update(&mut self, new_db: &Self) {
    //     for (task_path, new_task_data) in &new_db.tasks {
    //         match self.tasks.get(task_path) {
    //             Some(TaskData{status: JobStatus::FAILED, ..}) | None => {
    //                 self.tasks.insert(task_path.clone(), new_task_data.clone());
    //             }
    //             _ => {}
    //         }
    //     }
    // }

    //TODO this is not multi-threaded / multi-program safe.
    pub fn save_to_cache(&self) -> Result<(), std::io::Error> {
        let cache_path = Self::get_cache_path(); 
        self.save_experiment(&cache_path)
    }

    fn save_experiment(&self, file_path: &Path) -> Result<(), std::io::Error> {
        set_up_host_dir(file_path.parent().unwrap())?;

        let file = File::create(file_path)?;
        let mut writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(&mut writer, &self)?;
        writer.flush()?;
        Ok(())
    }

    pub fn get_task_data(&self, task_path: &Path) -> Option<TaskData> {
        self.tasks.get(&task_path.to_path_buf()).cloned()
    }

    pub fn insert(&mut self, loc: &Path, job_id: &str, task_idx: &usize) {
        self.tasks.insert(loc.to_path_buf(), TaskData{
            job_id: Some(job_id.to_string()),
            task_idx: Some(task_idx.to_owned()),
            status: JobStatus::TOSUBMIT
        });
    }

    pub fn get_status(&self, loc: &Path) -> Option<JobStatus> {
        self.tasks.get(&loc.to_path_buf()).and_then(
            |task_data| Some(task_data.status.clone()))
    }

    pub fn set_status(&mut self, loc: &Path, new_status: &JobStatus) {
        if let Some(task_data) = self.tasks.get_mut(loc) {
            task_data.status = new_status.clone();
        }
    }

    fn set_job_id(&mut self, loc: &Path, job_id: &str) {
        if let Some(task_data) = self.tasks.get_mut(loc) {
            task_data.job_id = Some(job_id.to_owned());
        }
    }

    pub fn get_job_task_format(&mut self, loc: &Path) -> Option<String>{
        self.tasks.get(&loc.to_path_buf()).and_then(
            |task_data| {
                if let (Some(job_id), Some(task_idx)) = (&task_data.job_id, &task_data.task_idx) {
                    let bench_job_id = format!("{}_{}", job_id, task_idx);
                    Some(bench_job_id)
                } else {
                    None
                }
            })
    }

    pub fn set_task_id(&mut self, loc: &Path, task_idx: &usize) {
        if let Some(task_data) = self.tasks.get_mut(loc) {
            task_data.task_idx = Some(task_idx.to_owned());
        }
    }

    pub fn set_experiment_status(&mut self, exp: &Experiment, new_status: &JobStatus) {
            exp.for_each_run_path(|path| self.set_status(path, new_status));
    }

    pub fn set_bench_suite_job_id(&mut self, bench_suite: &BenchmarkSuite, job_id: &str, new_status: &Option<JobStatus>) {
        bench_suite.for_each_run_path(&mut |path: &Path| {
            self.set_job_id(&path, job_id);
            if let Some(stat) = new_status {
                self.set_status(&path, stat);
            }
        });
    }

    fn get_cache_path() -> PathBuf {
        Path::new(CACHE_FOLDER_NAME).join(EXPERIMENT_DB_NAME)
    }

    pub fn get_paths(&self) -> HashSet<PathBuf> {
        self.tasks.keys().cloned().collect()
    }
}

pub fn set_up_host_dir(host_dst_path: &Path) -> Result<(), std::io::Error> {
        if ! (host_dst_path.exists() && host_dst_path.is_dir()) {
            std::fs::create_dir_all(&host_dst_path)?;
        }
        Ok(())
}
