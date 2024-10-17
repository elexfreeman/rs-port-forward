use chrono::Local;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io::BufReader;
use tokio::io::{self};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigConnect {
    name: String,
    local_port: u16,
    remote_port: u16,
    remote_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    connect_list: Vec<ConfigConnect>,
}

fn get_config_file(args: &[String]) -> Option<String> {
    if let Some(index) = args.iter().position(|arg| arg == "--config") {
        if index + 1 < args.len() {
            return Some(args[index + 1].clone());
        }
    }
    None
}

fn load_config() -> Result<Config, std::io::Error> {
    let mut config_file_name = String::from("rs-port-forward.config.json");
    let mut config_file_path = String::from("");

    let args: Vec<String> = env::args().collect();
    let config_file_from_args = get_config_file(&args);

    if config_file_from_args.is_some() {
        config_file_name = config_file_from_args.unwrap();
    } else {
        if !cfg!(target_os = "windows") {
            config_file_path = String::from("/etc/");
        }
    }

    let file_path = config_file_path + &config_file_name;
    println!("Use config: {:?}", file_path);

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}

fn print_config() {
    let config = load_config().unwrap();
    println!("Connection list:");
    for (index, item) in config.connect_list.iter().enumerate() {
        println!(
            "{} | Connection: {} >> local_port: {}, remote host:  {}, remote port: {}",
            index + 1,
            item.name,
            item.local_port,
            item.remote_address,
            item.remote_port
        );
    }
}

pub fn empty_string() -> std::string::String {
    String::from("")
}

async fn handle_connection(mut from: TcpStream, remote_address: String, remote_port: u16) {
    match TcpStream::connect(format!("{}:{}", remote_address, remote_port)).await {
        Ok(mut to) => {
            let (mut from_reader, mut from_writer) = from.split();
            let (mut to_reader, mut to_writer) = to.split();

            let from_to_to = async { io::copy(&mut from_reader, &mut to_writer).await };

            let to_to_from = async { io::copy(&mut to_reader, &mut from_writer).await };

            tokio::join!(from_to_to, to_to_from);
        }
        Err(err) => {
            eprintln!(
                "{} Error: {}",
                Local::now().format("%A, %B %e %Y, %I:%M:%S %p"),
                err
            );
        }
    }
}

async fn port_dorward(config_connect: &ConfigConnect) -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config_connect.local_port)).await?;

    println!(
        "Proxy start at {} to {}:{}",
        config_connect.local_port, config_connect.remote_address, config_connect.remote_port
    );

    loop {
        match listener.accept().await {
            Ok((from, _)) => {
                let remote_address_clone = config_connect.remote_address.clone();
                tokio::spawn(handle_connection(
                    from,
                    remote_address_clone,
                    config_connect.remote_port,
                ));
            }
            Err(err) => {
                eprintln!("Error accepting connection: {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let config = load_config().unwrap();
    let (_tx, mut rx) = mpsc::channel::<String>(32);
    print_config();
    for item in config.connect_list.iter().enumerate() {
        let config_connect = ConfigConnect {
            local_port: item.1.local_port,
            remote_port: item.1.remote_port,
            remote_address: item.1.remote_address.clone(),
            name: item.1.name.clone(),
        };
        tokio::spawn(async move {
            port_dorward(&config_connect).await;
        });
    }
    while let Some(message) = rx.recv().await {
        println!("GOT = {}", message);
    }
}
