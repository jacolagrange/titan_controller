use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::collections::HashMap;

pub fn fill_template(template_path: &Path, dst_path: &Path, replacement_map: &HashMap<String, String>){
    let mut template_file = File::open(template_path).unwrap();
    let mut data = String::new();
    let _ = template_file.read_to_string(&mut data);
    drop(template_file); //File is closed

    for (old_str, new_str) in replacement_map.into_iter(){
        data = data.replace(&*old_str, &new_str);
    }

    let mut dst_file = File::create(dst_path).unwrap();
    let _ = dst_file.write(data.as_bytes());
}
