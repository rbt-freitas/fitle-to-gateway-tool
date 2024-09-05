# Text File Interpreter

## Project Description
The Text File Interpreter is a Rust-based project that reads data from various file formats (CSV, fixed-width text files) and publishes the data to a RabbitMQ queue or stores it in a MongoDB collection. This project is designed to handle different data types and configurations, making it flexible and adaptable to various use cases.

## Prerequisites
Before you begin, ensure you have met the following requirements:
- **Rust**: You need to have Rust installed on your machine. You can download and install Rust from rust-lang.org.
- **RabbitMQ**: You need to have RabbitMQ installed and running. You can download and install RabbitMQ from rabbitmq.com.
- **MongoDB**: You need to have MongoDB installed and running. You can download and install MongoDB from mongodb.com.

or you can use Docker to install a RabbitMQ and MongoDB in containers

- **Docker**: You need to have Docker installed to run RabbitMQ and MongoDB in containers. You can download and install Docker from docker.com.

## Installation
1. Clone the repository:
    ```sh
    git clone https://github.com/rbt-freitas/text-file-interpreter.git
    cd text-file-interpreter
    ```

2. Set up the environment variables:
    Create a `.env` file in the root of the project and add the following:
    ```env
    AMQP_ADDR=amqp://127.0.0.1:5672/%2f
    MONGODB_URI=mongodb://localhost:27017
    ```

3. Build the project:
    ```sh
    cargo build
    ```

## Running RabbitMQ and MongoDB with Docker
To run RabbitMQ and MongoDB using Docker, you can use the following commands:

1. **RabbitMQ**:
    ```sh
    docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management
    ```

2. **MongoDB**:
    ```sh
    docker run -d --name mongodb -p 27017:27017 mongo
    ```

## Usage
To run the project, use the following command: cargo run <config_file> <data_file> <queue>

    ```sh
    cargo run config/cars-config.json data/cars-data.csv no
    ```

## Configuration File Format
{
    "name": "Person data",
    "versao": "1.0",
    "fields": [
        {
            "name": "XXXXXXXXXXXXX",
            "description": "XXXXXXXXXXX",
            "position": 1,
            "size": 1,
            "field_type": "XXXXXXX"
        },
        {
            "name": "XXXXXXXXXXXXX",
            "description": "XXXXXXXXXXX",
            "position": 1,
            "size": 1,
            "field_type": "XXXXXXX"
        },
    ]
}

## Data files
The people file data was created by me using google maps data.
The data of the cars file was extracted from: https://www.kaggle.com/datasets/lainguyn123/australia-car-market-data

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request.

## License
This project is licensed under the MIT License - see the LICENSE file for details.