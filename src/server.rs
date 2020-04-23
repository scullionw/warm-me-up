use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use hex_literal::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;
use std::io::Cursor;
use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::Command;
use std::time::Duration;

lazy_static! {
    static ref PING_RE: Regex = Regex::new(r"time=(?P<time>\d+)ms").unwrap();
}

const BIND_ADDR: &str = "0.0.0.0:25111";

#[derive(Debug)]
pub struct QueriedServer {
    addr: SocketAddrV4,
    info: ServerResponse,
    pub latency: u32,
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
    pub fn from_addr(addr: SocketAddrV4) -> Result<Self> {
        Ok(Self {
            addr,
            info: server_query(addr)?,
            latency: ping(addr.ip())?,
        })
    }

    pub fn connect(&self) -> Result<()> {
        let launch_command = "steam://rungame/730/76561202255233023/+connect%20";
        let full_command = ["start", " ", launch_command, &self.addr.to_string()].join("");

        let output = Command::new("cmd").args(&["/C", &full_command]).output()?;

        if output.status.success() {
            Ok(())
        } else {
            println!("status: {}", output.status);
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            Err(anyhow!("Connection to CS server failed"))
        }
    }

    pub fn should_join(&self) -> bool {
        let real_players = self.info.players - self.info.bots;
        let available_slots = self.info.max_players - real_players;
        available_slots > 0 && real_players > 6 && self.latency < 50
    }
}

fn server_query(server: SocketAddrV4) -> Result<ServerResponse> {
    let query_data =
        hex!("ff ff ff ff 54 53 6f 75 72 63 65 20 45 6e 67 69 6e 65 20 51 75 65 72 79 00");

    let socket = UdpSocket::bind(BIND_ADDR)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;

    socket.send_to(&query_data, server)?;

    let mut buf = [0; 1024];
    let (amt, _) = socket.recv_from(&mut buf)?;

    ServerResponse::from_data(&buf[..amt])
}

fn ping(addr: &Ipv4Addr) -> Result<u32> {
    let output = Command::new("ping")
        .args(&["-n", "1", &addr.to_string()])
        .output()?;

    if output.status.success() {
        let ping_result = String::from_utf8_lossy(&output.stdout);
        let caps = PING_RE
            .captures(&ping_result)
            .ok_or_else(|| anyhow!("No captures found."))?;
        let time: u32 = caps["time"].parse()?;
        Ok(time)
    } else {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Err(anyhow!("Ping to server failed"))
    }
}

#[derive(Debug)]
pub struct ServerResponse {
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
    fn variable_length_string(rdr: &mut std::io::Cursor<&[u8]>) -> Result<String> {
        let mut buf = Vec::new();
        loop {
            let b = rdr.read_u8()?;
            if b != 0x00 {
                buf.push(b)
            } else {
                return Ok(String::from_utf8_lossy(&buf).to_string());
            }
        }
    }

    fn from_data(data: &[u8]) -> Result<ServerResponse> {
        let mut rdr = Cursor::new(data);

        Ok(ServerResponse {
            header: rdr.read_u8()?,
            protocol: rdr.read_u8()?,
            name: ServerResponse::variable_length_string(&mut rdr)?,
            map: ServerResponse::variable_length_string(&mut rdr)?,
            folder: ServerResponse::variable_length_string(&mut rdr)?,
            game: ServerResponse::variable_length_string(&mut rdr)?,
            id: rdr.read_u16::<LittleEndian>()?,
            players: rdr.read_u8()?,
            max_players: rdr.read_u8()?,
            bots: rdr.read_u8()?,
        })
    }
}
