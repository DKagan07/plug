use inquire;
use netstat2::{AddressFamilyFlags, ProtocolFlags};

fn main() {
    let address_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocol_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let socket_info = match netstat2::get_sockets_info(address_flags, protocol_flags) {
        Ok(socket_info) => socket_info,
        Err(err) => panic!("error getting socket info: {err:?}"),
    };

    println!("socket info ex: {:?}", socket_info[0]);
}
