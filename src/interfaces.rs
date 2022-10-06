use rtnetlink::{new_connection, Handle};
use rtnetlink::Error as RtError;
use std::error::Error;
use std::fs::read_to_string;
use std::{thread, time};
use std::vec::Vec;
use futures::TryStreamExt;

enum LinkState {
    Up,
    Down,
}

#[tokio::main]
pub async fn cycle_interfaces(sleep_duration: u32){
    let interfaces = retrieve_interfaces().unwrap();
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    for interface in interfaces.iter() {
        match link_change_state(handle.clone(), interface.to_string(), LinkState::Down).await {
            Ok(_) => {},
            Err(_) => { println!("ERROR: Failed to set {} down", interface); },
        }
    }

    thread::sleep(time::Duration::from_secs(sleep_duration.into()));

    for interface in interfaces.iter() {
        match link_change_state(handle.clone(), interface.to_string(), LinkState::Up).await {
            Ok(_) => {},
            Err(_) => { println!("ERROR: Failed to set {} down", interface); },
        }
    }

}

fn retrieve_interfaces() -> Result<Vec<String>, Box<dyn Error>> {
    let interface_file = "/proc/net/dev";
    let mut interfaces: Vec<String> = Vec::new();

    for line in read_to_string(interface_file)?
        .lines()
        .skip(2)
    {
        let interface = line.split(':')
            .next()
            .unwrap()
            .trim()
            .to_string();

        if interface == "lo" { continue; }

        interfaces.push(interface);
    }

    Ok(interfaces)
}

async fn link_change_state(handle: Handle, name: String, status: LinkState) -> Result<(), RtError> {
    let mut links = handle.link().get().match_name(name.clone()).execute();
    if let Some(link) = links.try_next().await? {
        match status{
            LinkState::Down => {
                handle
                    .link()
                    .set(link.header.index)
                    .down()
                    .execute()
                    .await?
            },
            LinkState::Up => {
                handle
                    .link()
                    .set(link.header.index)
                    .up()
                    .execute()
                    .await?
            }
        }
    }
    Ok(())
}
