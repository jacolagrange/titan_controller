use crate::communication::ssh;

use std::path::Path;
use std::vec::Vec;
use std::str::FromStr;

pub fn print_vms() {
    let (stdout, stderr) = ssh::send_command("tree -d -L 1 /home/slurmslave/virtualbox");
    if stderr.len() > 0 {println!("{}", stderr);}
    else {println!("{}", stdout);}
}

pub fn upload_vm(vm_path: &Path) {
    let destination = Path::new("/home/slurmaslave/virtualbox/");
    println!("Uploading VM to bacchus");
    let _ = ssh::send_files(vm_path.to_str().unwrap(), destination.to_str().unwrap());
    println!("Distributing to other nodes");
    let command = format!("/home/slurmadmin/scripts/distribute_VM.py {}/{}", destination.to_str().unwrap(), vm_path.file_name().unwrap().to_str().unwrap());
    ssh::send_command(&command);
}

fn get_active_nodes() -> Vec<usize> {
    let mut nodes = vec!();
    let (stdout, _) = ssh::send_command("sinfo -o \"%N %a\" | grep up");
    let range_titan = stdout.split_whitespace().next().unwrap();
    if let (Some(start_idx), Some(end_idx)) = (range_titan.find("["), range_titan.find("]")) {
        let portions = &range_titan[start_idx+1..end_idx];
        for portion in portions.split(",") {
            if portion.contains("-") {
                let dash_idx = portion.find("-").unwrap();
                let a = usize::from_str(&portion[0..dash_idx]).unwrap();
                let b = usize::from_str(&portion[dash_idx+1..]).unwrap();
                let mut total_range = (a..b).collect();
                nodes.append(&mut total_range);
            } else {
                nodes.push(usize::from_str(portion).unwrap());
            }
            
        }

    }
    nodes
}

pub fn delete_vm(vm_name: &str) {
    println!("Deleting VM {} from bacchus and all titan nodes", vm_name);
    println!("This cannot be undone, are you sure you want to continue? y|N");
    let mut answ = String::new();
    std::io::stdin().read_line(&mut answ).unwrap();
    if answ.to_lowercase().contains("y"){
        println!("Proceeding");
        let active_nodes = get_active_nodes();
        ssh::send_command(format!("rm -r /home/slurmslave/virtualbox/{}", vm_name).as_str());
        let unregister_cmd = format!("VBoxManage unregistervm {} --delete", vm_name);
        for node_nr in active_nodes {
            let command = format!("ssh titan{node_nr:#02} {unregister_cmd}");
            println!("{}", command);
            ssh::send_command(command.as_str());
        }
    } else {
        println!("deletion aborted");
    }
}

pub fn print_traces() {
    let (stdout, stderr) = ssh::send_command("tree -d /home/slurmslave/traces");
    if stderr.len() > 0 {println!("{}", stderr);}
    else {println!("{}", stdout);}
}

pub fn upload_trace(trace_path: &Path) {
    if ! trace_path.is_dir() {
        println!("The traces should be contained within a folder to upload at once.");
        return;
    }
    let destination = Path::new("/home/slurmaslave/traces/sift");
    println!("Uploading trace to bacchus");
    let _ = ssh::send_files(trace_path.to_str().unwrap(), destination.to_str().unwrap());
    println!("Distributing to other nodes");
    let command = format!("/home/slurmadmin/scripts/distribute_VM.py {}/{}", destination.to_str().unwrap(), trace_path.file_name().unwrap().to_str().unwrap());
    ssh::send_command(&command);
}
