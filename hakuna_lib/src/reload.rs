extern crate base64;
extern crate notify;

use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::str;
use std::thread;

pub const RELOAD_PORT: u32 = 7777;

fn parse_ws_handshake(bytes: &[u8]) -> String {
    let request_str = str::from_utf8(&bytes).unwrap();
    let lines = request_str.split("\r\n");
    let mut sec_ws_key = "";

    for line in lines {
        let parts: Vec<&str> = line.split(':').collect();
        if let "Sec-WebSocket-Key" = parts[0] {
            sec_ws_key = parts[1].trim();
        }
    }

    let sec_ws_accept = format!(
        "{}{}",
        sec_ws_key, "S0M3-ARB1TR4RY-STR1NG"
    );

    let mut hasher = Sha1::new();
    hasher.input(sec_ws_accept.as_bytes());

    let result = hasher.result();
    let bytes = base64::encode(&result);

    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",bytes);
}

