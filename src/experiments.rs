use json;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt::Write;
use std::collections::HashMap;

pub struct ExperimentArguments<'a> {
    pub job_arguments: HashMap<&'a str, String>,
    pub benchmark_arguments: Vec<HashMap<&'a str, String>>
}

pub struct Experiment{
    exp: json::JsonValue,
    bench: json::JsonValue
}

impl Experiment{
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

    pub fn get_arguments(&self) -> Vec<ExperimentArguments> {
        let mut job_args = Vec::new();
        let job_git_repos: String = self.get_git_repos(&self.exp);
        let job_mounts: String = self.get_vm_mounts(&self.exp);

        for suite in self.bench["suites"].members() {
            let mut total_jobs: usize = suite["benchmarks"].len() * self.get_number_configs();
            total_jobs = 1; //debug

            let mut git_repos: String = self.get_git_repos(&suite);
            git_repos.push_str(&job_git_repos);

            let mut mounts: String = self.get_vm_mounts(&suite);
            mounts.push_str(&job_mounts);

            let job_vals = &self.exp["job"];
            let job_name = json_value_to_string(&job_vals["name"],"") + &format!("_{}", suite["suite"]);
            let args = HashMap::from([
                // ("<ACCOUNT>",   String::new()),
                ("<JOB>",       job_name),
                ("<CORES>",     json_value_to_string(&job_vals["core_per_experiment"],"")),
                ("<MEMORY>",    self.get_tot_memory().to_string()),
                ("<TASKS>",     total_jobs.to_string()),
                ("<VM_NAME>",   json_value_to_string(&job_vals["vm_name"],"")),
                ("<GIT-REPOSITORIES>", git_repos),
                ("<MOUNTS>",    mounts),
            ]);

            let bench_args = self.get_exp_arguments(&suite);
            job_args.push(ExperimentArguments{job_arguments: args, benchmark_arguments: bench_args});
        }
        return job_args;
    }

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

    fn get_sniper_arguments(&self) -> Vec<String> {
        let mut sniper_args = Vec::new();
        let arguments = json_value_to_string(&self.exp["sniper_parameters"]["arguments"], " ");
        let param_values = &self.exp["sniper_parameters"]["param_values"];
        let mut keys: Vec<String> = Vec::new();
        for (key, _) in param_values.entries() {keys.push(key.to_string());}
        let param_combinations = create_all_param_values(&keys, &param_values);

        for combination in param_combinations {
            let mut arg = arguments.clone();
            for (key, val) in std::iter::zip(keys.clone(), combination) {
                arg = arg.replace(&*format!("{{{key}}}"), &val);
            }
            sniper_args.push(arg);
        }
        return sniper_args;
    }

    fn get_exp_arguments(&self, benchmark_suite: &json::JsonValue) -> Vec<HashMap<&str, String>> {
        let mut exp_arguments = Vec::new();
        let sniper_args = self.get_sniper_arguments();
        
        let suite_path = json_value_to_string(&benchmark_suite["suite_path"],"");
        let bench_sniper_args = json_value_to_string(&benchmark_suite["sniper_args"], " ");
        let binary = json_value_to_string(&benchmark_suite["type"],"") == "binaries";

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

            let benchmark_str = if binary {
                let binary_name = json_value_to_string(&benchmark["binary"],"");
                let benchmark_args = json_value_to_string(&benchmark["arguments"], " ");
                format!("-- ${{BENCHMARKS_DIR}}/{benchmark_path}/{binary_name} {benchmark_args}")
            } else {
                let mut trace_vec = Vec::new();
                for trace_name in benchmark["traces"].members(){
                    trace_vec.push(format!("${{TRACES_DIR}}/{benchmark_path}/{}", json_value_to_string(trace_name,"")));
                }
                let trace_names = trace_vec.join(",");
                format!("--traces={trace_names}")
            };

            for sniper_arg in &sniper_args {
                let all_arguments = sniper_arg.to_owned() + " " + &bench_sniper_args + " " + &benchmark_str;
                let args = HashMap::from([
                    ("<BENCH_DIR>", benchmark_path.clone()),
                    ("<BUILD_COMMAND>", benchmark_build.clone()),
                    ("<SETUP_CMD>", setup_cmd.clone()),
                    ("<ARGUMENTS>", all_arguments)
                ]);
                exp_arguments.push(args);
            }
        }
        return exp_arguments;
    }
}

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
