use std::fs;
use serde_json;

pub fn load_similarity_threshold() -> Result<f32, String> {
    let config_content = fs::read_to_string("configs.json")
        .map_err(|e| format!("Lỗi đọc file configs.json: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Lỗi parse JSON: {}", e))?;

    let value = config["Value"].as_f64()
        .ok_or_else(|| "Không thể đọc giá trị Value từ configs.json".to_string())?;

    let threshold = ((value / -(2.0/35.0)) as f32) / 100.0;
    
    Ok(threshold)
}
