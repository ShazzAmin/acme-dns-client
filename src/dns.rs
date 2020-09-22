// RFC 1035 (https://tools.ietf.org/html/rfc1035)
extern crate packed_struct;

use std::net::UdpSocket;
use std::str;
use std::time::Duration;
use packed_struct::prelude::*;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use std::fmt;
use std::io;

const MESSAGE_MAX_BYTES: usize = 512;
const READ_TIMEOUT: Duration = Duration::from_secs(1);
const QUESTION_TYPE_TXT: u16 = 16;
const QUESTION_CLASS_IN: u16 = 1;

#[derive(Debug)]
pub enum Error {
    PackingError(PackingError),
    Utf8Error(str::Utf8Error),
    IoError(io::Error),
    RequestError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::PackingError(e) => write!(f, "{}", e),
            Error::Utf8Error(e) => write!(f, "{}", e),
            Error::IoError(e) => write!(f, "{}", e),
            Error::RequestError => write!(f, "request error"),
        }
    }
}

impl std::error::Error for Error {}

impl From<PackingError> for Error {
    fn from(e: PackingError) -> Self {
        Error::PackingError(e)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8Error(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(endian="msb")]
pub struct Header {
    id: u16,
    qr: bool,
    opcode: Integer<u8, packed_bits::Bits4>,
    aa: bool,
    tc: bool,
    rd: bool,
    ra: bool,
    z: Integer<u8, packed_bits::Bits3>,
    rcode: Integer<u8, packed_bits::Bits4>,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

#[derive(PackedStruct, Debug)]
#[packed_struct(endian="msb")]
pub struct QuestionInfo {
    kind: u16,
    class: u16,
}

#[derive(PackedStruct, Debug)]
#[packed_struct(endian="msb")]
pub struct ResourceInfo {
    kind: u16,
    class: u16,
    ttl: u32,
    rdlength: u16,
}

fn extract_labels(mut buf: &mut [u8]) -> Result<(Vec<&str>, &mut [u8]), Error> {
    let mut labels: Vec<&str> = vec![];

    let mut labels_end = false;
    while !labels_end {
        buf = match buf.split_first_mut() {
            Some((&mut length, buf_rest)) => {
                if length == 0 {
                    labels_end = true;
                    buf_rest
                } else {
                    let (buf_label, buf_rest) = buf_rest.split_at_mut(length as usize);
                    labels.push(str::from_utf8_mut(buf_label)?);

                    buf_rest
                }
            },

            None => return Err(Error::RequestError),
        };
    }

    Ok((labels, buf))
}

fn put_labels<'a>(mut buf: &'a mut [u8], labels: Vec<&str>) -> &'a mut [u8] {
    for &label in labels.iter() {
        buf = {
            let (buf_label, buf_rest) = buf.split_at_mut(1 + label.len());
            buf_label[0] = label.len() as u8;
            buf_label[1..=label.len()].copy_from_slice(label.as_bytes());

            buf_rest
        }
    }
    
    buf[0] = 0;

    &mut buf[1..]
}

fn put_character_string(buf: &mut [u8], string: &str) {
    buf[0] = string.len() as u8;
    buf[1..=string.len()].copy_from_slice(string.as_bytes());
}

fn process<'a>(buf: &'a mut [u8], proof: &str) -> Result<&'a [u8], Error> {
    // get header
    let (buf_header, buf_rest) = buf.split_at_mut(Header::packed_bytes());
    let req_header = Header::unpack_from_slice(buf_header)?;

    // extract labels & question
    let (labels, buf_rest) = extract_labels(buf_rest)?;
    let (buf_question_info, buf_rest) = buf_rest.split_at_mut(QuestionInfo::packed_bytes());
    let question_info = QuestionInfo::unpack_from_slice(buf_question_info)?;
    if question_info.kind != QUESTION_TYPE_TXT || question_info.class != QUESTION_CLASS_IN {
        return Err(Error::RequestError)
    }

    // build response
    Header {
        id: req_header.id,
        qr: true,
        opcode: 0u8.into(),
        aa: true,
        tc: false,
        rd: req_header.rd,
        ra: false,
        z: 0u8.into(),
        rcode: 0u8.into(),
        qdcount: 1,
        ancount: 1,
        nscount: 0,
        arcount: 0,
    }.pack_to_slice(buf_header)?;

    let buf_rest = put_labels(buf_rest, labels);
    let (buf_resource_info, buf_rest) = buf_rest.split_at_mut(ResourceInfo::packed_bytes());
    ResourceInfo {
        kind: question_info.kind,
        class: question_info.class,
        ttl: 0,
        rdlength: 1 + proof.len() as u16,
    }.pack_to_slice(buf_resource_info)?;
    put_character_string(buf_rest, &proof);

    Ok(buf)
}

pub fn start_responding_with(proof: String) -> SyncSender<()> {
    let (kill_tx, kill_rx) = sync_channel(0);

    thread::spawn(move || {
        let socket = UdpSocket::bind("0.0.0.0:53").expect("DNS server failed to bind to 0.0.0.0:53.");
        socket.set_read_timeout(Some(READ_TIMEOUT)).expect("Failed to set socket read timeout.");

        loop {
            let mut buf: [u8; MESSAGE_MAX_BYTES] = [0; MESSAGE_MAX_BYTES];
            if let Ok((_, source)) = socket.recv_from(&mut buf) {
                match process(&mut buf, &proof) {
                    Ok(buf) => {
                        if let Err(e) = socket.send_to(&buf, &source) {
                            println!("Failed to respond to DNS request.");
                            eprintln!("{}", e);
                        }
                    },
                    Err(e) => {
                        println!("Ignoring invalid DNS request.");
                        eprintln!("{}", e);
                    },
                }
            }

            if let Ok(_) = kill_rx.try_recv() {
                break;
            }
        }
    });

    kill_tx
}
