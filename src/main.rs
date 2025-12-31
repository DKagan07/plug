use chrono::{DateTime, Local, TimeZone, Utc};
use core::fmt;
use inquire::Select;
use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, Process, System};

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

#[derive(Debug)]
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

impl Clone for PortInfo {
    fn clone(&self) -> Self {
        PortInfo {
            port_number: self.port_number.clone(),
            pid: self.pid.clone(),
            process_name: self.process_name.clone(),
            protocol: self.protocol.clone(),
            port_status: self.port_status.clone(),
        }
    }
}

impl PortInfo {
    fn display_specs(&self, proc: &Process) {
        let local = Local::now();
        let start_time: DateTime<Utc> = Utc.timestamp_opt(proc.start_time() as i64, 0).unwrap();
        let tz = local.timezone();
        let current_time = start_time.with_timezone(&tz);

        println!("in display specs!");
        println!("Port number: {}", self.port_number);
        println!("Port status: {}", self.port_status);
        println!("Memory Usage: {} bytes", proc.memory());
        println!("CPU Usage: {}%", proc.cpu_usage());
        println!("Run time: {}", human_readable_date(proc.run_time()));
        println!("Start time: {} UTC", current_time);
        println!("Command: {:?}", proc.cmd());
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
    system_info: System,
}
// TODO: Process-part of the Manager
// process_info: Vec<sysinfo::Process>,

impl Manager {
    fn new() -> Manager {
        Manager {
            port_infos: vec![],
            by_port: HashMap::new(),
            by_process: HashMap::new(),
            system_info: System::new(),
            // process_info: vec![],
        }
    }

    fn handle_selected(self, picked: PortInfo) {
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
            Ok(choice) => self.handle_event(choice, picked),
            Err(_) => println!("there was an error picking a choice"),
        }
    }

    fn handle_event(self, event: Choices, picked: PortInfo) {
        let process = match self.system_info.process(Pid::from_u32(picked.pid)) {
            Some(process) => process,
            None => return,
        };

        match event {
            Choices::Kill => {
                self.kill_process_by_pid(picked.pid, process);
                println!("kill: {}", picked.process_name);
            }
            Choices::ViewDetails => {
                println!("{}", picked.process_name);
                picked.display_specs(process);
            }
        };
    }

    fn kill_process_by_pid(&self, pid: u32, process: &Process) -> bool {
        println!("found process to kill:");
        println!("process: {:?}", process.name());
        println!("process pid: {}", pid);
        println!("process runtime: {:?}", process.run_time());
        println!("process disk usage: {:?}", process.disk_usage());

        process.kill()
    }

    fn kill_process_by_port(self, port: u16) {
        // need to get processes associated with the port
        let list_of_indexes_to_port_infos = match self.by_port.get(&port) {
            Some(list) => list,
            None => return,
        };

        let mut unique_pids = HashSet::new();
        for index in list_of_indexes_to_port_infos {
            unique_pids.insert(self.port_infos[*index].clone().pid);
        }

        for pid in unique_pids {
            let process = match self.system_info.process(Pid::from_u32(pid)) {
                Some(process) => process,
                None => return,
            };

            let success = self.kill_process_by_pid(pid, process);
            if !success {
                println!("failed to send kill message for pid: {}", pid)
            }
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

    // let mut sysinfo = System::new();
    let mut manager = Manager::new();
    manager.system_info.refresh_all();

    let proc = manager.system_info.processes();
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
        manager.port_infos.clone(),
    )
    .prompt();

    match selection {
        Ok(choice) => manager.handle_selected(choice), // functionality goes here
        Err(_) => println!("there was an error, please try again"),
    };
}

fn human_readable_date(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    match (days, hours, minutes, seconds) {
        (0, 0, 0, s) => format!("{s}s"),
        (0, 0, m, s) => format!("{m}m {s}s"),
        (0, h, m, s) => format!("{h}h {m}m {s}s"),
        (d, h, m, s) => format!("{d}d {h}h {m}m {s}s"),
    }
}
