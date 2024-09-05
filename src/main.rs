use std::env;
use std::error::Error;
use std::fs;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use serde::{Serialize, Deserialize};
use serde_json::json;
use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};

#[derive(Serialize, Deserialize, Debug)]
struct Layout {
    name: String,
    version: usize,
    delimiter: Option<char>, 
    file_type: String,
    fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Field {
    name: String,
    description: String, 
    position: usize,
    size: usize,
    field_type: String,
}

#[derive(Debug)]
struct Record {
    fields: HashMap<String, serde_json::Value>
}

/// Reads the file containing the layout settings.
/// 
/// # Parameters
/// 
/// - file_name: Name of the file containing the layout for data extraction.
/// 
/// # Returns
/// 
/// A vector of `Layout` structs representing the layout.
/// 
/// # Example
/// 
/// ```
/// let layout = read_config_json(&layout_file);
/// ```
/// 
fn read_config_json(file_name: &str) -> Result<Layout, Box<dyn Error>> {
    let config = fs::read_to_string(file_name)?;
    let layout: Layout = serde_json::from_str(&config)?;
    Ok(layout)
}

/// Reads the CSV data file and extracts the lines based on the provided layout.
/// 
/// # Parameters
/// 
/// - file_name: Name of the CSV data file.
/// - layout: A slice of `Field` structs representing the layout.
/// 
/// # Returns
/// 
/// A vector of `Record` structs containing the extracted data.
/// 
/// # Example
/// 
/// ```
/// let records = read_csv_data("data.csv", "layout.txt", ",");
/// ```
/// 
fn read_csv_data(file_name: &str, layout: &Layout) -> Vec<Record> {
    let file = fs::File::open(file_name).expect("Unable to open data file");
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let delimiter = layout.delimiter.unwrap_or(',');

    for (line_number, line) in reader.lines().enumerate() {
        let line = line.expect("Unable to read line");
        let mut fields = HashMap::new();
        let values: Vec<&str> = line.split(delimiter).collect();

        for (i, field) in layout.fields.iter().enumerate() {
            if let Some(value) = values.get(i) {
                let value = value.trim().trim_matches('"');
                let json_value = match field.field_type.as_str() {
                    "string" => serde_json::Value::String(value.to_string()),
                    "int" => serde_json::Value::Number(value.parse::<i64>().unwrap_or(0).into()),
                    "float" => serde_json::Value::Number(serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.0)).unwrap()),
                    "bool" => serde_json::Value::Bool(value.parse::<bool>().unwrap_or(false)),
                    _ => serde_json::Value::String(value.to_string()),
                };
                fields.insert(field.name.clone(), json_value);
            }
        }
        records.push(Record { fields });
    }
    records
}

/// Reads the data file and extracts the lines based on the provided layout.
/// 
/// # Parameters
/// 
/// - file_name: Name of the data file.
/// - layout: A slice of `Field` structs representing the layout.
/// 
/// # Returns
/// 
/// A vector of `Record` structs containing the extracted data.
/// 
/// # Example
/// 
/// ```
/// let records = read_fixed_data("data.txt", "layout.txt");
/// ```
/// 
fn read_fixed_data(file_name: &str, layout: &Layout) -> Vec<Record> {
    let file = fs::File::open(file_name).expect("Unable to open data file");
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut lines = reader.lines();

    while let Some(line) = lines.next() {
        let line = line.expect("Unable to read line");
        let mut fields = HashMap::new();
        let mut current_line = line.clone();
        let mut current_pos  = 0;

        for field in &layout.fields {
            if field.position < current_pos {
                if let Some(next_line) = lines.next() {
                    current_line = next_line.expect("Unable to read line");
                    current_pos = 0;
                } else {
                    break;
                }
            }
            let value = current_line[field.position -1 .. field.position -1 + field.size].trim().to_string();
            let json_value = match field.field_type.as_str() {
                "string" => serde_json::Value::String(value.to_string()),
                "int" => serde_json::Value::Number(value.parse::<i64>().unwrap_or(0).into()),
                "float" => serde_json::Value::Number(serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.0)).unwrap()),
                "bool" => serde_json::Value::Bool(value.parse::<bool>().unwrap_or(false)),
                _ => serde_json::Value::String(value.to_string()),
            };
            fields.insert(field.name.clone(), json_value);
            current_pos = field.position + field.size -1;
        }
        records.push(Record { fields });
    }
    records
}

/// Sends the JSON output to a message queue.
///
/// # Parameters
///
/// - `json_output`: The JSON string containing the records.
///
/// # Example
///
/// ```
/// send_to_queue(&json_output);
/// ```
async fn send_to_queue(json_output: &str) {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_|"amqp://127.0.0.1:5672/%2f".into());
    let conn = Connection::connect(&addr, ConnectionProperties::default()).await.expect("Connection error");

    let channel = conn.create_channel().await.expect("Create channel error");
    channel.queue_declare("records"
                         , QueueDeclareOptions::default()
                         , FieldTable::default(),
    ).await.expect("Queue declare error");

    channel.basic_publish(""
                         , "records"
                         , BasicPublishOptions::default()
                         , json_output.as_bytes()
                         , BasicProperties::default().with_delivery_mode(1),
    ).await.expect("Basic publish error");

}

/// Saves the JSON output to a MongoDB database.
///
/// # Parameters
///
/// - `json_output`: The JSON string containing the records.
///
/// # Example
///
/// ```
/// save_to_mongodb(&json_output);
/// ```
fn save_to_mongodb(json_output: &str) {
    // Implement the logic to save the JSON output to a MongoDB database
}

#[tokio::main]
async fn main() {
    // Check parameters
    if env::args().len() < 2 {
        println!("Program requires two arguments <layout file> <data file>");
        return
    }

    // Reading the parameters
    let layout_file: String = env::args().nth(1).unwrap();
    let data_file: String = env::args().nth(2).unwrap();
    let send_records_to_queue: String = env::args().nth(3).unwrap();

    // Reads configuration and data files
    let layout = read_config_json(&layout_file).expect("Unable to read layout file");

    let records = if layout.file_type == "csv" {
        read_csv_data(&data_file, &layout)
    } else if layout.file_type == "fixed" {
        read_fixed_data(&data_file, &layout)
    } else {
        println!("Undefined file type");
        return;
    };
    
    // Convert records to json format
    let json_records: Vec<_> = records.iter().map(|record| json!(record.fields)).collect();
    let json_output = serde_json::to_string_pretty(&json_records).unwrap();
    println!("{}", json_output);
    
    if send_records_to_queue == "yes" {
        // Convert records to JSON format and send to RabbitMQ queue
        println!("Sent records to RabbitMQ queue");
        for record in records {
            let json_record = serde_json::to_string(&record.fields).unwrap();
            send_to_queue(&json_record).await;
        }
    }

}
