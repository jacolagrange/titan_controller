mod communication;
mod credentials;
use credentials::Credentials;
use crate::constants::EXPERIMENT_DB_NAME;
mod stat;
mod job;
use job::JobHandler;
mod fill_template;
mod experiments;
mod job_data;
mod test_job;
mod constants;
mod config_parse;
mod caching;
mod sniper_config;

use clap::{Parser, ArgGroup};
use std::str::FromStr;
use std::path::Path;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
#[command(group(
            ArgGroup::new("TitanCmd")
                .required(true)
                .args(["list", "submit", "delete", "collect"]),
        ))]
struct Args{
    #[arg(long, group="Glist", value_parser = TitanObject::parse_titan_obj)]
    list: Option<TitanObject>, 

    #[arg(long, group="Gsubmit", value_parser = TitanObject::parse_titan_obj)]
    submit: Option<TitanObject>,

    #[arg(long, group="Gdelete", value_parser = TitanObject::parse_titan_obj)]
    delete: Option<TitanObject>,

    #[arg(long, group="Gsubmit", value_parser = TitanObject::parse_titan_obj)]
    collect: Option<TitanObject>,

    // If we display all the information or restricted to the user. (Only valid with --list)
    #[arg(short, long, requires = "Glist")]
    all: bool,

    // If we display the completed jobs as well. (Only valid with --list jobs)
    #[arg(short, long, requires = "Glist")]
    completed: Option<usize>,

    // The jobid we wish to delete
    #[arg(short='i', long, requires = "Gdelete", value_parser = JobIds::parse_jobids)]
    jobid: Option<JobIds>,
    //jobid: Option<Vec<usize>>,

    // Path of the object we wish to upload/execute. In case of submit job, the first file will be
    // executed on sniper, and the second one (e.g. execute.sh, to run on the VM) will only be copied to the titan node.
    // (e.g usage ./bin -u job -p job.sh execute.sh)
    #[arg(short, long, requires = "Gsubmit")]
    path: Option<Vec<String>>,

    #[arg(short, long, requires = "Gdelete")]
    name: Option<String>,

    #[arg(long, requires = "Gsubmit")]
    dry: bool
}

//Clap bug? -> Had to encapsulate the vec inside a struct
#[derive(Debug, Clone)]
struct JobIds{
    pub ids: Vec<usize>
}

impl JobIds{
    fn parse_jobids(arg: &str) -> Result<Self, std::num::ParseIntError> {
        let mut ids = Vec::<usize>::new();
        for part in arg.split(",") {
            if part.contains("-") {
                let extremes: Vec<&str> = part.split("-").collect();
                let first = usize::from_str(extremes[0])?;
                let last = usize::from_str(extremes[1])? + 1;
                let mut vals = (first..last).collect();
                ids.append(&mut vals);
            } else {
                ids.push(usize::from_str(part)?);
            }
        }
        Ok(JobIds{ids})
    }
}


#[derive(Clone, Debug)]
enum TitanObject {
    Job,
    VM,
    Trace
}

impl TitanObject {
    fn parse_titan_obj(arg: &str) -> Result<Self, String> {
        match arg.to_lowercase().as_str() {
            "job" => Ok(TitanObject::Job),
            "jobs" => Ok(TitanObject::Job),
            "vm" => Ok(TitanObject::VM),
            "vms" => Ok(TitanObject::VM),
            "trace" => Ok(TitanObject::Trace),
            "traces" => Ok(TitanObject::Trace),
            _ => Err(format!("{} is not a recognized option, choose: Job|VM|Trace", arg)),
        }
    }
}

pub fn main() {
    let args = Args::parse();

    let creds = Credentials::new().unwrap();

    if let Some(titan_obj) = args.list {
        //let titan_obj = TitanObject::validate_value_cli(titan_object, "LIST");
        match titan_obj {
            TitanObject::Job => {
                let s = JobHandler::new(creds,args.all,args.completed);
                let res = s.print_jobs();
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error when retreiving the job list: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TitanObject::VM => {
                let res = stat::print_vms();
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error when retreiving the VMS list: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            TitanObject::Trace => {
                let res = stat::print_traces();
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error when retreiving the traces: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    } else if let Some(titan_obj) = args.delete {
        //let titan_obj = TitanObject::validate_value_cli(titan_object, "DELETE");
        match titan_obj {
            TitanObject::Job => {
                if let Some(jobids) = args.jobid {
                    let s = JobHandler::new(creds, false, None);
                    let res = s.delete_jobs(jobids.ids);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error when deleting JOB: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            TitanObject::VM => {
                let res = stat::delete_vm(&args.name.unwrap());
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error when deleting vm: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                println!("Unsupported feature"); //need to add script on bacchus itself -> to remove from all the nodes
            }
        }
    } else if let Some(titan_obj) = args.submit {
        match titan_obj {
            TitanObject::VM => {
                if let Some(paths) = args.path {
                    let dockerfile_path = Path::new(&paths[0]);
                    let res = stat::upload_dockerfile(dockerfile_path);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error when uploading dockerfile from path {}: {}", dockerfile_path.display(), e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            TitanObject::Trace => {
                if let Some(paths) = args.path {
                    let trace_path = Path::new(&paths[0]);
                    let res = stat::upload_trace(trace_path);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error when uploading trace from path {}: {}", trace_path.display(), e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            TitanObject::Job => {
                let s = JobHandler::new(creds, false, None);
                let paths = args.path.unwrap();
                if paths.len() >= 2 {
                    let experiment_path = &paths[0];
                    let benchmarks_path = &paths[1];
                    let res = s.submit_job(experiment_path, benchmarks_path, &args.dry);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error when submitting job: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("You need to provide an experiment and an benchmarks path to submit a Job");
                }
            }
        }
    } else if let Some(titan_obj) = args.collect {
        match titan_obj {
            TitanObject::Job => {
                let mut s = JobHandler::new(creds, false, None);
                if let Some(paths) = args.path {
                    let experiment_map_path = Path::new(&paths[0]);
                    let res = s.collect_jobs(&experiment_map_path);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error when collecting jobs: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("You need to provide a Path with a {} file to download the experiments", EXPERIMENT_DB_NAME);
                }
            }
            _ => {
                println!("Unsupported feature");
            }
        }
    }
}
