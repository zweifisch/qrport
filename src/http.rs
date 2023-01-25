use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{TcpStream, TcpListener, SocketAddr};
use std::io::{Read, Write};
use std::fs::File;
use std::thread;

#[derive(Debug)]
pub enum HttpError {
    NoBody,
    NoProtocol,
    InvalidProtocol,
    InvalidHeader,
    IOError(std::io::Error),
}

impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> HttpError {
        HttpError::IOError(err)
    }
}

#[derive(Debug)]
pub struct Request {
    stream: RefCell<TcpStream>,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl TryFrom<TcpStream> for Request {
    type Error = HttpError;
    fn try_from(value: TcpStream) -> Result<Self, Self::Error> {
        parse(value)
    }
}

pub type HttpHandler = fn(req: Request);

impl Request {
    pub fn send_bytes(&self, bytes: &Vec<u8>) -> Result<(), HttpError> {
        let response = b"HTTP/1.1 200 OK\r
Content-Type: application/octet-stream\r
\r\n";
        self.stream.borrow_mut().write(response)?;
        self.stream.borrow_mut().write(&bytes)?;
        Ok(())
    }

    pub fn send_file(&self, path: &str) -> Result<(), HttpError> {

        let abs_path = if path.starts_with("~") {
            let home = std::env::var("HOME").unwrap();
            format!("{}{}", home, path.strip_prefix("~").unwrap())
        } else {
            path.to_string()
        };

        let mut file = File::open(&abs_path).unwrap();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        self.send_bytes(&content)
    }

    pub fn not_found(&self, msg: &str) -> Result<(), HttpError> {
        let response = b"HTTP/1.1 404 Not Found\r
\r";
        self.stream.borrow_mut().write(response)?;
        self.stream.borrow_mut().write(msg.as_bytes())?;
        Ok(())
    }

    pub fn addr(&self) -> Result<SocketAddr, HttpError> {
        Ok(self.stream.borrow_mut().peer_addr()?)
    }
}

fn parse(mut stream: TcpStream) -> Result<Request, HttpError> {
    let mut buf = [0u8; 4096];
    let len = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[0..len]);

    let (raw_headers, body) = req.split_once("\r\n\r\n").ok_or(HttpError::NoBody)?;

    let mut splitted = raw_headers.split("\r\n");

    let mut protocol = splitted.next()
        .ok_or(HttpError::NoProtocol)?
        .split(' ');

    let method = protocol.next().ok_or(HttpError::InvalidProtocol)?;
    let path = protocol.next().ok_or(HttpError::InvalidProtocol)?;
    let _version = protocol.next().ok_or(HttpError::InvalidProtocol)?;

    let mut headers = HashMap::new();
    for header in splitted {
        let (key, value) = header.split_once(":").ok_or(HttpError::InvalidHeader)?;
        headers.insert(key.to_string(), value.to_string());
    }

    Ok(Request {
        stream: RefCell::new(stream),
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body: body.to_string(),
    })
}

pub fn serve<T>(port: u16, handler: T) where T: Fn(&Request) + Send + Clone + 'static {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();

    for stream in listener.incoming() {
        let handler = handler.clone();
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let request = Request::try_from(stream).unwrap();
                    handler(&request)
                });
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}
