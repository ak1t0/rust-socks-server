use std::io::prelude::*;
use std::io::{copy, Error};
use std::str::FromStr;
use std::net::{TcpStream, TcpListener, Shutdown, IpAddr, Ipv4Addr, ToSocketAddrs};
use std::thread;
use std::process::{exit};

fn main() {

    let socks_address = "127.0.0.1:10334";
    let listener;

    match TcpListener::bind(socks_address) {
        Err(e) => {
            println!("Error: {}", e);
            exit(0);
        },
        Ok(l) => {
            listener = l;
        },
    };

    loop {
        match listener.accept() {
            Err(e) => {
                println!("Error: {}", e)
            },
            Ok((stream, addr)) => {
                println!("Received request from {}", addr);
                // handle request in spawned thread
                thread::spawn(move || {
                    println!("This is in spawned thread");
                    handle_request(stream);                
                });
            }
        }
    }
    
    println!("Finish program");
}

fn handle_request(s: TcpStream) {
    let mut client_stream = s.try_clone().unwrap();

    let (r, ver, cmd) = check_request(&client_stream);
    if !r {
        println!("Error: SOCKS request is not valid");
        return;
    }

    let mut target_address;
    if ver == 4 {
        match process_request_v4(&client_stream) {
            Ok(target) => {
                target_address = target;
            },
            Err(e) => {
                println!("Error: failed to process SOCKS requset");
                return
            },
        }
    } else {
        println!("Error: SOCKS request is not valid");
        return;
    }
  
    // connect to target server
    let mut target_stream;
    match TcpStream::connect(&*target_address)  {
        Err(e) => {
            println!("Error: {}", e);
            exit(0);
        },
        Ok(s) => {
            target_stream = s;
            println!("remote connection is successful");
        },
    };

    // forward stream
    let mut client_stream_c = client_stream.try_clone().unwrap();
    let mut target_stream_c = target_stream.try_clone().unwrap();
    
    thread::spawn(move || {
        copy(&mut target_stream_c, &mut client_stream_c);
        target_stream_c.shutdown(Shutdown::Read);
        client_stream_c.shutdown(Shutdown::Write);
    });

    copy(&mut client_stream, &mut target_stream);
    target_stream.shutdown(Shutdown::Write);
    client_stream.shutdown(Shutdown::Read);

    println!("forwarding is completed");
}

fn process_request_v4(s: &TcpStream) -> Result<String, Error> {
    let mut stream = s.try_clone().unwrap();    

    let dstp = read_u16(&mut stream);
    let (dstp1, dstp2) = u16tou8(dstp);
    println!("dstport(u16): {}", dstp);

    let mut ip1 = read_u8(&mut stream);
    let mut ip2 = read_u8(&mut stream);
    let mut ip3 = read_u8(&mut stream);
    let mut ip4 = read_u8(&mut stream);
    println!("IP: {}.{}.{}.{}", ip1, ip2, ip3, ip4);

  
    read_user(&stream);
    let reply = [0, 90, dstp1, dstp2, ip1, ip2, ip3, ip4];
    stream.write(&reply);

    println!("SOCKS check fin!");

    return Ok(format!("{}.{}.{}.{}:{}", ip1, ip2, ip3, ip4, dstp));
}

fn read_user(s: &TcpStream) -> String {
    let mut stream = s.try_clone().unwrap();
    let mut buf: [u8; 128] = [0; 128];
    let mut user = "".to_string();

    stream.read(&mut buf);

    for i in 1..128 {
        let ch = buf[i] as char;
        user.push(ch);
    };
    println!("user: {}", user);

    return user;
}


fn read_u8(s: &mut TcpStream) -> u8 {
    let mut buf: [u8; 1] = [0];

    s.read(&mut buf);
    return buf[0];
}

fn read_u16(s: &mut TcpStream) -> u16 {
    let mut buf: [u8; 2] = [0, 0];

    s.read(&mut buf);

    // two u8 to u16
    let data1 = buf[0] as u16;
    let data2 = buf[1] as u16;
    let data = data1 * 256 + data2;
    return data;
}

fn check_request(s: &TcpStream) -> (bool, u8, u8) {
    let mut stream = s.try_clone().unwrap();
    let mut buf: [u8; 128] = [0; 128];
    let ver = read_u8(&mut stream);
    let cmd = read_u8(&mut stream);
    println!("Version: {}", ver);
    println!("Command: {}", cmd);

    if ((ver == 4) | (ver == 5)) & ((cmd == 1) | (cmd == 2)) {
        (true, ver, cmd)
    } else {
        (false, 0, 0)
    }
}

fn u16tou8(t: u16) -> (u8, u8) {
    if t < 256 {
        return (0, t as u8)
    } else {
        return ((t / 256) as u8, (t % 256) as u8)
    }
}
