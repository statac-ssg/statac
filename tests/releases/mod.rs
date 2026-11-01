//! A server on this machine standing in for get.statac.dev, and a key of
//! the tests' own standing in for Statac's, shared by the install scripts'
//! tests and `statac upgrade`'s, which include it by its path.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

use rustls::SignatureScheme;
use rustls::crypto::aws_lc_rs;
use rustls::pki_types::PrivatePkcs8KeyDer;

/// Statac's key, as the scripts and `src/upgrade.rs` carry it, and the
/// tests' own, its private half in PKCS #8.
pub(crate) const KEY: &str = "9ba90e0b6ec11a149baad9a54d6cf33d15a872e75c08d915f7361225d32febde";
pub(crate) const TEST_KEY: &str =
    "530c1dad1c6081a3172af0e023067098707b8619b7725705cbd5bd3e7299cc9b";
const TEST_PRIVATE: &str = "302e020100300506032b657004220420c8e9544d25e30fee7798a7808d7fc8495fdec2be7890ab945763f9096ee66dcd";

pub(crate) fn bytes_of_hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|at| u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).unwrap())
        .collect()
}

pub(crate) fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The tests' key's signature of `message`.
pub(crate) fn signature(message: &[u8]) -> Vec<u8> {
    let der = bytes_of_hex(TEST_PRIVATE);
    let key = aws_lc_rs::sign::any_eddsa_type(&PrivatePkcs8KeyDer::from(&der[..])).unwrap();
    let signer = key.choose_scheme(&[SignatureScheme::ED25519]).unwrap();
    signer.sign(message).unwrap()
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    let suite = aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .unwrap();
    hex_of(suite.common.hash_provider.hash(bytes).as_ref())
}

/// What the server answers a path with: its status line and body.
type Answers = Arc<Mutex<HashMap<String, (&'static str, Vec<u8>)>>>;

/// The server, answering each path it holds with its bytes, each path set
/// to fail with `500`, and any other with `404`, and remembering each path
/// asked for.
pub(crate) struct Releases {
    pub(crate) addr: SocketAddr,
    answers: Answers,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Releases {
    pub(crate) fn start() -> Releases {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let answers: Answers = Arc::default();
        let asked: Arc<Mutex<Vec<String>>> = Arc::default();
        let (held, heard) = (answers.clone(), asked.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|end| end == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => request.extend(&buffer[..read]),
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let path = request.split(' ').nth(1).unwrap_or_default().to_owned();
                heard.lock().unwrap().push(path.clone());
                let answer = held.lock().unwrap().get(&path).cloned();
                let (status, body) = answer.unwrap_or(("404 Not Found", b"nothing here".to_vec()));
                // As get.statac.dev serves the scripts.
                let kind = match path.ends_with(".zip") {
                    true => "application/zip",
                    false => "text/plain; charset=utf-8",
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Releases {
            addr,
            answers,
            asked,
        }
    }

    pub(crate) fn address(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(crate) fn put(&self, path: &str, bytes: impl Into<Vec<u8>>) {
        let mut answers = self.answers.lock().unwrap();
        answers.insert(path.to_owned(), ("200 OK", bytes.into()));
    }

    /// What the server answers `path` with, where it holds it.
    pub(crate) fn get(&self, path: &str) -> Vec<u8> {
        self.answers.lock().unwrap()[path].1.clone()
    }

    pub(crate) fn remove(&self, path: &str) {
        self.answers.lock().unwrap().remove(path);
    }
}

// Not every test that includes this uses these.
#[allow(dead_code)]
impl Releases {
    pub(crate) fn fail(&self, path: &str) {
        let mut answers = self.answers.lock().unwrap();
        let body = b"something went wrong".to_vec();
        answers.insert(path.to_owned(), ("500 Internal Server Error", body));
    }

    pub(crate) fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}
