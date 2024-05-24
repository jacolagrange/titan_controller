use crate::communication::ssh;
use crate::credentials::Credentials;
use crate::fill_template::fill_template;
use crate::experiments::{ParseExperiment, get_job_map, write_submit_job_map};
use crate::job_data::{Arguments, JobArgument, ExperimentArgument, JobStatus, BenchmarkArgument};
use crate::test_job;
use crate::constants::{JOB_TEMPLATE, EXECUTE_TEMPLATE, TEMP_FOLDER_NAME, TITAN_SUBMIT_DIR};

use std::str::FromStr;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use lazy_static::lazy_static;
use time::{OffsetDateTime, Duration, format_description};

#[derive(Debug)]
struct TitanJobStat {
    job_id: String,
    name: String,
    account: String,
    cores: usize,
    time: String,
    state: String
}

pub struct JobHandler{
    creds: Credentials,
    all: bool,
    completed: Option<usize>, //to list the completed jobs in the last x days
    temp_path: PathBuf
}

impl JobHandler{
    pub fn new(creds: Credentials, all: bool, completed: Option<usize>) -> Self {
        let temp_path = Path::new(TEMP_FOLDER_NAME).to_path_buf();
        if ! temp_path.is_dir(){
            let _ = fs::create_dir(&temp_path);
        }

        JobHandler{creds, all, completed, temp_path}
    }

    fn get_jobs(&self) -> Vec<TitanJobStat> {
        let (stdout, skip_nr) = 
        if let Some(days) = self.completed {
            lazy_static! {
                static ref SPACES: Regex = Regex::new(" +").unwrap();
            }
            let mut command = String::from("sacct");
            if !self.all {
                command += &(" --account=".to_owned() + &self.creds.username);
            }
            let start_time = OffsetDateTime::now_utc().checked_sub(Duration::days(days as i64)).unwrap();
            let sacct_time_format = format_description::parse("[year]-[month]-[day]").unwrap();
            command += &format!(" -S {} --format=JobID,JobName%150,Account,NCPUS,Submit,State%30", start_time.format(&sacct_time_format).unwrap());
            let (mut stdout, _stderr) = ssh::send_command(&command);
            stdout = SPACES.replace_all(&stdout, r" ").to_string();
            (stdout, 2)
        } else {
            let mut command = String::from("squeue");
            if !self.all {
                command += &(" -A ".to_owned() + &self.creds.username);
            }
            command += " -o \"%i %j %a %C %M %R\"";
            let (stdout, _stderr) = ssh::send_command(&command);
            (stdout, 1)
        };
    
        let output = stdout.split("\n").skip(skip_nr);
    
        let mut jobs = Vec::new();
        for line in output {
            let inputs: Vec<&str> = line.split(" ").collect();
            if inputs.len() < 5 || inputs[0].contains(".") {continue;}
            let t = TitanJobStat{
                job_id: String::from(inputs[0]),
                name: String::from(inputs[1]),
                account: String::from(inputs[2]),
                cores: usize::from_str(inputs[3]).unwrap(),
                time: String::from(inputs[4]),
                state: String::from(inputs[5])
            };
            jobs.push(t);
        }
        jobs
    }

    pub fn print_jobs(&self) {
        let jobs = self.get_jobs();
        println!("{:<10} {:<100} {:<15} {:<5} {:<20} {:<15}", "JOBID", "NAME", "ACCOUNT", "CORES", "TIME", "STATE");
        println!("{:-<10} {:-<100} {:-<15} {:-<5} {:-<20} {:-<15}", "", "", "", "", "", "");
        for job in jobs{
            println!("{:<10} {:<100} {:<15} {:<5} {:<20} {:<15}", job.job_id, job.name, job.account, job.cores, job.time, job.state);
        }
    }


    pub fn delete_jobs(&self, job_ids: Vec<usize>){
        for job_id in job_ids {
            let (stdout, stderr) = ssh::send_command(&format!("/home/slurmadmin/scripts/delete_job.py -j {} -u {} -p {}", job_id, self.creds.username, self.creds.password));
            if stdout.len() > 0 {println!("{}", stdout);}
            if stderr.len() > 0 {println!("{}", stderr);}
        }
    }

    pub fn submit_job(&self, experiment_path: &str, benchmark_path: &str) {
        //Get experiments parameters
        let experiment_path = Path::new(experiment_path);
        let benchmark_path = Path::new(benchmark_path);
        let experiment = ParseExperiment::new(&experiment_path, &benchmark_path);
        let mut repl_maps = experiment.get_arguments();


        //Obtain a unique hash from the server
        let mut hashes = ssh::get_hash_titan(repl_maps.job_arguments.len()).into_iter();

        for job_argument in &mut repl_maps.job_arguments {
            self.submit_one_job(job_argument, &hashes.next().unwrap());
        }
        let _ = experiment.create_submit_job_map(&repl_maps);
    }

