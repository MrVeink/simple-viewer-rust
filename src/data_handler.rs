use std::path::Path;
use std::error::Error;
use std::fs::File;
use csv::ReaderBuilder;
use reqwest::blocking::Client;
use serde_json::Value;
use crate::data_types::TableData;

// Common header processing logic used by both local CSV and Google Sheets
fn process_headers(headers: Vec<String>) -> (Vec<String>, Vec<bool>) {
    let columns_to_hide = vec![
        "sport_id", "team_members", "team_name",
        "info", "result_code", "position_pre"
    ];
    
    let mut processed_headers = Vec::new();
    let mut visible_columns = Vec::new();
    
    for header in headers {
        // Check if this column should be hidden
        let should_hide = columns_to_hide.iter()
            .any(|col| header.to_lowercase().contains(col));
        
        visible_columns.push(!should_hide);
        
        if !should_hide {
            // Apply header replacements
            let processed_header = replace_header(&header);
            processed_headers.push(processed_header);
        }
    }
    
    (processed_headers, visible_columns)
}

fn replace_header(header: &str) -> String {
    let header_lower = header.to_lowercase();
    
    // Header replacements mapping
    let replacements = [
        ("category", "Series"),
        ("first_name", "Name"),
        ("last_name", "Surname"),
        ("organization", "Club"),
        ("napat", "X"),
        ("result", "Result"),
        ("posit.", "Rank")
    ];
    
    // First check for part-X and psum-X patterns
    if header_lower.contains("part-") {
        if let Some(part_num) = header.split('-').nth(1) {
            return format!("S{}", part_num);
        }
    } else if header_lower.contains("psum-") {
        if let Some(part_num) = header.split('-').nth(1) {
            return format!("P{}", part_num);
        }
    }
    
    // Then check other replacements
    for (original, replacement) in replacements.iter() {
        if header_lower.contains(original) {
            return replacement.to_string();
        }
    }
    
    header.to_string()
}

// Load data from local CSV file
pub fn load_csv_file<P: AsRef<Path>>(path: P) -> Result<TableData, Box<dyn Error>> {
    let mut data = TableData::empty();
    
    // Detect delimiter
    let delimiter = detect_delimiter(&path)?;
    
    let file = File::open(&path)?;
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter as u8)
        .flexible(true)
        .from_reader(file);
    
    // Process headers
    let headers: Vec<String> = reader.headers()?
        .iter()
        .map(String::from)
        .collect();
    
    let (processed_headers, visible_columns) = process_headers(headers);
    data.headers = processed_headers;
    
    // Process rows
    for result in reader.records() {
        let record = result?;
        
        // Skip empty rows
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }
        
        // Filter visible columns
        let filtered_row: Vec<String> = record.iter()
            .enumerate()
            .filter(|(i, _)| i < &visible_columns.len() && visible_columns[*i])
            .map(|(_, field)| field.to_string())
            .collect();
        
        data.rows.push(filtered_row);
    }
    
    Ok(data)
}

fn detect_delimiter<P: AsRef<Path>>(path: P) -> Result<char, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut first_line = String::new();
    std::io::Read::read_to_string(&mut file, &mut first_line)?;
    
    if first_line.contains(';') {
        Ok(';')
    } else {
        Ok(',')
    }
}

// Extract spreadsheet ID from URL
fn extract_spreadsheet_id(url: &str) -> Result<String, Box<dyn Error>> {
    let parts: Vec<&str> = url.split('/').collect();
    
    for (i, part) in parts.iter().enumerate() {
        if *part == "d" && i + 1 < parts.len() {
            return Ok(parts[i+1].to_string());
        }
    }
    
    Err("Invalid spreadsheet URL".into())
}

// Load data from Google Sheets using public sheets API (no OAuth needed)
pub fn load_google_sheet(url: &str, sheet_name: &str) -> Result<TableData, Box<dyn Error>> {
    let mut data = TableData::empty();
    
    // Get spreadsheet ID from URL
    let spreadsheet_id = extract_spreadsheet_id(url)?;
    
    // Default to "Sheet1" if no sheet name provided
    let sheet = if sheet_name.is_empty() { "Sheet1" } else { sheet_name };
    
    // Build Google Sheets API URL
    let api_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}!A:Z?key=AIzaSyAa8yy0GdcGPHdtD083HiGGx_S0vMPScDM",
        spreadsheet_id, sheet
    );
    
    // Make API request
    let client = Client::new();
    let response = client.get(&api_url).send()?;
    
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()).into());
    }
    
    let json: Value = response.json()?;
    
    // Process the data
    if let Some(values) = json.get("values").and_then(Value::as_array) {
        if values.is_empty() {
            return Ok(data);
        }
        
        // Find header row (look for "category")
        let mut start_index = 0;
        for (i, row) in values.iter().enumerate() {
            if let Some(first_cell) = row.get(0).and_then(Value::as_str) {
                if first_cell.to_lowercase() == "category" {
                    start_index = i;
                    break;
                }
            }
        }
        
        // Get relevant data starting from header row
        let relevant_data = &values[start_index..];
        if relevant_data.is_empty() {
            return Ok(data);
        }
        
        // Process headers
        if let Some(header_row) = relevant_data.first() {
            let headers: Vec<String> = header_row
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect();
            
            let (processed_headers, visible_columns) = process_headers(headers);
            data.headers = processed_headers;
            
            // Process data rows
            for row_value in relevant_data.iter().skip(1) {
                if let Some(row_array) = row_value.as_array() {
                    // Skip empty rows
                    if row_array.is_empty() || row_array.iter().all(|cell| {
                        cell.as_str().map_or(true, |s| s.trim().is_empty())
                    }) {
                        continue;
                    }
                    
                    // Convert row to strings and filter visible columns
                    let row_data: Vec<String> = row_array
                        .iter()
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .collect();
                    
                    let filtered_row: Vec<String> = row_data
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| i < &visible_columns.len() && visible_columns[*i])
                        .map(|(_, s)| s.clone())
                        .collect();
                    
                    data.rows.push(filtered_row);
                }
            }
        }
    }
    
    Ok(data)
}
