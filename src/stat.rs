use crate::communication::ssh;

use std::path::Path;
use std::vec::Vec;
use std::str::FromStr;

pub fn print_vms() -> Result<(), std::io::Error> {
    let (stdout, stderr) = ssh::send_command("tree -d -L 1 /home/slurmslave/virtualbox")? ;
    if stderr.len() > 0 {println!("{}", stderr);}
    else {println!("{}", stdout);}
    Ok(())
}

// pub fn upload_vm(vm_path: &Path) {
//     let destination = Path::new("/home/slurmaslave/virtualbox/");
//     println!("Uploading VM to bacchus");
//     let _ = ssh::send_files(vm_path.to_str().unwrap(), destination.to_str().unwrap());
//     println!("Distributing to other nodes");
//     let command = format!("/home/slurmadmin/scripts/distribute_VM.py {}/{}", destination.to_str().unwrap(), vm_path.file_name().unwrap().to_str().unwrap());
//     ssh::send_command(&command);
// }

pub fn upload_dockerfile(dockerfile_path: &Path) -> Result<(), std::io::Error> {
    let destination = Path::new("/home/slurmslave/Dockerfiles/");
    println!("Uploading Dockerfile to bacchus");
    let _ = ssh::send_files(dockerfile_path.to_str().unwrap(), destination.to_str().unwrap());
    let mut container_name = String::from(dockerfile_path.file_stem().unwrap().to_str().unwrap());
    container_name = container_name.to_lowercase().replace("dockerfile", "").trim_matches('_').to_string().trim_matches('-').to_string();
    if !container_name.is_empty() {
        println!("Distributing {container_name} to other nodes");
        let remote_file_path = destination.join(dockerfile_path.file_name().unwrap());
        let command = format!("for node in {{01..16}}; do scp {remote_file_path_str} titan${{node}}:{remote_file_path_str}; ssh titan${{node}} 'docker build --no-cache -t {container_name} -f {remote_file_path_str} {destination_str}' & done", remote_file_path_str = remote_file_path.to_str().unwrap(), destination_str = destination.to_str().unwrap());
        ssh::send_command(&command)?;
    } else {
        eprintln!("Error the container name is empty, please name you file such as dockerfile_your_name!");
    }
    Ok(())
}

fn get_active_nodes() -> Result<Vec<usize>, std::io::Error> {
    let mut nodes = vec!();
    let (stdout, _) = ssh::send_command("sinfo -o \"%N %a\" | grep up")?;
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
    Ok(nodes)
}

pub fn delete_vm(vm_name: &str) -> Result<(), std::io::Error> {
    println!("Deleting VM {} from bacchus and all titan nodes", vm_name);
    println!("This cannot be undone, are you sure you want to continue? y|N");
    let mut answ = String::new();
    std::io::stdin().read_line(&mut answ).unwrap();
    if answ.to_lowercase().contains("y"){
        println!("Proceeding");
        let active_nodes = get_active_nodes()?;
        ssh::send_command(format!("rm -r /home/slurmslave/virtualbox/{}", vm_name).as_str())?;
        let unregister_cmd = format!("VBoxManage unregistervm {} --delete", vm_name);
        for node_nr in active_nodes {
            let command = format!("ssh titan{node_nr:#02} {unregister_cmd}");
            println!("{}", command);
            ssh::send_command(command.as_str())?;
        }
    } else {
        println!("deletion aborted");
    }
    Ok(())
}

pub fn print_traces() -> Result<(), std::io::Error> {
    let (stdout, stderr) = ssh::send_command("tree -d /home/slurmslave/traces")?;
    if stderr.len() > 0 {println!("{}", stderr);}
    else {println!("{}", stdout);}
    Ok(())
}

pub fn upload_trace(trace_path: &Path) -> Result<(), std::io::Error> {
    if ! trace_path.is_dir() {
        println!("The traces should be contained within a folder to upload at once.");
        return Ok(());
    }
    let destination = Path::new("/home/slurmaslave/traces/sift");
    println!("Uploading trace to bacchus");
    let _ = ssh::send_files(trace_path.to_str().unwrap(), destination.to_str().unwrap());
    println!("Distributing to other nodes");
    let command = format!("/home/slurmadmin/scripts/distribute_VM.py {}/{}", destination.to_str().unwrap(), trace_path.file_name().unwrap().to_str().unwrap());
    ssh::send_command(&command)?;
    Ok(())
}
