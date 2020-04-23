mod server;

use anyhow::Result;
use server::QueriedServer;
use std::net::SocketAddrV4;
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

const FRAGSHACK_SERVERS: &[&str] = &[FFA1, FFA2, FFA3, FFA4, FFA7, FFA8, FFA9];

const POLL_TIME: u64 = 1;

fn main() -> Result<()> {
    let config = Config::from_args();

    if let Some(addr) = config.server_addr {
        single_server(addr)?;
    } else {
        let servers = FRAGSHACK_SERVERS
            .iter()
            .map(|&x| x.parse())
            .collect::<Result<Vec<_>, _>>()?;

        if config.show {
            show(&servers)?;
        } else {
            all(&servers)?;
        }
    }
    Ok(())
}

fn query(servers: &[SocketAddrV4]) -> Vec<QueriedServer> {
    let mut queried = servers
        .iter()
        .copied()
        .filter_map(|s| QueriedServer::from_addr(s).ok())
        .collect::<Vec<_>>();

    queried.sort_by_key(|q| q.latency);

    queried
}

fn show(servers: &[SocketAddrV4]) -> Result<()> {
    let queried = query(servers);
    queried.iter().for_each(|q| println!("{}", q));
    Ok(())
}

fn single_server(addr: SocketAddrV4) -> Result<()> {
    loop {
        let queried = QueriedServer::from_addr(addr)?;
        println!("{}", queried);
        if queried.should_join() {
            println!("Joining!");
            queried.connect()?;
            return Ok(());
        }
        thread::sleep(Duration::from_secs(POLL_TIME));
    }
}

fn all(servers: &[SocketAddrV4]) -> Result<()> {
    loop {
        let queried = query(servers);
        queried.iter().for_each(|q| println!("{}", q));

        for queried_server in queried {
            if queried_server.should_join() {
                println!("Joining: {}", queried_server);
                queried_server.connect()?;
                return Ok(());
            }
        }
        thread::sleep(Duration::from_secs(POLL_TIME));
    }
}

#[derive(StructOpt)]
struct Config {
    #[structopt(short = "s")]
    /// Query and list
    show: bool,

    server_addr: Option<SocketAddrV4>,
}
