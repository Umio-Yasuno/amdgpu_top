use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};

use amdgpu_top_json::JsonApp;

pub fn gui_server(addr: &SocketAddr, mut j: JsonApp) {
    println!("server");
    let listener = TcpListener::bind(addr).unwrap();
    let share_s = Arc::new(Mutex::new(j.json().to_string()));

    {
        let share_s = share_s.clone();

        std::thread::spawn(move || loop {
            j.update();

            let lock = share_s.lock();

            if let Ok(mut s) = lock {
                *s = j.json().to_string();
            }
        });
    }

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                /* {
                    let mut ping = [0u8; 4];
                    stream.read_exact(&mut ping).unwrap();
                    if &ping == b"ping" {
                        println!("rec ping");
                    } else {
                        continue;
                    }
                } */

                let share_s = share_s.clone();
                let mut cmd = [0u8; 1];

                std::thread::spawn(move || loop {
                    'cmd: loop {
                        if stream.read_exact(&mut cmd).is_ok() && &cmd == b"d" {
                            cmd = [0u8; 1];
                            break 'cmd;
                        }
                    }

                    let lock = share_s.try_lock();
                    if let Ok(s) = lock {
                        if let Err(_) = stream.write(format!("{s}\n").as_bytes()) {
                            return;
                        }
                    }
                });
            },
            Err(_) => {},
        }
    }
}

pub fn gui_client(addr: &SocketAddr) {
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| {
        panic!("Failed to connect: {e}");
    });
    let mut buf = String::new();
    let mut stream_read = BufReader::new(stream.try_clone().unwrap());

    for _ in 0..1 {
        buf.clear();
        let Ok(_) = stream.write(b"d") else { continue };
        let Ok(_) = stream_read.read_line(&mut buf) else { continue };
        println!("{buf}\n");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
