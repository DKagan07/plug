use inquire::Select;
use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use std::collections::HashMap;
use sysinfo::{Pid, System};

#[derive(Debug, Clone)]
enum ProtocolInfo {
    TCP,
    UDP,
}

#[derive(Debug, Clone)]
struct PortInfo {
    port_number: u16,
    pid: u32,
    process_name: String,
    protocol: ProtocolInfo,
    port_status: String,
}

#[derive(Debug, Clone)]
struct Manager {
    port_infos: Vec<PortInfo>,

    by_port: HashMap<u16, Vec<usize>>,    // port -> socket indices
    by_process: HashMap<u32, Vec<usize>>, // pid -> socket indices
}

impl Manager {
    fn new() -> Manager {
        Manager {
            port_infos: vec![],
            by_port: HashMap::new(),
            by_process: HashMap::new(),
        }
    }
}

fn main() {
    let address_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocol_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let socket_info = match netstat2::get_sockets_info(address_flags, protocol_flags) {
        Ok(socket_info) => socket_info,
        Err(err) => panic!("error getting socket info: {err:?}"),
    };

    let mut sysinfo = System::new();
    sysinfo.refresh_all();

    let proc = sysinfo.processes();

    let mut manager = Manager::new();
    let mut i = 0;

    for socket in socket_info.clone() {
        for assoc_pid in socket.associated_pids.clone() {
            let process = match proc.get(&Pid::from_u32(assoc_pid)) {
                Some(p) => p,
                None => return,
            };

            let (protocol, state) = match &socket.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => (ProtocolInfo::TCP, tcp.state.to_string()),
                ProtocolSocketInfo::Udp(_) => (ProtocolInfo::UDP, String::from("N/A")),
            };

            let port_info = PortInfo {
                port_number: socket.local_port(),
                pid: assoc_pid,
                process_name: process.name().to_string_lossy().to_string(),
                protocol: protocol,
                port_status: state,
            };

            manager.port_infos.push(port_info);

            match manager.by_process.get_mut(&assoc_pid) {
                Some(p_ind) => p_ind.push(i),
                None => {
                    manager.by_process.insert(assoc_pid, vec![i]);
                }
            }

            match manager.by_port.get_mut(&socket.local_port()) {
                Some(l_ind) => l_ind.push(i),
                None => {
                    manager.by_port.insert(socket.local_port(), vec![i]);
                }
            }

            i += 1;
        }
    }

    let p_i = manager
        .port_infos
        .iter()
        .map(|pi| {
            format!(
                "{}:{} -- {} Status: {} -- Protocol: {:?}",
                pi.pid, pi.port_number, pi.process_name, pi.port_status, pi.protocol
            )
        })
        .collect();

    let selection = Select::new(
        "List of processes:\nPid:Port -- Name -- Status -- Protocol",
        p_i,
    )
    .prompt();

    match selection {
        Ok(choice) => println!("good choice!: {choice}"),
        Err(_) => println!("there was an error, please try again"),
    };
}
