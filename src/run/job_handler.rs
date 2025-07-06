use std::path::{Path, PathBuf};
use std::io::Write;
use std::fs::File;
use std::fs;

use super::status::JobStatus;
use super::parse_experiment::ParseExperiment;
use super::test_job;
use super::db::ExperimentsDataBase;
use crate::hpc::slurm_handle;
use crate::constants::{TEMP_FOLDER_NAME, EXECUTE_TEMPLATE, EXPERIMENT_DB_NAME };
use crate::communication::ssh;
use crate::run::{
    experiment::Experiment,
    benchmark_suite::BenchmarkSuite,
    benchmark_parameter::BenchmarkParameter,
    parse_experiment
};
use crate::utils::fill_template::fill_template;

pub struct JobHandler {
    //TODO use a trait for hpc-handler
    hpc_handler: slurm_handle::SlurmHandler,
    temp_path: PathBuf

}

impl JobHandler{
    pub fn new(hpc_handler: slurm_handle::SlurmHandler) -> Self {
        let temp_path = Path::new(TEMP_FOLDER_NAME).to_path_buf();
        if ! temp_path.is_dir(){
            let _ = fs::create_dir(&temp_path);
        }
        JobHandler{hpc_handler, temp_path}
    }

    pub fn submit_jobs(&self, experiment_path: &str, benchmark_path: &str, dry_run: &bool) -> Result<(), std::io::Error> {
        //Get experiments parameters
        let experiment_path = Path::new(experiment_path);
        let benchmark_path = Path::new(benchmark_path);
        let parser = ParseExperiment::new(&experiment_path, &benchmark_path);
        let mut experiment = parser.get_arguments();

        let mut cur_db = match ExperimentsDataBase::from_cache() {
            Ok(Some(db)) => db,
            _ => ExperimentsDataBase::new(),
        };

        //remove existing benchmarks
        if experiment.is_done(&cur_db) {
            println!("Experiment is already fully done, nothing to do... bye");
            return Ok(());
        }

        if ! dry_run {
            //Obtain a unique hash from the server
            let mut hashes = ssh::get_hash_titan(experiment.benchmark_suites.len())?.into_iter();

            for benchmark_suite in &mut experiment.benchmark_suites {
                self.submit_one_job(benchmark_suite, &hashes.next().unwrap(), &mut cur_db)?;
            }
            cur_db.set_experiment_status(&experiment, &JobStatus::SUBMITTED)
        }

        if let Err(e) = cur_db.save_to_cache() { eprintln!("An error happened, when writing down the cache {}", e);}

        let dst = parser.get_exp_dst();
        let _ = write_submit_job_map(&experiment, &dst);

        Ok(())
    }

    fn submit_one_job(&self, benchmark_suite: &mut BenchmarkSuite, hash: &str, cur_db: &mut ExperimentsDataBase) -> Result<(), std::io::Error> {
        //Create the job file first
        let job_file = format!("job_{hash}.sh");
        let job_file_path = self.temp_path.join(Path::new(&job_file));
        let _ = self.hpc_handler.create_job_file(benchmark_suite, &job_file_path).expect("Failed to create job file for suite {benchmark_suite.suite}");

        if let Some(job_nr) = self.hpc_handler.submit_job(&job_file_path)? {
            //Submission job suceeeded
            //Now create the experiment files
            let exp_dir = self.create_job_files(benchmark_suite, &job_nr, cur_db)?;
            self.hpc_handler.submit_experiment(&exp_dir);
            cur_db.set_bench_suite_job_id(&benchmark_suite, &job_nr, &Some(JobStatus::SUBMITTED));
            println!("Submitted job {} (jobid {})", &benchmark_suite.meta_arguments["<JOB>"], &job_nr);
        } else {
            println!("Not all the jobs could be subitted to titan. Run collect, to retry those jobs later");
        }
        Ok(())
    }

    //This file runs completely inside the VM, so is independent on the server infrastructure
    fn create_job_files(&self, job_argument: &mut BenchmarkSuite, job_nr: &str, cur_db: &mut ExperimentsDataBase) -> Result<PathBuf, std::io::Error> {
        let temp_exp_path = self.temp_path.join(Path::new(&job_nr));
        if ! temp_exp_path.is_dir() { let _ = fs::create_dir(&temp_exp_path); }

        let mut task_idx = 1;
        let bench_suite_dir = &job_argument.host_dst_path;
        for sim_param in &mut job_argument.simulator_parameters{
            let sim_dir = sim_param.get_dir(&bench_suite_dir);
            for bench_param in &mut sim_param.benchmark_parameters {
                let bench_param_dir = bench_param.get_dir(&sim_dir);
                for bench_run in &mut bench_param.benchmark_runs {
                    let bench_run_dir = bench_run.get_dir(&bench_param_dir);

                    let exp_file_path_str = format!("execute_{job_nr}_{task_idx}.sh");
                    let exp_file_path = Path::new(&exp_file_path_str);
                    let dst_exp_path = temp_exp_path.join(&exp_file_path);
                    fill_template(EXECUTE_TEMPLATE.to_owned(), &dst_exp_path, &bench_param.arguments);

                    cur_db.set_task_id(&bench_run_dir, &task_idx);

                    task_idx += 1;
                }
            }
        }
        Ok(temp_exp_path)
    }

