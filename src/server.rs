use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};

use amdgpu_top_json::JsonApp;

pub fn gui_server(addr: &SocketAddr, title: &str, j: &mut JsonApp) {
    println!("server");
    let listener = TcpListener::bind(addr).unwrap();
    let j = Arc::new(Mutex::new(j));

    let update_j = j.clone();

    std::thread::spawn(move || loop {
        let lock = update_j.lock();

        if let Ok(mut j) = lock {
            j.update();
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                let mut ping = [0u8; 4];
                stream.read_exact(&mut ping).unwrap();
                if &ping == b"ping" {
                    println!("rec ping");
                } else {
                    continue;
                }

                loop {
                    j.update();

                    let s = j.json(title).to_string();

                    if let Ok(_) = stream.write(s.as_bytes()) {
                        let _ = stream.write(b"\n").unwrap();
                    }
                }
            },
            Err(_) => {},
        }
    }
}

pub fn gui_client(addr: &SocketAddr) {
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| {
        panic!("Failed to connect: {e}");
    });
    stream.write(b"ping").unwrap();
    let mut buf = String::new();
    let mut stream = BufReader::new(stream);

    for _ in 0..10 {
        buf.clear();
        stream.read_line(&mut buf).unwrap();
        println!("{buf}\n");
    }
}
