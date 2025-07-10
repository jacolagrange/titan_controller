// use std::fs::File;
// use std::path::Path;
// use std::io::prelude::*;
use crate::constants::ID_FILE;

#[derive(Debug)]
pub struct Credentials{
    pub username: String,
    pub password: String
}

impl Credentials {
    pub fn new() -> std::io::Result<Self> {
        // let file_path = Path::new(".id");
        // if !file_path.exists() {
        //     eprintln!("No .id file found");
        //     std::process::exit(1);
        // }

        // let mut file = File::open(file_path)?;
        // let mut content = String::new();
        // file.read_to_string(&mut content)?;
        //let creds: Vec<&str> = content.split(':').collect();
        let creds: Vec<&str> = ID_FILE.split(':').collect();
        
        if creds.len() != 2 {
            eprintln!("Problem reading in the credentials from the \".id\" file.");
            std::process::exit(1);
        }
        
        Ok(Credentials{
            username: String::from(creds[0]), 
            password: String::from(creds[1])
        })
    }
}
