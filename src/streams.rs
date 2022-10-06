use std::collections::HashMap;
use std::fs::read_to_string;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

macro_rules! unwrap_or_continue {
    ($opt: expr) => {
        match $opt {
            Some(v) => v,
            None => {continue;}
        }
    }
}
macro_rules! unwrap_or_return {
    ($opt: expr) => {
        match $opt {
            Ok(v) => v,
            Err(_) => {return None;}
        }
    }
}

pub struct Stream {
    pub local: SocketAddr,
    pub remote: SocketAddr,
}

pub fn streams() -> Result<HashMap<u64, Stream>, Box<dyn Error>> {
    let mut streams = HashMap::new();
    let proc_file: &str = "/proc/net/tcp";
    for line in read_to_string(proc_file)?
        .lines()
        .skip(1)
    {
        let arr: Vec<&str> = line.split_ascii_whitespace().collect();

        // Only return established connections
        if arr[3] != "01" { continue; }

        let local = unwrap_or_continue!(proc_net_tcp_ipv4_parse(arr[1]));
        let remote = unwrap_or_continue!(proc_net_tcp_ipv4_parse(arr[2]));
        let inode: u64 = match arr[9].parse() {
            Ok(v) => v,
            Err(_) => {continue;},
        };
        streams.insert(inode, Stream { local, remote });
    }

    Ok(streams)
}

fn proc_net_tcp_ipv4_parse(s: &str) -> Option<SocketAddr> {
    if s.len() != 8 + 1 + 4 {
        return None;
    }
    let (ip, port) = match s.split_once(':') {
        Some((ip, port)) => (ip, port),
        None => {return None;}
    };
    if ip.len() != 8 {
        return None;
    }
    if port.len() != 4 {
        return None;
    }
    let ip = unwrap_or_return!(u32::from_str_radix(ip, 16));
    let port = unwrap_or_return!(u16::from_str_radix(port, 16));
    Some(SocketAddr::from(SocketAddrV4::new(
        Ipv4Addr::from(u32::from_be(ip)),
        port,
    )))
}