    pub fn collect_jobs(&mut self, experiment_map_file: &Path) -> Result<(), std::io::Error> {
        //TODO
        //let mut experiment = parse_experiment::get_exp_map(&experiment_map_file).unwrap();

        //let titan_job_stats = self.hpc_handler.get_jobs_retry(false, None)?;
        //let titan_job_ids: Vec<String> = titan_job_stats.iter().map(|stat| stat.get_job_id()).collect();

        //for benchmark_suite in &mut experiment.benchmark_suites {
        //    for simulator_parameter in &mut benchmark_suite.simulator_parameters {
        //        for benchmark_parameter in &mut simulator_parameter.benchmark_parameters {
        //            if benchmark_parameter.status != JobStatus::SUBMITTED {continue;}
        //            match(benchmark_suite.job_nr.as_ref(), benchmark_parameter.task_idx){
        //                (Some(job_nr), Some(task_idx)) => {
        //                    let bench_job_id = format!("{}_{}", job_nr, task_idx);
        //                    if ! titan_job_ids.contains(&bench_job_id) {
        //                        let dst_path = benchmark_suite.host_dst_path.join(&simulator_parameter.simulator_dir_name);
        //                        let res = self.retrieve_result(benchmark_parameter, &bench_job_id, &dst_path);
        //                        match res {
        //                            Ok(_) => {}
        //                            Err(_) => {
        //                                println!("Could not retreive job_id {}.", &bench_job_id);
        //                                benchmark_parameter.status = JobStatus::FAILED;
        //                            }
        //                        }
        //                    }
        //                }
        //                _ => {
        //                    println!("Could not find the information back about job_id or task_id in the json.");
        //                    benchmark_parameter.status = JobStatus::FAILED;
        //                }
        //            }
        //        }
        //    }
        //}

        //let mut failed_exp = experiment.clone();
        //failed_exp.keep_tasks(&[JobStatus::FAILED, JobStatus::TOSUBMIT]);

        //if failed_exp.benchmark_suites.len() > 0 {
        //    failed_exp.change_state_benchmarks(&Some(JobStatus::FAILED), &JobStatus::TOSUBMIT);
        //    self.retry_experiment(&mut failed_exp)?;

        //    experiment.change_state_benchmarks(&Some(JobStatus::FAILED), &JobStatus::RETRIED);
        //    experiment.change_state_benchmarks(&Some(JobStatus::TOSUBMIT), &JobStatus::RETRIED);
        //    experiment.benchmark_suites.append(&mut failed_exp.benchmark_suites);
        //}
        //let _ = write_submit_job_map(&experiment, &experiment_map_file);
        Ok(())
    }

    fn retrieve_result(&self, benchmark_parameter: &mut BenchmarkParameter, bench_job_id: &str, host_dst_path: &Path) -> Result<(), std::io::Error> {
     //   let result_file = format!("results_{bench_job_id}.tar.gz");
     //   let src_path = format!("/home/slurmslave/results/{result_file}");
     //   let tar_file_path = self.temp_path.join(&result_file);
     //   let dst_path = host_dst_path.join(&benchmark_parameter.benchmark_name).join(&benchmark_parameter.run_idx.to_string());
     //   
     //   let _ = ssh::get_files(&src_path, self.temp_path.to_str().unwrap())?;
     //   let _ = ssh::untar(&tar_file_path, &dst_path, true)?;
     //   
     //   benchmark_parameter.status = 
     //       if test_job::job_succeed(&dst_path) {
     //           println!("Experiment {bench_job_id} was successfully downloaded");
     //           JobStatus::DONE
     //       } else {
     //           println!("Experiment {bench_job_id} did not pass the tests");
     //           let _ = ssh::clean_dir(&dst_path);
     //           JobStatus::FAILED
     //       };
        Ok(())
    }

    fn retry_experiment(&self, experiment: &mut Experiment) -> Result<(), std::io::Error> {
      //  let mut hashes = ssh::get_hash_titan(experiment.benchmark_suites.len())?.into_iter();
      //  println!("Failed or not yet submitted experiment detected... retyring/sending them.");

      //  for benchmark_suite in &mut experiment.benchmark_suites {
      //      self.submit_one_job(benchmark_suite, &hashes.next().unwrap())?;
      //  }
        Ok(())
    }
}

pub fn write_submit_job_map(experiment: &Experiment, host_dst_path: &Path) -> std::io::Result<()> {
    let _ = set_up_host_dir(host_dst_path);
    let file_path = host_dst_path.join(EXPERIMENT_DB_NAME);

    let file = File::create(file_path)?;
    let mut writer = std::io::BufWriter::new(file);
    let _ = serde_json::to_writer_pretty(&mut writer, experiment)?;
    writer.flush()?;
    Ok(())
}

pub fn set_up_host_dir(host_dst_path: &Path) -> Result<(), std::io::Error> {
        if ! (host_dst_path.exists() && host_dst_path.is_dir()) {
            std::fs::create_dir_all(&host_dst_path)?;
        }
        Ok(())
}
