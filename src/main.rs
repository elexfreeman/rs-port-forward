// Утилита проброса TCP-портов на Tokio.
// Читает JSON‑конфиг, поднимает слушатели на локальных портах и
// двунаправленно проксирует данные к удалённым адресам/портам.
// В каждом направлении применён таймаут простоя: если чтение не
// происходит дольше указанного срока — соединение закрывается.
use chrono::Local;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io::BufReader;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

/// Описание одного правила проброса порта.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigConnect {
    /// Имя правила (для удобства в логах).
    name: String,
    /// Локальный порт, на котором слушаем входящие соединения.
    local_port: u16,
    /// Удалённый порт, куда проксируем данные.
    remote_port: u16,
    /// Удалённый адрес (IP или DNS‑имя), куда идёт проброс.
    remote_address: String,
    /// Таймаут простоя в секундах. Если не указан — используется значение по умолчанию.
    idle_timeout_seconds: Option<u64>,
}

/// Корневой объект конфигурации: набор правил проброса.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    connect_list: Vec<ConfigConnect>,
}

/// Возвращает путь к конфигу, если он передан через аргументы `--config <path>`.
fn get_config_file(args: &[String]) -> Option<String> {
    if let Some(index) = args.iter().position(|arg| arg == "--config") {
        if index + 1 < args.len() {
            return Some(args[index + 1].clone());
        }
    }
    None
}

/// Загружает конфигурацию из JSON‑файла.
/// Приоритет путей:
/// 1) Значение после `--config` в аргументах.
/// 2) По умолчанию: `./rs-port-forward.config.json` (Windows) или `/etc/rs-port-forward.config.json` (Unix).
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

/// Печатает список правил проброса для наглядности при старте.
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

/// Обрабатывает одно клиентское соединение: устанавливает исходящее подключение к
/// удалённому адресу и двунаправленно проксирует данные. На чтение в каждом
/// направлении наложен `idle_timeout`.
async fn handle_connection(
    name: String,
    from: TcpStream,
    remote_address: String,
    remote_port: u16,
    idle_timeout: Duration,
) {
    let from_peer = from.peer_addr().ok();
    match TcpStream::connect(format!("{}:{}", remote_address, remote_port)).await {
        Ok(to) => {
            let (mut from_reader, mut from_writer) = from.into_split();
            let (mut to_reader, mut to_writer) = to.into_split();

            // Byte counters
            let mut bytes_from_to: u64 = 0;
            let mut bytes_to_from: u64 = 0;

            println!(
                "{} {} New connection: from {:?} -> {}:{}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                name,
                from_peer,
                remote_address,
                remote_port
            );

            // Два направления копирования:
            // - client -> remote (buf_a)
            // - remote -> client (buf_b)
            // Каждое чтение обёрнуто в `timeout(..)`. При истечении таймаута
            // возвращаем ошибку `TimedOut`, что приводит к закрытию соединения.
            let mut buf_a = vec![0u8; 8192];
            let mut buf_b = vec![0u8; 8192];

            let a_to_b = async {
                loop {
                    let n = match timeout(idle_timeout, from_reader.read(&mut buf_a)).await {
                        Ok(Ok(n)) => n,
                        Ok(Err(e)) => return Err::<(), io::Error>(e),
                        Err(_) => {
                            eprintln!("{} {} Connection timeout (client->remote)", Local::now().format("%Y-%m-%d %H:%M:%S"), name);
                            return Err::<(), io::Error>(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "idle timeout (client->remote)",
                            ))
                        }
                    };
                    // n == 0 означает EOF: клиент закрыл соединение.
                    if n == 0 {
                        return Ok::<(), io::Error>(());
                    }
                    bytes_from_to += n as u64;
                    to_writer.write_all(&buf_a[..n]).await?;
                }
            };

            let b_to_a = async {
                loop {
                    let n = match timeout(idle_timeout, to_reader.read(&mut buf_b)).await {
                        Ok(Ok(n)) => n,
                        Ok(Err(e)) => return Err::<(), io::Error>(e),
                        Err(_) => {
                            eprintln!("{} {} Connection timeout (remote->client)", Local::now().format("%Y-%m-%d %H:%M:%S"), name);
                            return Err::<(), io::Error>(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "idle timeout (remote->client)",
                            ))
                        }
                    };
                    // n == 0 означает EOF: удалённая сторона закрыла соединение.
                    if n == 0 {
                        return Ok::<(), io::Error>(());
                    }
                    bytes_to_from += n as u64;
                    from_writer.write_all(&buf_b[..n]).await?;
                }
            };

            // Гонка направлений: закрываем соединение при завершении любого из них
            // (EOF/ошибка/таймаут). Второе направление завершится вследствие закрытия сокетов.
            tokio::select! {
                res = a_to_b => {
                    match res {
                        Ok(_) => (),
                        Err(e) => eprintln!("{} {} Connection closed (client->remote): {}", Local::now().format("%Y-%m-%d %H:%M:%S"), name, e),
                    }
                }
                res = b_to_a => {
                    match res {
                        Ok(_) => (),
                        Err(e) => eprintln!("{} {} Connection closed (remote->client): {}", Local::now().format("%Y-%m-%d %H:%M:%S"), name, e),
                    }
                }
            }

            println!(
                "{} {} Disconnected {:?} -> {}:{} | bytes c->r: {}, r->c: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                name,
                from_peer,
                remote_address,
                remote_port,
                bytes_from_to,
                bytes_to_from
            );
        }
        Err(err) => {
            eprintln!(
                "{} {} Error: {}",
                Local::now().format("%A, %B %e %Y, %I:%M:%S %p"),
                name,
                err
            );
        }
    }
}

/// Поднимает TCP‑слушатель на `local_port` и создаёт задачу `handle_connection`
/// для каждого входящего подключения. Таймаут берётся из `idle_timeout_seconds`
/// или используется значение по умолчанию.
async fn port_forward(config_connect: &ConfigConnect) -> io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config_connect.local_port)).await?;

    println!(
        "Proxy start {} at {} to {}:{}",
        config_connect.name,
        config_connect.local_port,
        config_connect.remote_address,
        config_connect.remote_port
    );

    loop {
        match listener.accept().await {
            Ok((from, _)) => {
                let remote_address_clone = config_connect.remote_address.clone();
                // Таймаут простоя на чтение в секундах; дефолт — 10 сек.
                let idle = Duration::from_secs(config_connect.idle_timeout_seconds.unwrap_or(10));
                tokio::spawn(handle_connection(
                    config_connect.name.clone(),
                    from,
                    remote_address_clone,
                    config_connect.remote_port,
                    idle,
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
    // Загружаем конфиг (panic при ошибке чтения/парсинга).
    let config = load_config().unwrap();
    // Канал зарезервирован под возможные сообщения (пока не используется).
    let (_tx, mut rx) = mpsc::channel::<String>(32);
    // Выводим список правил проброса.
    print_config();
    for item in config.connect_list.iter().enumerate() {
        // Подготавливаем копию параметров для задачи прослушивания.
        let config_connect = ConfigConnect {
            local_port: item.1.local_port,
            remote_port: item.1.remote_port,
            remote_address: item.1.remote_address.clone(),
            name: item.1.name.clone(),
            idle_timeout_seconds: item.1.idle_timeout_seconds,
        };
        tokio::spawn(async move {
            // Запускаем бесконечный цикл accept + spawn.
            let _ = port_forward(&config_connect).await;
        });
    }
    // Ждём сообщений (блокирующая точка удерживает main живым).
    while let Some(message) = rx.recv().await {
        println!("GOT = {}", message);
    }
}
