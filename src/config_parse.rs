use std::collections::HashMap;
use std::path::Path;
use config::{Config, File, ConfigError};
use std::fs;

use crate::sniper_config::SniperConfig;
use crate::constants::LOCAL_SNIPER_DIR;

#[derive(Debug)]
pub struct ParsedArgs {
    configs: Vec<String>,
    cmd_settings: HashMap<String, String>,
    others: Vec<String>,
}

impl ParsedArgs{
    pub fn new(input: &str) -> Self {
        let mut configs = vec![String::from("base")];
        let mut cmd_settings = HashMap::new();
        let mut others = Vec::new();
    
        let mut iter = input.split_whitespace().peekable();
    
        while let Some(arg) = iter.next() {
            match arg {
                "-c" => {
                    if let Some(value) = iter.next() {
                        configs.push(value.to_string());
                    }
                }
                "-g" => {
                    if let Some(setting) = iter.next() {
                        // Allow --key=value or key=value
                        let setting = setting.strip_prefix("--").unwrap_or(setting);
                        if let Some((key, value)) = setting.split_once('=') {
                            cmd_settings.insert(key.to_string(), value.to_string());
                        }
                    }
                }
                other => {
                    others.push(other.to_string());
                }
            }
        }
    
        ParsedArgs {
            configs,
            cmd_settings,
            others,
        }
    }

    pub fn get_final_config(&self) -> Result<Config, ConfigError> {
        let mut settings = Config::builder();

        let config_dir = Path::new(LOCAL_SNIPER_DIR).join("config");
        for config in &self.configs {
            settings = settings.add_source(File::new(config_dir.join(format!("{config}.cfg")).to_str().unwrap(), SniperConfig));
        }

        for (key, val) in &self.cmd_settings {
            //settings = settings.set_override(key, val.as_str())?;
            let parsed_key = key.replace("/", ".");
            let res = settings.set_override(&parsed_key, val.as_str());
            settings = 
            if res.is_err() {
                println!("ERROR happened {:#?}", &res);
                res?
            } else {
                res?
            };
        }
        
        settings.build()
    }
}
