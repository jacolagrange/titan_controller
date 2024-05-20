use json;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::io::Write as IoWrite;
use std::fmt::Write;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

/*
 * A Job is a the whole collections of all the different experiments (There is one job / benchmark
 * sutie, because of different mounting and git requirements.)
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct JobArgument {
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
}

/*
 * An experiment is defined for a given (sniper) configuration with defined parameters
 */
#[derive(Serialize, Deserialize, Debug)]
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
        let file = File::create(dst_path.join("args.json"))?;
        let mut writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(&mut writer, &self.variable_sniper_parameters)?;
        writer.flush()?;
        Ok(())
    }

}


/*
 * A Benchmark is the based on the Experiment, all the inputs it needs to be run with
 */ 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkArgument{
    pub arguments: HashMap<String, String>,
    pub benchmark_name: String,
    pub run_idx: usize,

    pub task_idx: Option<usize>
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


#[derive(Debug)]
pub struct ParseExperiment{
    exp: json::JsonValue,
    bench: json::JsonValue
}

impl ParseExperiment{
    pub fn new(exp_json_path: &Path, bench_json_path: &Path) -> Self{
        let mut exp_json_file = File::open(exp_json_path).unwrap();
        let mut exp_json_data = String::new();
        let _ = exp_json_file.read_to_string(&mut exp_json_data).unwrap();
        drop(exp_json_file);
        let exp = json::parse(&exp_json_data).unwrap();

        let mut bench_json_file = File::open(bench_json_path).unwrap();
        let mut bench_json_data = String::new();
        let _ = bench_json_file.read_to_string(&mut bench_json_data).unwrap();
        drop(bench_json_file);
        let bench = json::parse(&bench_json_data).unwrap();

        return Self {exp, bench}
    }

    pub fn get_arguments(&self) -> Vec<JobArgument> {
        let mut job_args = Vec::new();
        let job_git_repos: String = self.get_git_repos(&self.exp);
        let job_mounts: String = self.get_vm_mounts(&self.exp);

        for suite in self.bench["suites"].members() {
            let mut git_repos: String = self.get_git_repos(&suite);
            git_repos.push_str(&job_git_repos);

            let mut mounts: String = self.get_vm_mounts(&suite);
            mounts.push_str(&job_mounts);

            let job_vals = &self.exp["job"];

            let nr_runs: usize = job_vals["runs"].as_usize().unwrap();
            let total_jobs: usize = suite["benchmarks"].len() * self.get_number_configs() * nr_runs;

            let job_name = json_value_to_string(&job_vals["name"],"") + &format!("_{}", suite["suite"]);

            let meta_arguments = HashMap::from([
                // ("<ACCOUNT>",   String::new()),
                (String::from("<JOB>"),       job_name),
                (String::from("<CORES>"),     json_value_to_string(&job_vals["core_per_experiment"],"")),
                (String::from("<MEMORY>"),    self.get_tot_memory().to_string()),
                (String::from("<TASKS>"),     total_jobs.to_string()),
                (String::from("<VM_NAME>"),   json_value_to_string(&job_vals["vm_name"],"")),
                (String::from("<GIT-REPOSITORIES>"), git_repos),
                (String::from("<MOUNTS>"),    mounts),
            ]);

            let experiment_arguments = self.get_exp_arguments(&suite, nr_runs);

            let host_dst_path_str = json_value_to_string(&self.exp["host_destination_path"], "");
            let host_dst_path = Path::new(&host_dst_path_str);

            job_args.push(
                JobArgument{
                    meta_arguments, 
                    experiment_arguments,
                    host_dst_path: host_dst_path.to_path_buf(),
                    job_nr: None
                });
        }
        return job_args;
    }


    /*
     * Helper function to fill the JOB/Meta arguments
     */

    fn get_tot_memory(&self) -> usize {
        let core_str = json_value_to_string(&self.exp["job"]["core_per_experiment"],"");
        let mem_per_core_str = json_value_to_string(&self.exp["job"]["mem_per_core"],"");

        let core_nr: usize = core_str.parse().unwrap();
        let mem_per_core: usize = mem_per_core_str.parse().unwrap();

        core_nr * mem_per_core
    }

    fn get_git_repos(&self, args_obj: &json::JsonValue) -> String {
        let mut git_repos = String::new();
        if args_obj.has_key("git") {
            for (repo, branch) in args_obj["git"].entries() {
                let repo_name = repo.strip_suffix("_branch").unwrap();
                let branch_str = branch.as_str().unwrap();
                let _ = write!(git_repos, "checkout_git_repo {repo_name} {branch_str}\n");
            }
        }
        return git_repos;
    }

    fn get_vm_mounts(&self, args_obj: &json::JsonValue) -> String {
        let mut vm_mounts = String::new();
        //First mount the git repos
        if args_obj.has_key("git"){
            for (repo, _) in args_obj["git"].entries() {
                let repo_name = repo.strip_suffix("_branch").unwrap();
                let repo_name_upper = repo_name.to_uppercase();
                let _ = write!(vm_mounts, "mount_vbox {repo_name}_mount /home/slurmslave/{repo_name}/${{{repo_name_upper}_GIT_ID}}\n");
            }
        }

        //Mount additional repos
        if args_obj.has_key("vm_mount") {
            for (mnt_name, host_path) in args_obj["vm_mount"].entries() {
                let host_path_str = host_path.as_str().unwrap();
                if host_path_str.to_lowercase() == "none" {continue;}
                let _ = write!(vm_mounts, "mount_vbox {mnt_name} {host_path}\n");
            }
        }

        return vm_mounts;
    }

    fn get_number_configs(&self) -> usize {
        let mut nr_conf: usize = 1;
        for (_, vals) in self.exp["sniper_parameters"]["param_values"].entries(){
            nr_conf *= vals.len();
        }
        return nr_conf;
    }

    /*
     * Function to process all the arguments for the benchmark themselves
     */
    fn get_exp_arguments(&self, benchmark_suite: &json::JsonValue, nr_runs: usize) -> Vec<ExperimentArgument> {
        let mut exp_arguments: Vec<ExperimentArgument> = Vec::new();
        let sniper_arg_maps = self.get_sniper_arguments();
        let mut sniper_str_arguments = json_value_to_string(&self.exp["sniper_parameters"]["arguments"], " ");
        sniper_str_arguments.push_str(" ");
        let benchmarks_arguments = self.get_benchmark_arguments(&benchmark_suite, nr_runs);

        //let host_job_exp_meta_info_path_str = json_value_to_string(&self.exp["host_destination_path"], "");
        //let host_job_exp_meta_info_path = Path::new(&host_job_exp_meta_info_path_str);
        
        for sniper_arg in &sniper_arg_maps {
            let mut sniper_str_arg: String = sniper_str_arguments.clone();
            for (key, val) in sniper_arg {sniper_str_arg = sniper_str_arg.replace(&*format!("{{{key}}}"), &val);}

            let mut benchmarks = benchmarks_arguments.clone();
            for benchmark in &mut benchmarks {
                benchmark.arguments.get_mut("<ARGUMENTS>").unwrap().insert_str(0, &sniper_str_arg);
            }

            let sniper_dir_name: String = sniper_arg.clone().into_values().collect::<Vec<String>>().join("_");
            
            exp_arguments.push(
                ExperimentArgument{
                    sniper_dir_name,
                    variable_sniper_parameters: sniper_arg.clone(),
                    benchmarks
                });
        }
        return exp_arguments;
    }

    fn get_benchmark_arguments(&self, benchmark_suite: &json::JsonValue, nr_runs: usize) -> Vec<BenchmarkArgument> {
        let mut benchmark_arguments = Vec::new();

        let benchmark_parameters = json_value_to_string(&benchmark_suite["sniper_args"], " ");
        let is_binary = json_value_to_string(&benchmark_suite["type"],"") == "binaries";
        let suite_path = json_value_to_string(&benchmark_suite["suite_path"],"");

        for benchmark in benchmark_suite["benchmarks"].members(){
            let benchmark_path = if benchmark.has_key("bench_path") {
                format!("{suite_path}/{}", json_value_to_string(&benchmark["bench_path"],""))
            } else {
                suite_path.to_string()
            };

            let benchmark_build = if benchmark.has_key("build_cmd") {
                format!("\"{}\"", json_value_to_string(&benchmark["build_cmd"], " "))
            } else {
                String::from("make")
            };

            let setup_cmd = if benchmark.has_key("setup_cmd") {
                json_value_to_string(&benchmark["setup_cmd"], "\n")
            } else {
                String::new()
            };

            let benchmark_str = if is_binary {
                let binary_str = json_value_to_string(&benchmark["binary"],"");
                let benchmark_args = json_value_to_string(&benchmark["arguments"], " ");
                format!("-- ${{BENCHMARKS_DIR}}/{benchmark_path}/{binary_str} {benchmark_args}")
            } else {
                let mut trace_vec = Vec::new();
                for trace_str in benchmark["traces"].members(){
                    trace_vec.push(format!("${{TRACES_DIR}}/{benchmark_path}/{}", json_value_to_string(trace_str,"")));
                }
                let traces_string = trace_vec.join(",");
                format!("--traces={traces_string}")
            };

            let benchmark_name = json_value_to_string(&benchmark["name"], "");

            let all_arguments = format!("{benchmark_parameters} {benchmark_str}");

            for run_idx in 0..nr_runs {
                let arguments = HashMap::from([
                    (String::from("<BENCH_DIR>"), benchmark_path.clone()),
                    (String::from("<BUILD_COMMAND>"), benchmark_build.clone()),
                    (String::from("<SETUP_CMD>"), setup_cmd.clone()),
                    (String::from("<ARGUMENTS>"), all_arguments.clone())
                ]);
                benchmark_arguments.push(
                    BenchmarkArgument{
                        arguments,
                        benchmark_name: benchmark_name.clone(),
                        run_idx,
                        task_idx: None
                    });
            }
        }
        return benchmark_arguments;
    }



    fn get_sniper_arguments(&self) -> Vec<HashMap<String, String>> {
        let mut sniper_args = Vec::new();
        //let arguments = json_value_to_string(&self.exp["sniper_parameters"]["arguments"], " ");
        let param_values = &self.exp["sniper_parameters"]["param_values"];
        let mut keys: Vec<String> = Vec::new();
        for (key, _) in param_values.entries() {keys.push(key.to_string());}
        let param_combinations = create_all_param_values(&keys, &param_values);

        for combination in param_combinations {
            let arg = HashMap::from_iter(std::iter::zip(keys.clone(), combination));
            sniper_args.push(arg);
        }
        return sniper_args;
    }


    pub fn create_submit_job_map(&self, job_arguments: &Vec<JobArgument>) -> std::io::Result<()> {
        let host_dst_path_str = json_value_to_string(&self.exp["host_destination_path"], "");
        let host_dst_path = Path::new(&host_dst_path_str);

        let file = File::create(host_dst_path.join("experiments.json"))?;
        let mut writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(&mut writer, job_arguments)?;
        writer.flush()?;
        Ok(())
    }
}

/* Function that creates all the possible combinations of values
 * Output is a vector of each combination, and a combination is a vector of the string combinations
 */
fn create_all_param_values(keys: &[String], values: &json::JsonValue) ->Vec<Vec<String>> {
    if keys.is_empty() {return Vec::new();}

    let key = &keys[0];
    let mut combinations = Vec::new();
    if keys.len() == 1 {
        for key_value in values[key].members() {
            combinations.push(vec![json_value_to_string(&key_value, "")]);
        }
    } else {
        let next_values = create_all_param_values(&keys[1..], values);

        for key_value in values[key].members() {
            for next_value in &next_values {
                let mut val = vec![json_value_to_string(&key_value, "")];
                val.append(&mut next_value.clone());
                combinations.push(val);
            }
        }
    }
    return combinations;
}

fn json_value_to_string(json_val: &json::JsonValue, separator: &str) -> String {
    match json_val {
        json::JsonValue::Array(array) => {
            array.into_iter().map(|x| json_value_to_string(x,"")).collect::<Vec<String>>().join(separator)
        },
        json::JsonValue::Number(_number) => {json_val.as_isize().unwrap().to_string()},
        json::JsonValue::String(string) => {string.clone()},
        json::JsonValue::Short(short) => {short.as_str().to_string()}
        json::JsonValue::Boolean(bool_val) => {bool_val.to_string()}
        _ => {String::new()}
    }
}
