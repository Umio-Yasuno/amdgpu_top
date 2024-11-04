use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Read, Write};

use amdgpu_top_json::JsonApp;

pub fn gui_server(addr: &SocketAddr, title: &str, j: &mut JsonApp) {
    println!("server");
    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                let mut ping = [0u8; 4];
                stream.read_exact(&mut ping).unwrap();
                if &ping == b"ping" {
                    println!("rec ping");
                } else {
                    panic!();
                }
                j.run(title, None, &mut Some(stream));
            },
            Err(_) => {},
        }
    }
}

pub fn gui_client(addr: &SocketAddr) {
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| {
        panic!("Failed to connect: {e}");
    });
    println!("connect");
    stream.write(b"ping").unwrap();
    let mut buf = String::new();
    let mut buf_array = [0u8; 16];
    let mut stream = BufReader::new(stream);

    for _ in 0..10 {
        println!("DDD");
        // buf.clear();
        // stream.write(&[0u8, 1u8]).unwrap();
        // println!("await");
        stream.read_line(&mut buf).unwrap();
        // stream.read_exact(&mut buf_array).unwrap();
        println!(" rr: {buf:?}");
    }
}
