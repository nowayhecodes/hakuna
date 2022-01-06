#[cfg(feature = "https")]
use native_tls::{Identity, TlsAcceptor};

#[cfg(feature = "https")]
use std::sync::Arc;

use std::ffi::OsStr;
use std::fs;
use std::io::BufRead;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::str;
use std::thread;

#[cfg(feature = "reload")]
mod reload;

pub fn read_header<T: Read + Write>(stream: &mut T) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut reader = std::io::BufReader::new(stream);
    loop {
        reader.read_until(b'\n', &mut buffer).unwrap();
        if &buffer[buffer.len() - 4..] == b"\r\n\r\n" {
            break;
        }
    }
    buffer
}

#[allow(unused)]
fn handle_client<T: Read + Write>(mut stream: T, root_path: &str, reload: bool, headers: &str) {
    let buffer = read_header(&mut stream);
    let request_string = str::from_utf8(&buffer).unwrap();
    if request_string.is_empty() {
        return;
    }

    // split the request into different parts
    let mut parts = request_string.split(' ');
    let _method = parts.next().unwrap().trim();
    let mut path = parts.next().unwrap().trim();
    let _http_version = parts.next().unwrap().trim();

    // trim parameters from URL
    if let Some(parameters_index) = path.find('?') {
        path = &path[..parameters_index];
    }

    let path = path.replace("../", "").replace("%20", " ");
    let path = if path.ends_with("/") {
        Path::new(root_path).join(Path::new(&format!(
            "{}{}",
            path.trim_start_matches('/'),
            "index.html"
        )))
    } else {
        Path::new(root_path).join(path.trim_matches('/'))
    };

    let extension = path.extension().and_then((OsStr::to_str));

    let (file_contents, extensions) = if extension != None {
        (fs::read(&path), extension)
    } else {
        if let Ok(file_contents) = fs::read(&path) {
            println!("WARN: Serving file [ {} ] without extension with media type 'application/octet-stream'", &path.to_str().unwrap());
            (Ok(file_contents), None)
        } else {
            let file = fs::read(&path.with_extension("html"));
            (file, Some("html"))
        }
    };

    if let Ok(mut file_contents) = file_contents {}
}
