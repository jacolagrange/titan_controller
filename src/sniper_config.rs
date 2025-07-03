use config::{Config, Format, Map, Value, ValueKind, FileStoredFormat};

#[derive(Debug, Clone)]
pub struct SniperConfig;

impl Format for SniperConfig {
    fn parse(&self, uri: Option<&String>, text: &str) -> Result<Map<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Map::new();
        let mut current_section = String::new();
        
        for (line_num, line) in text.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Handle section headers
            if let (true, Some(end_char)) = (line.starts_with('['), line.find(']')) {
                current_section = line[1..end_char].to_string().replace("/", ".");
                
                // Create section key-value pair with empty map as value
                //let section_map = Map::new();
                //results.insert(
                //    current_section.clone(),
                //    Value::new(None, ValueKind::Table(section_map))
                //);
                //println!("Found a new section: {current_section}");
                continue;
            }
            
            // Handle key-value pairs
            if let Some(eq_pos) = line.find('=') {
                let key_part = line[..eq_pos].trim();
                let value_part = line[eq_pos + 1..].trim();
                
                // Remove inline comments
                let value_part = if let Some(comment_pos) = value_part.find('#') {
                    value_part[..comment_pos].trim()
                } else {
                    value_part
                };
                
                // Determine if this is an array key (ends with [])
                let (base_key, is_array) = if key_part.ends_with("[]") {
                    (&key_part[..key_part.len()-2], true)
                } else {
                    (key_part, false)
                };
                
                // Create full key path
                let full_key = if current_section.is_empty() {
                    base_key.to_string()
                } else {
                    format!("{}.{}", current_section, base_key)
                };
                
                // Parse the value
                let parsed_value = if is_array {
                    parse_array_value(value_part, uri)?
                } else {
                    parse_single_value(value_part, uri)?
                };
                
                results.insert(full_key, parsed_value);
            } else if !line.is_empty() {
                return Err(format!("Invalid syntax on line {}: {}", line_num + 1, line).into());
            }
        }
        
        Ok(results)
    }
}

fn parse_single_value(value: &str, uri: Option<&String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Try to parse as boolean
    match value.to_lowercase().as_str() {
        "true" => return Ok(Value::new(uri, ValueKind::Boolean(true))),
        "false" => return Ok(Value::new(uri, ValueKind::Boolean(false))),
        _ => {}
    }
    
    // Try to parse as integer
    if let Ok(int_val) = value.parse::<i64>() {
        return Ok(Value::new(uri, ValueKind::I64(int_val)));
    }
    
    // Try to parse as float
    if let Ok(float_val) = value.parse::<f64>() {
        return Ok(Value::new(uri, ValueKind::Float(float_val)));
    }
    
    // Default to string
    Ok(Value::new(uri, ValueKind::String(value.to_string())))
}

fn parse_array_value(value: &str, uri: Option<&String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let elements: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
    let mut array_values = Vec::new();
    
    for element in elements {
        array_values.push(parse_single_value(element, uri)?);
    }
    
    Ok(Value::new(uri, ValueKind::Array(array_values)))
}

impl FileStoredFormat for SniperConfig {
    fn file_extensions(&self) -> &'static [&'static str] {
        &["cfg"]
    }
}