    fn submit_one_job(&self, job_argument: &mut JobArgument, hash: &str) {
        let titan_path = Path::new(TITAN_SUBMIT_DIR);
        let _ = job_argument.prepare_host_directories();

        let nr_tasks = job_argument.get_number_task();
        //let meta_arguments = &mut job_argument.meta_arguments;
        if ! job_argument.meta_arguments.contains_key("<TASKS>") {
            job_argument.meta_arguments.insert(String::from("<TASKS>"), nr_tasks.to_string());
        } else {
            *job_argument.meta_arguments.get_mut("<TASKS>").unwrap() = nr_tasks.to_string();
        }

        if ! job_argument.meta_arguments.contains_key("<ACCOUNT>") {
            job_argument.meta_arguments.insert(String::from("<ACCOUNT>"), self.creds.username.clone());
        }

        //Create job file
        let job_file_path_str = format!("job_{hash}.sh");
        let job_file_path = Path::new(&job_file_path_str);
        let dst_job_path = self.temp_path.join(&job_file_path);
        fill_template(JOB_TEMPLATE.to_owned(), &dst_job_path , &job_argument.meta_arguments);
        let titan_job_path = titan_path.join(&job_file_path);
        let _ = ssh::send_files(&dst_job_path.to_str().unwrap(), &titan_job_path.to_str().unwrap());

        let (stdout, stderr) = ssh::send_command(&format!("sbatch {}", titan_job_path.to_str().unwrap()));
        //Sbatch is send, the job-file is not needed anymore
        let _ = ssh::send_command(&format!("rm {}", titan_job_path.to_str().unwrap()));
        
        let job_nr = stdout.split("\n").next().unwrap().split(" ").last().unwrap();

        if stdout.contains("Submitted batch job") {
            self.submit_experiment(job_argument, job_nr);
            job_argument.job_nr = Some(job_nr.to_owned());
            println!("Submitted job {} (jobid {})", &job_argument.meta_arguments["<JOB>"], job_nr);
        } else {
            eprintln!("Job submission did not produce a job-nr \nOutput:\n{stdout}\n\nErr:\n{stderr}");
        }
    }

    /*
     * Takes a vecotr of experiments, and apply the template to them and finally sends all the
     * files to Titan.
     */
    fn submit_experiment(&self, job_argument: &mut JobArgument, job_nr: &str) {
        let titan_path = Path::new(TITAN_SUBMIT_DIR);
        let temp_exp_path = self.temp_path.join(Path::new(&job_nr));
        if ! temp_exp_path.is_dir() { let _ = fs::create_dir(&temp_exp_path); }

        let mut task_idx = 1;
        for ExperimentArgument{sniper_dir_name: _, variable_sniper_parameters: _, benchmarks} in &mut job_argument.experiment_arguments{
            for BenchmarkArgument{arguments, benchmark_name: _, run_idx: _, task_idx: benchmark_task_idx, status: _} in benchmarks {
                let exp_file_path_str = format!("execute_{job_nr}_{task_idx}.sh");
                let exp_file_path = Path::new(&exp_file_path_str);
                let dst_exp_path = temp_exp_path.join(&exp_file_path);
                fill_template(EXECUTE_TEMPLATE.to_owned(), &dst_exp_path, &arguments);
                *benchmark_task_idx = Some(task_idx);
                task_idx += 1;
            }
        }
        let _ = ssh::send_files(&temp_exp_path.join(Path::new("*")).to_str().unwrap(), &titan_path.to_str().unwrap());
    }

    pub fn collect_jobs(&mut self, experiment_map_file: &Path) {
        let mut jobs = get_job_map(&experiment_map_file).unwrap();

        self.all = false;
        self.completed = None;
        let titan_job_stats = self.get_jobs();
        let titan_job_ids: Vec<String> = titan_job_stats.iter().map(|stat| stat.job_id.clone()).collect();

        for job_argument in &mut jobs.job_arguments {
            for experiment_argument in &mut job_argument.experiment_arguments {
                for benchmark_argument in &mut experiment_argument.benchmarks {
                    if benchmark_argument.status != JobStatus::SUBMITTED {continue;}
                    let bench_job_id = format!("{}_{}", job_argument.job_nr.as_ref().unwrap(), benchmark_argument.task_idx.unwrap());
                    if ! titan_job_ids.contains(&bench_job_id) {
                        let dst_path = job_argument.host_dst_path.join(&experiment_argument.sniper_dir_name);
                        self.retrieve_result(benchmark_argument, &bench_job_id, &dst_path);
                    }
                }
            }
        }

        let mut failed_jobs = jobs.clone();
        failed_jobs.keep_failed_task();

        if failed_jobs.job_arguments.len() > 0 {
            self.retry_jobs(&mut failed_jobs);
            failed_jobs.change_state_benchmarks(&JobStatus::FAILED, &JobStatus::SUBMITTED);

            jobs.change_state_benchmarks(&JobStatus::FAILED, &JobStatus::RETRIED);
            jobs.job_arguments.append(&mut failed_jobs.job_arguments);
        }
        let _ = write_submit_job_map(&jobs, &experiment_map_file);
    }

    fn retrieve_result(&self, benchmark_argument: &mut BenchmarkArgument, bench_job_id: &str, host_dst_path: &Path) {
        let result_file = format!("results_{bench_job_id}.tar.gz");
        let src_path = format!("/home/slurmslave/results/{result_file}");
        let tar_file_path = self.temp_path.join(&result_file);
        let dst_path = host_dst_path.join(&benchmark_argument.benchmark_name).join(&benchmark_argument.run_idx.to_string());
        
        let _ = ssh::get_files(&src_path, self.temp_path.to_str().unwrap());
        let _ = ssh::untar(&tar_file_path, &dst_path, true);
        
        benchmark_argument.status = 
            if test_job::job_succeed(&dst_path) {
                println!("Experiment {bench_job_id} was successfully downloaded");
                JobStatus::DONE
            } else {
                println!("Experiment {bench_job_id} did not pass the tests");
                let _ = ssh::clean_dir(&dst_path);
                JobStatus::FAILED
            }
    }

    fn retry_jobs(&self, jobs: &mut Arguments) {
        let mut hashes = ssh::get_hash_titan(jobs.job_arguments.len()).into_iter();
        println!("Failed jobs detected... retyring them.");

        for job_argument in &mut jobs.job_arguments {
            self.submit_one_job(job_argument, &hashes.next().unwrap());
        }
    }
}
