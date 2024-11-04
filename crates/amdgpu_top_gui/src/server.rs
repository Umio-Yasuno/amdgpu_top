use std::net::{TcpListener, TcpStream};

pub fn gui_server(addr: &str) {
    println!("server");
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
            },
            Err(_) => {},
        }
    }
}

pub fn gui_client(addr: &str) {
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| {
        panic!("Failed to connect: {e}");
    });
    println!("connect");
}
