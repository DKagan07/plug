use inquire::Select;
use netstat2::{AddressFamilyFlags, ProtocolFlags};

fn main() {
    let address_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocol_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let socket_info = match netstat2::get_sockets_info(address_flags, protocol_flags) {
        Ok(socket_info) => socket_info,
        Err(err) => panic!("error getting socket info: {err:?}"),
    };

    let socket_info_names: Vec<String> = socket_info
        .iter()
        .map(|socket| format!("Port :{}", socket.local_port()))
        .collect();

    let selection = Select::new("Active ports:", socket_info_names.clone()).prompt();

    match selection {
        Ok(choice) => println!("good choice!: {choice}"),
        Err(_) => println!("there was an error, please try again"),
    };
}
