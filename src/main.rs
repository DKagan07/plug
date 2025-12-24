use core::fmt;
use inquire::Select;
use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use std::collections::HashMap;
use sysinfo::{Pid, System};

#[derive(Debug, Clone)]
enum ProtocolInfo {
    TCP,
    UDP,
}

enum Choices {
    Kill,
    ViewDetails,
}

impl fmt::Display for Choices {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Choices::Kill => write!(f, "Kill"),
            Choices::ViewDetails => write!(f, "View Details"),
        }
    }
}

fn create_choices_vec() -> Vec<Choices> {
    vec![Choices::Kill, Choices::ViewDetails]
}

#[derive(Debug, Clone)]
struct PortInfo {
    port_number: u16,
    pid: u32,
    process_name: String,
    protocol: ProtocolInfo,
    port_status: String,
}

impl fmt::Display for PortInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{} -- {} Status: {} -- Protocol: {:?}",
            self.pid, self.port_number, self.process_name, self.port_status, self.protocol
        )
    }
}

// TODO: ***********************************************************************
// TODO: REALLY FLESH OUT THE PORT PART FIRST, MAKE IT AWESOME, THEN WORK ON
// TODO: THE PROCESS PART OF THE PROGRAM
// TODO: ***********************************************************************

#[derive(Debug)]
struct Manager {
    // Port-part of the Manager
    port_infos: Vec<PortInfo>,
    by_port: HashMap<u16, Vec<usize>>,    // port -> socket indices
    by_process: HashMap<u32, Vec<usize>>, // pid -> socket indices
}
// TODO: Process-part of the Manager
// process_info: Vec<sysinfo::Process>,

impl Manager {
    fn new() -> Manager {
        Manager {
            port_infos: vec![],
            by_port: HashMap::new(),
            by_process: HashMap::new(),
            // process_info: vec![],
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
                None => continue,
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

    let selection = Select::new(
        "List of processes:\nPid:Port -- Name -- Status -- Protocol",
        manager.port_infos,
    )
    .prompt();

    match selection {
        Ok(choice) => handle_selected(choice), // functionality goes here
        Err(_) => println!("there was an error, please try again"),
    };
}

fn handle_selected(picked: PortInfo) {
    // let choices = vec!["Kill", "ViewDetails"];

    let selection = Select::new(
        format!(
            "What would you like to do with {:?}:{:?}?",
            picked.process_name, picked.port_number,
        )
        .as_str(),
        create_choices_vec(),
    )
    .prompt();

    match selection {
        Ok(choice) => handle_event(choice, picked),
        Err(_) => println!("there was an error picking a choice"),
    }
}

fn handle_event(event: Choices, info: PortInfo) {
    match event {
        Choices::Kill => println!("kill: {}", info.process_name),
        Choices::ViewDetails => println!("view details: {}", info.process_name),
    };
}
