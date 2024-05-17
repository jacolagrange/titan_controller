use crate::communication::ssh;
use crate::credentials::Credentials;
use crate::fill_template::fill_template;
use crate::experiments::{ParseExperiment, JobArgument, ExperimentArgument, BenchmarkArgument};

use std::str::FromStr;
use std::fs;
use std::path::Path;
use regex::Regex;
use lazy_static::lazy_static;
use time::{OffsetDateTime, Duration, format_description};

#[derive(Debug)]
struct TitanJobStat {
    job_id: usize,
    name: String,
    account: String,
    cores: usize,
    time: String,
    state: String
}

pub struct JobHandler{
    creds: Credentials,
    all: bool,
    completed: Option<usize> //to list the completed jobs in the last x days
}

impl JobHandler{
    pub fn new(creds: Credentials, all: bool, completed: Option<usize>) -> Self {
        JobHandler{creds, all, completed}
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
                job_id: usize::from_str(inputs[0]).unwrap(),
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

    pub fn submit_job(&self) {
        //Create some tmp folder to put the scripts into
        let temp_path = Path::new("/tmp/titan_controller");
        if ! temp_path.is_dir(){
            let _ = fs::create_dir(temp_path);
        }

        //Get experiments parameters
        let experiment_path = Path::new("script-template/experiment_template.json");
        let benchmark_path = Path::new("script-template/benchmark_template.json");
        let experiment = ParseExperiment::new(&experiment_path, &benchmark_path);
        let repl_maps = experiment.get_arguments();

        let template_job = Path::new("script-template/job.sh");
        let titan_path = Path::new("/home/slurmslave/jobs/submitted/.");

        //Obtain a unique hash from the server
        let mut hashes = ssh::get_hash_titan(repl_maps.len()).into_iter();

        for JobArgument {mut meta_arguments, experiment_arguments} in repl_maps {
            meta_arguments.insert(String::from("<ACCOUNT>"), self.creds.username.clone());
            let hash = hashes.next().unwrap();

            //Create job file
            let job_file_path_str = format!("job_{hash}.sh");
            let job_file_path = Path::new(&job_file_path_str);
            let dst_job_path = temp_path.join(&job_file_path);
            fill_template(&template_job, &dst_job_path , &meta_arguments);
            let titan_job_path = titan_path.join(&job_file_path);
            let _ = ssh::send_files(&dst_job_path.to_str().unwrap(), &titan_job_path.to_str().unwrap());

            //TODO remove comment for debug
            //let (stdout, stderr) = ssh::send_command(&format!("sbatch {}", titan_job_path.to_str().unwrap()));
            let stdout = String::from("Submitted batch job 12345"); //TODO remove this
            let stderr = String::new();
            //Sbatch is send, the job-file is not needed anymore
            //TODO remove comment for debug
            //let _ = ssh::send_command(&format!("rm {}", titan_job_path.to_str().unwrap()));
            
            let job_nr = stdout.split("\n").next().unwrap().split(" ").last().unwrap();

            if stdout.contains("Submitted batch job") {
                self.submit_experiment(&experiment_arguments, &temp_path, job_nr);
                println!("Submitted job {} (jobid {})", &meta_arguments["<JOB>"], job_nr);
            } else {
                eprintln!("Job submission did not produce a job-nr \nOutput:\n{stdout}\n\nErr:\n{stderr}");
            }
        }
    }

    fn submit_experiment(&self, experiment_arguments: &Vec<ExperimentArgument>, destination: &Path, job_nr: &str) {
        let template_vm = Path::new("script-template/execute_Sniper.sh");
        let temp_exp_path = destination.join(Path::new(&job_nr));
        if ! temp_exp_path.is_dir() { let _ = fs::create_dir(&temp_exp_path); }

        let mut task_id = 1;
        for ExperimentArgument{exp_meta_info_path, variable_sniper_parameters, benchmarks} in experiment_arguments{
            for BenchmarkArgument{arguments, benchmark_name, run_idx} in benchmarks {

                let exp_file_path_str = format!("execute_{job_nr}_{task_id}.sh");
                let exp_file_path = Path::new(&exp_file_path_str);
                let dst_exp_path = temp_exp_path.join(&exp_file_path);
                fill_template(&template_vm, &dst_exp_path, &arguments);

                task_id += 1;
            }
        }

        //TODO remove comment for debug
        //let _ = ssh::send_files(&temp_exp_path.join(Path::new("*")).to_str().unwrap(), &titan_path.to_str().unwrap());
    }
}
