/*!
 * # Text-File-Interpreter
 * 
 * # Author: 
 * 
 * Roberto Freitas
 * 
 * # Description: 
 * 
 * The Text File Interpreter is a Rust-based project that reads data from various file 
 * formats (CSV, fixed-width text files) and publishes the data to a RabbitMQ queue or 
 * stores it in a MongoDB collection. This project is designed to handle different data 
 * types and configurations, making it flexible and adaptable to various use cases.
 *
 */
use std::env;
use std::error::Error;
use std::fs;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use serde::{Serialize, Deserialize};
use serde_json::json;
use lapin::{options::*, types::FieldTable, BasicProperties, Connection, ConnectionProperties};
use log::{info, error};
use env_logger;
use dotenv::dotenv;
use mongodb::{Client, options::ClientOptions};
use mongodb::bson::Document;

#[derive(Serialize, Deserialize, Debug)]
enum FieldType {
    Fixed,
    Delimited, 
}

#[derive(Serialize, Deserialize, Debug)]
struct Layout {
    name: String,
    version: usize,
    delimiter: Option<char>, 
    file_type: FieldType,
    destination: String, 
    storage_name: String, 
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
/// let records = read_csv_data("data.csv", "layout.txt");
/// ```
/// 
fn read_csv_data(file_name: &str, layout: &Layout) -> Vec<Record> {
    let file = fs::File::open(file_name).expect("Unable to open data file");
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let delimiter = layout.delimiter.unwrap_or(',');

    for line in reader.lines() {
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
async fn send_to_queue(json_output: &str, queue_name: &str) {
    let addr = std::env::var("AMQP_ADDR").expect("AMQP_ADDR not set in .env file");
    let conn = Connection::connect(&addr, ConnectionProperties::default()).await.expect("Connection error");

    let channel = conn.create_channel().await.expect("Create channel error");
    channel.queue_declare(queue_name
                         , QueueDeclareOptions::default()
                         , FieldTable::default(),
    ).await.expect("Queue declare error");

    channel.basic_publish(""
                         , queue_name
                         , BasicPublishOptions::default()
                         , json_output.as_bytes()
                         , BasicProperties::default().with_delivery_mode(1),
    ).await.expect("Basic publish error");
    info!("Sent records to RabbitMQ queue: {}", queue_name);
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
async fn save_to_mongodb(json_output: &str, collection_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client_options = ClientOptions::parse(&env::var("MONGODB_URI").expect("MONGODB_URI not set in .env file")).await?;
    let client = Client::with_options(client_options)?;
    let database = client.database("mydb");
    let collection = database.collection::<Document>(collection_name);

    let docs: Vec<mongodb::bson::Document> = serde_json::from_str(json_output)?;
    match collection.insert_many(docs, None).await {
        Ok(_) => {
            info!("Saved records to MongoDB collection: {}", collection_name);
            Ok(())
        },
        Err(e) => Err(Box::new(e))
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();

    // Check parameters
    if env::args().len() < 2 {
        error!("Program requires two arguments <layout file> <data file>");
        return
    }

    // Reading the parameters
    let layout_file: String = env::args().nth(1).unwrap();
    let data_file: String = env::args().nth(2).unwrap();

    // Reads configuration and data files
    let layout = read_config_json(&layout_file).expect("Unable to read layout file");

    let records = match layout.file_type {
        FieldType::Delimited => {
            read_csv_data(&data_file, &layout) 
        },
        FieldType::Fixed => {
            read_fixed_data(&data_file, &layout)
        }
    };
    
    // Convert records to json format
    let json_records: Vec<_> = records.iter().map(|record| json!(record.fields)).collect();
    let json_output = serde_json::to_string_pretty(&json_records).unwrap();
    println!("Processed records: {}", json_output);
    
    // Convert records to JSON format and send to RabbitMQ queue
    match layout.destination.as_str() {
        "queue" => {
            for record in records {
                let json_record = serde_json::to_string(&record.fields).unwrap();
                send_to_queue(&json_record, &layout.storage_name).await;
            }    
        }, 
        "both" => {
            for record in records {
                let json_record = serde_json::to_string(&record.fields).unwrap();
                send_to_queue(&json_record, &layout.storage_name).await;
            };    
            save_to_mongodb(&json_output, &layout.storage_name).await.unwrap();
        }
        "repository" => {
            save_to_mongodb(&json_output, &layout.storage_name).await.unwrap();
        },
        _ => error!("Invalid destination specified in config file")
    }

}
