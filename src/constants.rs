pub static ID_FILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.id"));

pub static JOB_TEMPLATE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/script-template/job_docker.sh"));
pub static EXECUTE_TEMPLATE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/script-template/execute_Sniper.sh"));

pub static EXPERIMENT_DB_NAME: &str = "experiments.json";
pub static SNIPER_ARGUMENT_FILE_NAME: &str = "args.json";

pub static TEMP_FOLDER_NAME: &str = "/tmp/titan_controller";

pub static TITAN_SUBMIT_DIR: &str = "/home/slurmslave/jobs/submitted/.";

pub static LOCAL_SNIPER_DIR: &str = "/home/jaime/Documents/sniperAFS/sniper";
