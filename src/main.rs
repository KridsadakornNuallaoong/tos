use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use rumqttd::{Broker, Config};
use tokio::time::{sleep, Duration};
use mongodb::{bson::{Bson, Document}, Client, options::ClientOptions};
use std::{net::Ipv4Addr, process::{exit}, sync::Arc};
use std::env;
use log::{info, error};
use flexi_logger::{Logger, FileSpec, WriteMode, Duplicate};
use flexi_logger::DeferredNow;
use log::Record;
use std::path::Path;

use crate::dbmanager::insert_data;
mod dbmanager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_path = Path::new("env/dev.env");
    dotenv::from_path(env_path).ok();

    let tos_log_path: String = env::var("TOS_LOG_PATH").unwrap_or_else(|_| {
                    eprintln!("Error: ENV:TOS_LOG_PATH must be set and non-empty.");
                    exit(1);
                }).to_string();

    let tos_log_basename: String = env::var("TOS_LOG_BASENAME").unwrap_or_else(|_| {
                    eprintln!("Error: ENV:TOS_LOG_BASENAME must be set and non-empty.");
                    exit(1);
                }).to_string();
    
    let tos_log_suffix: String = env::var("TOS_LOG_SUFFIX").unwrap_or_else(|_| {
                    eprintln!("Error: ENV:TOS_LOG_SUFFIX must be set and non-empty.");
                    exit(1);
                }).to_string();
    
                
    Logger::try_with_str("info,rumqttd=info")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory(&tos_log_path)      // directory
                .basename(&tos_log_basename)  // fixed file name
                .suffix(&tos_log_suffix)           // file extension
        )
        .append()                      // append to existing file
        .write_mode(WriteMode::Direct)
        .format(|writer: &mut dyn std::io::Write, now: &mut DeferredNow, record: &Record| {
            write!(
                writer,
                "{} [{}] - {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args(),
            )
        })
        .duplicate_to_stderr(Duplicate::Info) // also print to console
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed: {}", e));

    env::var("TOS_IPADDRESS").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_IPADDRESS must be set and non-empty.");
        exit(1);
    });

    let port: u16 = env::var("TOS_PORT") 
        .expect("TOS_PORT must be set")         
        .parse()                                
        .expect("TOS_PORT must be a valid number");

    if port == 0 {
        error!("Error: ENV:PORT is must be set non-empty")
    }

    std::panic::set_hook(Box::new(|info| {
        if !info.to_string().contains("console.rs") {
            error!("{}", info);
        }
    }));

    // --- Start MQTT Broker in background ---
    std::thread::spawn(move || {
        let broker_config = Config::default();
        let mut broker = Broker::new(broker_config);
        info!("OK: MQTT broker running on {}", env::var("TOS_IPADDRESS").unwrap().to_string());
        if let Err(e) = broker.start() {
            error!("Error: Broker error: {:?}", e);
        }
    });


    // Give the broker a second to start
    sleep(Duration::from_secs(1)).await;

    let mongo_protocol = env::var("TOS_CON_MONGODB_PTC").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_PTC must be set and non-empty.");
        exit(1);
    });

    let mongo_ipaddress = env::var("TOS_CON_MONGODB_IPADDRESS").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_IPADDRESS must be set and non-empty.");
        exit(1);
    });

    let mongo_port = env::var("TOS_CON_MONGODB_PORT").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_PORT must be set and non-empty.");
        exit(1);
    });
    
    let mongo_db_name = env::var("TOS_CON_MONGODB_DB_NAME").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_DB_NAME must be set and non-empty.");
        exit(1);
    });

    let mongo_user = env::var("TOS_CON_MONGODB_USERNAME").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_USERNAME must be set and non-empty.");
        exit(1);
    });

    let mongo_password = env::var("TOS_CON_MONGODB_PASSWORD").unwrap_or_else(|_| {
        error!("Error: ENV:TOS_CON_MONGODB_PASSWORD must be set and non-empty.");
        exit(1);
    });


    // --- Setup MongoDB connection (optional) ---
    let mongo_uri = format!("{}://{}:{}@{}:{}", &mongo_protocol, &mongo_user, &mongo_password, &mongo_ipaddress, &mongo_port);
    let client_options = ClientOptions::parse(mongo_uri).await?;
    let mongo_client = Arc::new(Client::with_options(client_options)?);
    let db = mongo_client.database(&mongo_db_name);
    // let collection = db.collection::<SensorData>("messages");

    // --- MQTT Client Setup ---
    let mut mqttoptions = MqttOptions::new(
        "Tiny-Orga-Server", 
        env::var("TOS_IPADDRESS").unwrap_or(Ipv4Addr::LOCALHOST.to_string()),
        env::var("TOS_PORT").expect("env must be set").parse().expect("Invalid number")); // panic if invalid;
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client.subscribe("sensors/+", QoS::AtLeastOnce).await?;
    info!("OK: Subscribed to topic sensors/+ ...");

    // --- Event Loop ---
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(rumqttc::Packet::Publish(p))) => {
                let payload = String::from_utf8_lossy(&p.payload);
                let d_path: Vec<&str>= p.topic.split("/").collect();

                let payload_json: Document = match serde_json::from_str(&payload) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Error: Failed to parse JSON: {:?}", e);
                        continue;
                    }
                };

                // Validate required fields
                let device_id = payload_json
                    .get("deviceID")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty());

                let device_type = payload_json
                    .get("type")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty());

                let data = payload_json
                    .get("data")
                    .and_then(|v: &Bson| v.as_document())
                    .filter(|m: &&Document| !m.is_empty());

                if device_id.is_none() || device_type.is_none() || data.is_none() {
                    error!(
                        "Error: Payload missing required fields or 'data' is null/empty: {:?}",
                        payload_json
                    );
                    continue;
                }

                if d_path[1] != device_type.unwrap_or("") {
                    eprintln!("Device type and path type not match data has no collected");
                    continue;
                }

                insert_data(&payload_json);
                println!(
                    "Data received topic: \"{}\", FROM: {}, TYPE: {}",
                    &p.topic,
                    &payload_json.get("deviceID").map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                    &payload_json.get("type").map(|v| v.to_string()).unwrap_or("N/A".to_string())
                );

                if let Err(e) = db.collection::<Document>(&d_path[1]).insert_one(&payload_json).await {
                    error!("Error: Failed to insert into MongoDB: {}", e);
                }
                println!(
                    "Data collected FROM: {}, TYPE: {}",
                    &payload_json.get("deviceID").map(|v| v.to_string()).unwrap_or("N/A".to_string()),
                    &payload_json.get("type").map(|v| v.to_string()).unwrap_or("N/A".to_string())
                );
            }
            Ok(_) => {}
            Err(e) => {
                error!("Error: MQTT Error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}