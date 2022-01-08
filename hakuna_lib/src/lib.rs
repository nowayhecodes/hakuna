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

    if let Ok(mut file_contents) = file_contents {
        // bind file extension to MIME type
        let content_type = extension_to_mime_impl(extension);

        #[allow(unused_mut)]
        let mut content_length = file_contents.len();

        // Prepare to inject code into HTML if reload is enabled
        #[cfg(feature = "reload")]
        let reload_append = include_bytes!("./reload.html");

        #[cfg(feature = "reload")]
        {
            if extension == Some("html") && reload {
                content_length += reload_append.len();
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-type: {}\r\nContent-Length: {}{}\r\n\r\n",
            content_type, content_length, headers
        );

        let mut bytes = response.as_bytes().to_vec();
        bytes.append(&mut file_contents);
        stream.write_all(&bytes).unwrap();

        #[cfg(feature = "reload")]
        {
            if extension == Some("html") && reload {
                stream.write_all(reload_append).unwrap();
            }
        }

        stream.flush().unwrap();
    } else {
        println!("Could not find file: {}", path.to_str().unwrap());
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

pub fn run(address: &str, port: u32, path: &str, reload: bool, headers: &str) {
    #[cfg(feature = "https")]
    let acceptor = {
        let bytes = include_bytes!("./identity.pfx");
        let identity = Identity::from_pkcs12(bytes, "debug").unwrap();

        Arc::new(TlsAcceptor::new(identity).unwrap())
    };

    #[cfg(feature = "reload")]
    {
        if reload {
            let address = address.to_owned();
            let path = path.to_owned();

            thread::spawn(move || {
                reload::watch_for_reloads(&address, &path);
            });
        }
    }

    let address_with_port = format!("{}:{:?}", address, port);
    let listener = TcpListener::bind(address_with_port).unwrap();

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            #[cfg(feature = "https")]
            let acceptor = acceptor.clone();
            let path = path.to_owned();
            let headers = headers.to_owned();

            thread::spawn(move || {
                let mut buf = [0; 2];
                stream.peek(&mut buf).expect("peek failed");

                #[cfg(feature = "https")]
                let is_https =
                    !((buf[0] as char).is_alphabetic() && (buf[1] as char).is_alphabetic());

                #[cfg(not(feature = "https"))]
                let is_https = false;

                if is_https {
                    #[cfg(feature = "https")]
                    if let Ok(stream) = acceptor.accept(stream) {
                        handle_client(stream, &path, reload, &headers);
                    }
                } else {
                    handle_client(stream, &path, reload, &headers);
                }
            });
        }
    }
}

fn extension_to_mime_impl(ext: Option<&str>) -> &'static str {
    match ext {
        _ => "application/octet-stream"
    }
}