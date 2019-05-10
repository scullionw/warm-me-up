use std::net::UdpSocket;

use byteorder::{LittleEndian, ReadBytesExt};
use hex_literal::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::Command;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;

const FFA1: &str = "72.5.195.76:27015";
const FFA2: &str = "74.91.125.129:27015";
const FFA3: &str = "74.201.57.120:27015";
const FFA4: &str = "74.91.114.102:27015";
const FFA7: &str = "104.153.107.122:27015";
const FFA8: &str = "104.153.107.121:27015";
const FFA9: &str = "104.153.107.120:27015";
const BIND_ADDR: &str = "0.0.0.0:25111";
const POLL_TIME: u64 = 1;
const FRAGSHACK_SERVERS: [&str; 7] = [FFA1, FFA2, FFA3, FFA4, FFA7, FFA8, FFA9];

lazy_static! {
    static ref PING_RE: Regex = Regex::new(r"time=(?P<time>\d+)ms").unwrap();
}

fn main() {
    let config = Config::from_args();

    if let Some(addr) = config.server_addr {
        single_server(addr);
    } else {
        let servers = FRAGSHACK_SERVERS
            .iter()
            .map(|&x| x.parse().unwrap())
            .collect::<Vec<_>>();

        if config.show {
            show(&servers);
        } else {
            all(&servers);
        }
    }
}

fn show(servers: &[SocketAddrV4]) {
    let mut queried_servers = servers
        .iter()
        .map(|s| QueriedServer::from_addr(*s))
        .collect::<Vec<_>>();

    queried_servers.sort_by_key(|q| q.latency);
    queried_servers.iter().for_each(|q| println!("{}", q));
}

fn single_server(addr: SocketAddrV4) {
    let mut connected = false;

    while !connected {
        let queried = QueriedServer::from_addr(addr);
        println!("{}", queried);
        if queried.should_join() {
            println!("Joining!");
            queried.connect().unwrap();
            connected = true;
        }
        thread::sleep(Duration::from_secs(POLL_TIME));
    }
}

fn all(servers: &[SocketAddrV4]) {
    let mut connected = false;

    while !connected {
        let mut queried_servers = servers
            .iter()
            .map(|s| QueriedServer::from_addr(*s))
            .collect::<Vec<_>>();

        queried_servers.sort_by_key(|q| q.latency);
        queried_servers.iter().for_each(|q| println!("{}", q));

        for queried_server in queried_servers {
            if queried_server.should_join() {
                println!("Joining: {}", queried_server);
                queried_server.connect().unwrap();
                connected = true;
                break;
            }
        }
        thread::sleep(Duration::from_secs(POLL_TIME));
    }
}

struct QueriedServer {
    addr: SocketAddrV4,
    info: ServerResponse,
    latency: u32,
}

impl fmt::Display for QueriedServer {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}  {}ms  {}/{} ({})",
            self.info.name,
            self.latency,
            self.info.players - self.info.bots,
            self.info.max_players,
            self.info.bots
        )
    }
}

impl QueriedServer {
    fn from_addr(addr: SocketAddrV4) -> Self {
        Self {
            addr,
            info: server_query(addr),
            latency: ping(*addr.ip()).unwrap(),
        }
    }

    fn connect(&self) -> Result<(), &'static str> {
        let launch_command = "steam://rungame/730/76561202255233023/+connect%20";
        let full_command = ["start", " ", launch_command, &self.addr.to_string()].join("");

        let output = Command::new("cmd")
            .args(&["/C", &full_command])
            .output()
            .expect("Failed to execute game launch command");

        if output.status.success() {
            Ok(())
        } else {
            println!("status: {}", output.status);
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            Err("Connection to CS server failed")
        }
    }

    fn should_join(&self) -> bool {
        let real_players = self.info.players - self.info.bots;
        let available_slots = self.info.max_players - real_players;
        available_slots > 0 && real_players > 8
    }
}

fn server_query(server: SocketAddrV4) -> ServerResponse {
    let query_data =
        hex!("ff ff ff ff 54 53 6f 75 72 63 65 20 45 6e 67 69 6e 65 20 51 75 65 72 79 00");

    let socket = UdpSocket::bind(BIND_ADDR).expect("Could not bind address");

    socket
        .send_to(&query_data, server)
        .expect("couldn't send data");

    let mut buf = [0; 1024];
    let (amt, _) = socket
        .recv_from(&mut buf)
        .expect("Error receiving from valve");

    ServerResponse::from_data(&buf[..amt])
}

fn ping(addr: Ipv4Addr) -> Result<u32, &'static str> {
    let output = Command::new("ping")
        .args(&["-n", "1", &addr.to_string()])
        .output()
        .expect("Couldnt launch ping command");

    if output.status.success() {
        let ping_result = String::from_utf8_lossy(&output.stdout);
        let caps = PING_RE.captures(&ping_result).unwrap();
        let time: u32 = caps["time"].parse().unwrap();
        Ok(time)
    } else {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Err("Ping to server failed")
    }
}

#[derive(Debug)]
struct ServerResponse {
    header: u8,
    protocol: u8,
    name: String,
    map: String,
    folder: String,
    game: String,
    id: u16,
    players: u8,
    max_players: u8,
    bots: u8,
}

impl ServerResponse {
    fn variable_length_string(rdr: &mut std::io::Cursor<&[u8]>) -> String {
        let mut buf = Vec::new();
        loop {
            let b = rdr.read_u8().unwrap();
            if b != 0x00 {
                buf.push(b)
            } else {
                break String::from_utf8_lossy(&buf).to_string();
            }
        }
    }

    fn from_data(data: &[u8]) -> ServerResponse {
        let mut rdr = Cursor::new(data);

        ServerResponse {
            header: rdr.read_u8().unwrap(),
            protocol: rdr.read_u8().unwrap(),
            name: ServerResponse::variable_length_string(&mut rdr),
            map: ServerResponse::variable_length_string(&mut rdr),
            folder: ServerResponse::variable_length_string(&mut rdr),
            game: ServerResponse::variable_length_string(&mut rdr),
            id: rdr.read_u16::<LittleEndian>().unwrap(),
            players: rdr.read_u8().unwrap(),
            max_players: rdr.read_u8().unwrap(),
            bots: rdr.read_u8().unwrap(),
        }
    }
}

#[derive(StructOpt)]
struct Config {
    #[structopt(short = "s")]
    /// Query and list
    show: bool,

    server_addr: Option<SocketAddrV4>,
}
