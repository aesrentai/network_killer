mod interfaces;
mod socketiter;
mod streams;
mod reset;
mod netlink;

use clap::{Arg, ArgAction, Command};
use interfaces::cycle_interfaces;
use nix::unistd::Uid;
use procfs::sys::kernel::Version;
use std::error::Error;
use std::net::IpAddr;
use std::vec::Vec;

fn main() -> Result<(), Box<dyn Error>> {
    if !Uid::effective().is_root() {
        panic!("This executable requires root permissions");
    }

    let command = Command::new("Network Killer")
        .author("Cody Ho <codyho@stanford.edu>")
        .about("Kills all TCP connections and brings down all interfaces")
        .arg(Arg::new("use_reset")
             .short('r')
             .long("reset")
             .action(ArgAction::SetTrue)
             .help("Force the use of TCP/RST instead of netlink"))
        .arg(Arg::new("whitelist")
             .short('w')
             .long("whitelist")
             .action(ArgAction::Append)
             .help("IPs of connections that should not be killed"))
        .arg(Arg::new("duration")
             .short('d')
             .long("duration")
             .action(ArgAction::Set)
             .default_value("120")
             .help("Duration all interfaces should be down, default 120 seconds"))
        .get_matches();
    let use_reset = command.get_one::<bool>("use_reset").unwrap().clone();
    let duration = command.get_one::<String>("duration").unwrap().parse::<u32>().unwrap_or(120);
    let whitelist = command.get_many::<String>("whitelist")
        .unwrap_or_default()
        .filter_map(|ip| ip.parse::<IpAddr>().ok())
        .collect::<Vec<_>>();

    // netlink works on kernels 4.9+ only
    let netlink_available = match Version::current() {
        Ok(version) => version.major > 4 || (version.major == 4 && version.minor > 9),
        Err(_) => false,
    };
    let use_netlink = netlink_available && !use_reset;

    _ = kill_established_connections(use_netlink, whitelist);
    cycle_interfaces(duration);

    Ok(())
}

fn kill_established_connections(use_netlink: bool, whitelist: Vec<IpAddr>) -> Result<(), Box<dyn Error>> {
    let streams = streams::streams()?;
    for sock in socketiter::SocketFdIterator::new()? {
        let sock = sock?;
        if let Some(stream) = streams.get(&sock.inode) {
            let localip = stream.local.ip();
            let remoteip = stream.remote.ip();

            if whitelist.contains(&localip) || whitelist.contains(&remoteip) {
                continue;
            }
            if use_netlink {
                if let Err(_) = netlink::kill(
                    localip,
                    stream.local.port(),
                    remoteip,
                    stream.remote.port()
                ) {
                    // Fallback to RST if killing the socket failed
                    _ = reset::kill(sock.pid as i32, sock.fd as i32);
                }
            } else {
                _ = reset::kill(sock.pid as i32, sock.fd as i32);
            }
        }
    }
    Ok(())
}
