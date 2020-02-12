mod bulb;

#[macro_use] extern crate prettytable;
use prettytable::Table;

use std::str;
use std::env;
use std::process::exit;
use std::{thread, time};
use std::net::{TcpStream, UdpSocket};
use std::sync::mpsc::{Sender, Receiver, channel};
use bulb::Bulb;
use std::io::{self, Write, BufRead, Read};

const MULTICAST_ADDR: &str = "239.255.255.250:1982";

fn main() {
    // Search for bulbs on a separate thread
    let socket = create_socket();
    send_search_broadcast(&socket);
    let receiver = find_bulbs(socket);
    
    let bulbs: Vec<Bulb> = remove_duplicates(receiver.try_iter().collect());

    if bulbs.is_empty() {
        println!("No bulbs found.");
        exit(1);
    }

    // Prase arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("At least two arguments are required");
        exit(1);
    }

    let mut arg_start: usize = 2;
    
    let bulb_id: u32 = match args[1].parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            arg_start = 3;
            1
        }
    };

    let bulb = &bulbs[bulb_id as usize];
    let cmd = &args[arg_start];
    let state = &args[arg_start + 1];

    process_cmd(bulb_id, &bulb, cmd, state);
}

fn process_cmd(bulb_id: u32, bulb: &Bulb, cmd: &String, state: &String) {
    match &cmd[..] {
        "pow" => {
            operate_on_bulb(&bulb_id, bulb, "set_power", &format!("\"{}\"", state)) 
        }
        _ => return
    }
}

fn find_bulbs(socket: UdpSocket) -> Receiver<Bulb> {
    let (sender, receiver): (Sender<Bulb>, Receiver<Bulb>) = channel();
    thread::spawn(move || {
        let mut buf = [0; 2048];
        loop {
            match socket.recv_from(&mut buf) {
                Ok(_) => {
                    let _ = sender.send(Bulb::new(str::from_utf8(&buf).unwrap()));
                },
                Err(e) => {
                    println!("Couldn't receive a datagram: {}", e);
                    break;
                }
            }
            thread::sleep(time::Duration::from_millis(200));
        }
    });
    // Give the other thread some time to find the bulbs
    thread::sleep(time::Duration::from_millis(1200));
    receiver
}

fn send_search_broadcast(socket: &UdpSocket) {
    let message = b"M-SEARCH * HTTP/1.1\r\n
                    HOST: 239.255.255.250:1982\r\n
                    MAN: \"ssdp:discover\"\r\n
                    ST: wifi_bulb";

    socket.send_to(message, MULTICAST_ADDR).expect("Couldn't send to socket");
}

fn create_socket() -> UdpSocket {
    match UdpSocket::bind("0.0.0.0:34254") {
        Ok(s) => s,
        Err(e) => panic!("couldn't bind socket: {}", e)
    }
}

fn remove_duplicates(bulbs: Vec<Bulb>) -> Vec<Bulb> {
    let mut new = Vec::new();
    let mut ids = Vec::new();
    for bulb in bulbs {
        if ids.contains(&bulb.id) { continue }
        ids.push(bulb.id.clone());
        new.push(bulb);
    }
    new
}

fn create_message(id: &u32, method: &str, params: &str) -> String {
    let strs = [
        "{\"id\":",
        &id.to_string()[..],
        ",\"method\":\"",
        method,
        "\",\"params\":[",
        params,
        "]}\r\n"
    ];
    strs.join("")
}

fn operate_on_bulb(id: &u32, bulb: &Bulb, method: &str, params: &str) {
    // Send message to the bulb
    let message = create_message(id, method, params);

    let ip = &bulb.ip.to_owned()[..];
    let mut stream = TcpStream::connect(ip).expect("Couldn't start the stream.");
    match stream.write(message.as_bytes()) {
        Ok(_) => {
            print!("The message sent to the bulb is: {}", message);
            io::stdout().flush().unwrap();
        },
        Err(_) => {
            println!("Couldn't send to the stream");
            return;
        }
    }
    let mut buf = [0; 2048];
    match stream.read(&mut buf) {
        Ok(_) => {
            print!("The bulb returns: {}", str::from_utf8(&buf).unwrap());
            io::stdout().flush().unwrap();
        },
        Err(_) => {
            println!("Couldn't read from the stream.");
        }
    }
}
