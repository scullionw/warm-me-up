use std::net::UdpSocket;

use byteorder::{LittleEndian, ReadBytesExt};
use hex_literal::*;
use std::io::Cursor;
use std::process::Command;
use structopt::StructOpt;
use std::thread;
use std::time::Duration;

const FFA9: &str = "104.153.107.120:27015";

fn main() {
    let config = Config::from_args();
    
    match config.server_addr {
        Some(addr) => {
            loop {
                let r = server_query(&addr).unwrap();
                if dbg!(r.players) < 16 {
                    println!("Joining!");
                    server_connect(&addr).unwrap();
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
        },
        None => {
            unimplemented!();
        }
    } 
}

fn server_query(addr: &str) -> std::io::Result<ServerResponse> {
    let query_data =
        hex!("ff ff ff ff 54 53 6f 75 72 63 65 20 45 6e 67 69 6e 65 20 51 75 65 72 79 00");

    let socket = UdpSocket::bind("0.0.0.0:25111").expect("Could not bind address");

    socket
        .send_to(&query_data, addr)
        .expect("couldn't send data");

    let mut buf = [0; 1024];
    let (amt, _) = socket.recv_from(&mut buf).expect("Error receiving data");

    let response = &buf[..amt];

    Ok(ServerResponse::from_data(response))
}

fn server_connect(addr: &str) -> Result<(), &'static str> {
    let launch_command = "steam://rungame/730/76561202255233023/+connect%20";

    let full_command = ["start", " ", launch_command, addr].join("");

    let output = Command::new("cmd")
        .args(&["/C", &full_command])
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        Ok(())
    } else {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Err("connect to server failed")
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
}

impl ServerResponse {
    fn from_data(data: &[u8]) -> ServerResponse {
        let mut rdr = Cursor::new(data);

        let header = rdr.read_u8().unwrap();
        let protocol = rdr.read_u8().unwrap();

        let mut name = Vec::new();
        let name = loop {
            let b = rdr.read_u8().unwrap();
            if b != 0x00 {
                name.push(b)
            } else {
                break String::from_utf8_lossy(&name).to_string();
            }
        };

        let mut map = Vec::new();
        let map = loop {
            let b = rdr.read_u8().unwrap();
            if b != 0x00 {
                map.push(b)
            } else {
                break String::from_utf8_lossy(&map).to_string();
            }
        };

        let mut folder = Vec::new();
        let folder = loop {
            let b = rdr.read_u8().unwrap();
            if b != 0x00 {
                folder.push(b)
            } else {
                break String::from_utf8_lossy(&folder).to_string();
            }
        };

        let mut game = Vec::new();
        let game = loop {
            let b = rdr.read_u8().unwrap();
            if b != 0x00 {
                game.push(b)
            } else {
                break String::from_utf8_lossy(&game).to_string();
            }
        };

        let id = rdr.read_u16::<LittleEndian>().unwrap();
        let players = rdr.read_u8().unwrap();
        let max_players = rdr.read_u8().unwrap();

        ServerResponse {
            header,
            protocol,
            name,
            map,
            folder,
            game,
            id,
            players,
            max_players,
        }
    }
}


#[derive(StructOpt)]
struct Config {
    #[structopt(short = "a")]
    /// Find best server
    all: bool,

 
    server_addr: Option<String>,
}