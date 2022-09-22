use std::io::Read;

use tokio::io::{BufReader, AsyncRead};

use crate::{
    error::{Error, Result},
    protocol::{Protocol, RawPiece},
};

#[derive(Debug)]
pub enum Command {
    Get { key: Vec<u8> },
}

impl Command {
    pub async fn from_resp2<R>(r: &mut BufReader<R>) -> Result<Self> 
    where R: AsyncRead + Unpin + Send {
        let pieces: Vec<RawPiece> =
            match RawPiece::parse(r).await? {
                RawPiece::SimpleString { data } | RawPiece::BulkString { data } => {
                    Self::split_vector_by_space(&data)
                        .iter()
                        .map(|each| RawPiece::SimpleString { data: each.clone() })
                        .collect()
                }
                RawPiece::Array(arr) => arr,
                _ => return Err(Error::BrokenProtocol(
                    "Request must be one of three forms: simple string, bulk string, and array."
                        .into(),
                )),
            };
        if pieces.len() == 0 {
            return Err(Error::BrokenProtocol(
                "empty lines given for command".into(),
            ));
        }
        let mut iter = pieces.into_iter();
        let cmd = match iter.next().unwrap() {
            RawPiece::SimpleString { data } | RawPiece::BulkString { data } => {
                Self::lower_bytes(&data);
                data
            }
            _ => return Err(Error::BrokenProtocol("command must be a string".into())),
        };
        match String::from_utf8(cmd)?.as_str() {
            "get" => {
                if let Some(key) = iter.next() {
                    if let Some(key) = key.read_string() {
                        Ok(Self::Get { key: key.clone() })
                    } else {
                        Err(Error::BrokenProtocol("missing key for get".into()))
                    }
                } else {
                    Err(Error::BrokenProtocol("missing key for get".into()))
                }
            }
            _ => Err(Error::Unsupported("unknown command".into())),
        }
    }

    fn lower_bytes(src: &Vec<u8>) -> Vec<u8> {
        let mut dst = src.to_vec();
        for c in dst.iter_mut() {
            if c.is_ascii_alphabetic() {
                c.make_ascii_lowercase();
            } 
        }
        dst
    }

    fn split_vector_by_space(src: &Vec<u8>) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let mut current_start = 0;
        for (offset, c) in src.iter().enumerate() {
            if c.is_ascii_whitespace() {
                if offset > current_start {
                    result.push(src[current_start..offset].to_vec());
                }
                current_start = offset + 1;
            }
        }
        result
    }
}
