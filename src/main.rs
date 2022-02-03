#![feature(once_cell)]
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{
    fmt::Display,
    io::Write,
    lazy::SyncOnceCell,
    net::{TcpListener, TcpStream, UdpSocket},
    str, thread, time,
};

#[derive(Deserialize)]
struct Config {
    receiver: Addr,
    failover: Addr,
}

#[derive(Deserialize)]
struct Addr {
    ip: String,
    port: String,
}

impl Display for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    count: u64,
    app_id: String,
    node_id: String,
}

static FAILOVER_ADDR: SyncOnceCell<String> = SyncOnceCell::new();
static RECEIVER_ADDR: SyncOnceCell<String> = SyncOnceCell::new();
static COUNT: AtomicU64 = AtomicU64::new(0);

fn main() {
    let config: Config =
        toml::from_str(include_str!("../Config.toml")).unwrap();
    let args = std::env::args().collect::<Vec<String>>();
    let mode = args.get(1).expect("Missing mode argument");

    FAILOVER_ADDR.get_or_init(|| config.failover.to_string());
    RECEIVER_ADDR.get_or_init(|| config.receiver.to_string());

    match &mode[..] {
        "Sender" => send("Master"),
        "Failover" => failover(),
        "Receiver" => receive(),
        _ => println!("Invalid mode {}", mode),
    }
}

fn receive() {
    // Bind to sockect
    let listener = TcpListener::bind(RECEIVER_ADDR.get().unwrap())
        .expect("could not bind to receiver address");
    // accept connections and process them
    for stream in listener.incoming() {
        // Extract payload
        let payload = Payload::deserialize(
            &mut serde_json::Deserializer::from_reader(stream.unwrap()),
        )
        .unwrap();
        println!("{:#?}", payload);
    }
}

fn send(node_id: &str) {
    // Set handler fot Ctrl-C
    ctrlc::set_handler(move || {
        heartbeat(&format!("fail:{}", COUNT.load(Ordering::SeqCst)));
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    loop {
        // Connect to socket
        let mut stream =
            TcpStream::connect(&RECEIVER_ADDR.get().unwrap()).unwrap();
        let payload = Payload { count: COUNT.load(Ordering::SeqCst),
                                app_id: "Sender".to_string(),
                                node_id: node_id.to_string() };
        // Send Payload
        stream.write_all(serde_json::to_string(&payload).unwrap().as_bytes())
              .unwrap();
        stream.flush().unwrap();
        // Send hearbeat to Failover System
        heartbeat(&format!("success:{}", COUNT.load(Ordering::SeqCst)));
        COUNT.fetch_add(1, Ordering::SeqCst);
        // Sleep for 1 sec
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn failover() {
    // Creates a UDP socket from the given address.
    let socket = UdpSocket::bind(FAILOVER_ADDR.get().unwrap())
        .expect("could not bind to failover address");
    let mut buf = [0; 64];
    let failover = Arc::new(Mutex::new(false));
    let failover_cloned = Arc::clone(&failover);

    thread::spawn(move || loop {
        loop {
            thread::sleep(Duration::from_secs(2));
            let mut fail = failover_cloned.lock().unwrap();
            if *fail {
                COUNT.fetch_add(2, Ordering::SeqCst);
                send("Slave");
            } else {
                *fail = true;
            }
        }
    });

    loop {
        let (size, _) = socket.recv_from(&mut buf).unwrap();
        let message = str::from_utf8(&buf[..size]).unwrap()
                                                  .split(':')
                                                  .collect::<Vec<&str>>();
        match message[..] {
            [status, cnt] => {
                let cnt = cnt.parse::<u64>().unwrap();
                COUNT.store(cnt, Ordering::SeqCst);
                if status == "fail" {
                    // Activate Failover System
                    send("Slave")
                } else {
                    let mut fail = failover.lock().unwrap();
                    *fail = false;
                }
            }
            _ => println!("Invalid message format"),
        }
    }
}

fn heartbeat(message: &str) {
    let socket = UdpSocket::bind("127.0.0.1:3400")
        .expect("could not bind to failover address");
    socket.send_to(message.as_bytes(), FAILOVER_ADDR.get().unwrap())
          .unwrap();
}