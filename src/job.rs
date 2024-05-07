use crate::communication::ssh;
use crate::credentials::Credentials;
use crate::fill_template::fill_template;
use crate::experiments::Experiment;

use std::str::FromStr;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use lazy_static::lazy_static;
use time::{OffsetDateTime, Duration, format_description};
use std::collections::HashMap;

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
        JobHandler{
            creds: creds,
            all: all,
            completed: completed
        }
    }

    fn get_jobs(&self) -> Vec<TitanJobStat> {
        let (stdout, skip_nr) = 
        if let Some(days) = self.completed {
            lazy_static! {
                static ref spaces: Regex = Regex::new(" +").unwrap();
            }
            let mut command = String::from("sacct");
            if !self.all {
                command += &(" --account=".to_owned() + &self.creds.username);
            }
            let start_time = OffsetDateTime::now_utc().checked_sub(Duration::days(days as i64)).unwrap();
            let sacct_time_format = format_description::parse("[year]-[month]-[day]").unwrap();
            command += &format!(" -S {} --format=JobID,JobName%150,Account,NCPUS,Submit,State%30", start_time.format(&sacct_time_format).unwrap());
            let (mut stdout, stderr) = ssh::send_command(&command);
            stdout = spaces.replace_all(&stdout, r" ").to_string();
            (stdout, 2)
        } else {
            let mut command = String::from("squeue");
            if !self.all {
                command += &(" -A ".to_owned() + &self.creds.username);
            }
            command += " -o \"%i %j %a %C %M %R\"";
            let (stdout, stderr) = ssh::send_command(&command);
            (stdout, 1)
       };
    
        let mut output = stdout.split("\n").skip(skip_nr);
    
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
            fs::create_dir(temp_path);
        }

        //Obtain a unique hash from the server
        let hash = &ssh::get_hash_titan(1)[0];

        //Get experiments parameters
        let experiment_path = Path::new("script-template/experiment_template.json");
        let benchmark_path = Path::new("script-template/benchmark_template.json");
        let mut experiment = Experiment::new(&experiment_path, &benchmark_path);
        let mut repl_map = experiment.get_job_arguments();
        repl_map.insert("<ACCOUNT>", self.creds.username.clone());

        //Create job file
        let template_job = Path::new("script-template/job.sh");
        let job_file_name = String::from(format!("job_{hash}.sh"));
        let job_file_path = Path::new(&job_file_name);
        let dst_job_path = temp_path.join(&job_file_path);
        fill_template(&template_job, &dst_job_path , &repl_map);
        let titan_path = Path::new("/home/slurmslave/jobs/submitted/.").join(&job_file_path);
        ssh::send_files(&dst_job_path.to_str().unwrap(), &titan_path.to_str().unwrap());
        let (stdout, stderr) = ssh::send_command(&format!("sbatch {}", titan_path.to_str().unwrap()));

        if stdout.contains("Submitted batch job") {
            println!("Job submission produced \nOutput:\n{stdout}\n\nErr:\n{stderr}");
            let job_nr = stdout.split("\n").next().unwrap().split(" ").last().unwrap();

            let temp_exp_path = temp_path.join(Path::new(&job_nr));
            if ! temp_exp_path.is_dir() { fs::create_dir(&temp_exp_path); }

            let template_vm = Path::new("script-template/execute_Sniper.sh");
            let exp_map_list = experiment.get_exp_arguments();
            for (idx, exp_map) in exp_map_list.iter().enumerate() {
                let task_id = idx + 1;
                let exp_name = String::from(format!("execute_{job_nr}_{task_id}.sh"));
                let exp_file_path = Path::new(&exp_name);
                let dst_exp_path = temp_exp_path.join(&exp_file_path);
                fill_template(&template_vm, &dst_exp_path, &exp_map);
            }
            ssh::send_files(&temp_exp_path.to_str().unwrap(), &titan_path.to_str().unwrap());
        } else {
            eprintln!("Job submission did not produce a job-nr \nOutput:\n{stdout}\n\nErr:\n{stderr}");
        }
    }

    // pub fn submit_job(&self, scripts_path: Vec<&Path>) -> String{
    //     //Create some tmp folder to put the scripts into
    //     let temp_path = Path::new("/tmp/titan_controller");
    //     if ! temp_path.is_dir(){
    //         fs::create_dir(temp_path);
    //     }

    //     //obtain a unique hash from the server
    //     let hash = &ssh::get_hash_titan(1)[0];

    //     //copy the scripts into the tmp folder with the hash as name inside
    //     let mut tmp_scripts = Vec::<Box<PathBuf>>::new();
    //     for script in scripts_path {
    //         let dst_name_str = format!("{}_{}.{}", script.file_stem().unwrap().to_str().unwrap(), hash, script.extension().unwrap().to_str().unwrap());
    //         let dst_name = Path::new(&dst_name_str);
    //         let dst_path = temp_path.join(dst_name);
    //         fs::copy(&script, &dst_path);
    //         tmp_scripts.push(Box::new(dst_path));
    //     }

    //     //upload all the files to titan
    //     //TODO replace this by submit in bulk: zip -> send -> unzip
    //     for script in &tmp_scripts {
    //         ssh::send_files(script.to_str().unwrap(), "/home/slurmslave/jobs/uploaded/.");
    //     }

    //     //run the script on titan
    //     let (stdout, stderr) = ssh::send_command(&format!("sbatch {}", &tmp_scripts[0].to_str().unwrap()));

    //     //Get the jobid from this job
    //     let jobid = 
    //         if let Some(line) = stdout.split("\n").next() {
    //             if let Some(jobid_str) = line.split(" ").last() {
    //                 jobid_str.to_string()
    //             } else {
    //                 println!("Something went wrong at sbatch\n stdout: {} \n stderr: {}", stdout, stderr);
    //                 String::from("-1")
    //             }
    //         } else {
    //             println!("Something went wrong at sbatch\n stdout: {} \n stderr: {}", stdout, stderr);
    //             String::from("-1")
    //         };
    //     return jobid;
    // }
}

// pub struct TitanJobMetaData {
//     cores: usize,
//     memory: Option<usize>,
//     vm_name: String,
//     localSaveDir: Path,
// }
// 
// struct SniperJob {
//     stats: TitanJobStat,
//     benchmark_branch: Option<String>,
//     benchmark_exports: Option<Vec<String>>,
//     benchmark_make_cmd: Option<Vec<String>>,
//     benchmark_cmd: Option<Vec<String>>,
//     sniper_branch: Option<String>,
//     sniper_arguments: Option<Vec<String>>,
//     meta: &TitanJobMetaData,
// }
// 
// impl SniperJob {
//     pub fn new(stats: TitanJobStat, 
//                benchmark_branch: Option<String>, 
//                benchmark_exports: Option<Vec<String>>, 
//                benchmark_make_cmd: Option<Vec<String>>, 
//                benchmark_cmd: Option<Vec<String>>, 
//                sniper_branch: Option<String>, 
//                sniper_arguments: Option<Vec<String>>, 
//                meta: &TitanJobMetaData) -> Self {
//         SniperJob{
//             stats: stats,
//             benchmark_exports: benchmark_exports,
//             benchmark_make_cmd: benchmark_make_cmd,
//             benchmark_cmd: benchmark_cmd,
//             sniper_arguments: sniper_arguments,
//             sniper_branch: sniper_branch,
//             benchmark_branch, benchmark_branch,
//             meta: meta
//         }
//     }
// }

//slurmslave@bacchus:~/jobs/uploaded$ sbatch job_test.sh
//Submitted batch job 265808

