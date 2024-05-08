use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

pub fn send_command(command: &str) -> (String, String){
    let output = Command::new("ssh")
        .args(["titan", command])
        .output()
        .expect("ssh command failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (stdout, stderr)
}

pub fn send_files(src_path: &str, dst_path: &str) -> Result<(), std::io::Error> {
    let full_dst = format!("titan:{dst_path}");
    let output = Command::new("bash")
        .arg("-c")
        .arg(["scp", "-r", src_path, &full_dst].join(" "))
        .output()?;
    println!("scp output\n{:?} \nerror\n{:?}\n", String::from_utf8(output.stdout.clone()).unwrap(), String::from_utf8(output.stderr.clone()).unwrap());
    if output.stderr.len() > 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "SCP sending files failed"));
    }
    Ok(())
}

pub fn get_files(src_path: &str, dst_path: &str) -> Result<(), std::io::Error> {
    let full_src = format!("titan:{src_path}");
    let output = Command::new("scp")
        .args(["-r", &full_src, dst_path])
        .output()?;
    if output.stderr.len() > 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "SCP sending files failed"));
    }
    Ok(())
}

pub fn get_hash_titan(amount: usize) -> Vec<String> {
    let cmd = format!("for i in $(seq 1 {}); do mktemp; done", amount);
    let (stdout, _) = send_command(&cmd);

    let mut hash_vec = Vec::<String>::new();
    let output_lines: Vec<&str> = stdout.split("\n").collect();
    for line in &output_lines {
        if ! line.contains("/tmp/tmp."){
            continue;
        }
        let line_split: Vec<&str> = line.split(".").collect();
        hash_vec.push(line_split[1].to_string());
    }
    
    let rm_cmd = format!("rm {}", output_lines.join(" "));
    send_command(&rm_cmd);

    return hash_vec;
}
