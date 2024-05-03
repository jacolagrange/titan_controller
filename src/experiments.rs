use json;
use json::JsonValue::Array;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt::Write;
use std::collections::HashMap;

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

    pub fn get_job_arguments(&self) -> HashMap<&str, String> {
        let total_jobs: usize = self.get_number_benchmarks() * self.get_number_configs();
        let git_repos: String = self.get_git_repos();
        let mounts: String = self.get_vm_mounts();
        let job_vals = &self.exp["job"];
        HashMap::from([
            // ("<ACCOUNT>", String::from("jdiroela")),
            ("<JOB>",       String::from(job_vals["name"].as_str().unwrap())),
            ("<CORES>",     String::from(job_vals["core_per_experiment"].as_str().unwrap())),
            ("<MEMORY>",    String::from(job_vals["mem_per_experiment"].as_str().unwrap())),
            ("<TASKS>",     total_jobs.to_string()),
            ("<VM_NAME>",   String::from(job_vals["vm_name"].as_str().unwrap())),
            ("<GIT-REPOSITORIES>", git_repos),
            ("<MOUNTS>",    mounts),
        ])
    }

    fn get_git_repos(&self) -> String {
        let mut git_repos = String::new();
        for (repo, branch) in self.exp["git"].entries() {
            let repo_name = repo.strip_suffix("_branch").unwrap();
            let branch_str = branch.as_str().unwrap();
            write!(git_repos, "checkout_git_repo {repo_name} {branch_str}\n");
        }
        return git_repos;
    }

    fn get_vm_mounts(&self) -> String {
        let mut vm_mounts = String::new();
        //First mount the git repos
        for (repo, _) in self.exp["git"].entries() {
            let repo_name = repo.strip_suffix("_branch").unwrap();
            let repo_name_upper = repo_name.to_uppercase();
            write!(vm_mounts, "mount_vbox {repo_name}_mount /home/slurmslave/{repo_name}/${{{repo_name_upper}_GIT_ID}}\n");
        }

        //Mount additional repos
        for (mnt_name, host_path) in self.exp["vm_mount"].entries() {
            let host_path_str = host_path.as_str().unwrap();
            write!(vm_mounts, "mount_vbox {mnt_name} {host_path}\n");
        }
        return vm_mounts;
    }

    fn get_number_benchmarks(&self) -> usize {
        let mut nr_bench: usize = 0;
        for suite in self.bench["suites"].members() {
            nr_bench += suite["benchmarks"].len();
        }
        return nr_bench;
    }

    fn get_number_configs(&self) -> usize {
        let mut nr_conf: usize = 0;
        for (_, vals) in self.exp["sniper_parameters"]["param_values"].entries(){
            nr_conf += vals.len();
        }
        return nr_conf;
    }
}
