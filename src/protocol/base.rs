use std::io::Write;

use async_trait::async_trait;
use log::debug;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};

use crate::error::{Error, Result};

use super::{
    Protocol,
};

const PREFIX_SIMPLE_STRING: u8 = '+' as u8;
const PREFIX_ERROR: u8 = '-' as u8;
const PREFIX_INTEGER: u8 = ':' as u8;
const PREFIX_BULK_STRING: u8 = '$' as u8;
const PREFIX_ARRAY: u8 = '*' as u8;

#[derive(Debug)]
struct LeadingUnit {
    /// for type
    prefix_char: u8,
    data: Vec<u8>,
}

impl LeadingUnit {
    fn read_int(&self) -> Option<i64> {
        parse_int(&self.data)
    }
}

fn parse_int(src: &Vec<u8>) -> Option<i64> {
    match String::from_utf8(src.clone()) {
        Ok(s) => match s.parse::<i64>() {
            Ok(i) => Some(i),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// Messages by RESP consist of lines, which is called unit by myself.
/// Unit is classified into two types: the leading unit and the payload unit.
/// Generally, the leading unit is used to given meta data, and indicates the length of
/// its following payload unit. And the payload unit carries the raw content following.
/// But there are some excepted cases in which a leading unit is enough to carry the whole
/// information, such as the simple string case.
/// A leading unit is a line in format as '<char><content>\r\n', and it consists of three parts:
/// 1. a `char` as prefix, indicates what type the unit is.
/// 2. content of the unit. it may be an integer or a string.
/// 3. \r\n, a fixed suffix ends the unit.
/// A payload unit, on the other hand, is much simpler than the leading one. It merely consists of
/// two parts: a string of content and a \r\n suffix. And the length of the string is given by
/// its previous leading unit.
#[derive(Debug)]
enum Unit {
    Leading(LeadingUnit),
    Payload(Vec<u8>),
}

/// A RawPiece represents a piece of request of RESP.
/// But in redis, every piece here is represented as robject.
pub enum RawPiece {
    SimpleString { data: Vec<u8> },
    Error { typ: Vec<u8>, cause: Vec<u8> },
    Integer(i64),
    BulkString { data: Vec<u8> },
    Array(Vec<RawPiece>),
    Null,
}

impl RawPiece {
    pub fn read_string(&self) -> Option<&Vec<u8>> {
        match self {
            RawPiece::SimpleString { data } => Some(&data),
            RawPiece::BulkString { data } => Some(&data),
            _ => None,
        }
    }

    fn space_pos(data: &[u8]) -> Option<usize> {
        for (pos, c) in data.iter().enumerate() {
            if *c as char == ' ' {
                return Some(pos);
            }
        }
        None
    }

    async fn read_unit<R: AsyncRead + Unpin>(r: &mut BufReader<R>) -> Result<Unit> {
        let mut buffer = bytes::BytesMut::new();
        loop {
            let mut cur = Vec::new();
            debug!("read line before: {:?}, bufferLen: {}", cur, buffer.len());
            match r.read_until('\n' as u8, &mut cur).await {
                Ok(read_size) => {
                    if read_size == 0 {
                        if buffer.len() == 0 {
                            return Err(Error::EOF);
                        } else {
                            break;
                        }
                    } else {
                        // let cur = cur.chars().filter(|&c| c!='\0').map(|c| c as u8).collect::<Vec<char>>();
                        let cur = cur.iter().map(|&c| c).filter(|&c| c != '\0' as u8).collect::<Vec<u8>>();
                        debug!("Got buf({:?}): {} {:?}", read_size, cur.len(), cur);
                        if cur.len() > 0 {
                            buffer.extend(&cur);
                            // buffer.extend_from_slice(&cur);
                            // buf.push_str(cur.trim_start());
                        }
                        
                        if cur.ends_with(&['\r' as u8, '\n' as u8]) {
                            break;
                        }
                    }
                }
                Err(e) => return Err(Error::IO(e)),
            }
        }
        let data = buffer.to_vec();
        // let data: Vec<char> = buffer.chars().collect();
        debug!("read line: `{:?}` buf.len={} data[0] = {:?} data={:?}", buffer, buffer.len(), data[0] as char, data);
        let unit = match data[0] as char {
            '+' | '-' | ':' | '$' | '*' => Unit::Leading(LeadingUnit {
                prefix_char: data[0],
                data: data[1..].to_vec(),
            }),
            _ => Unit::Payload(data.to_vec()),
        };
        debug!("Got Unit: {:?}", unit);
        Ok(unit)
        // Err(Error::EOF)
    }
}

#[async_trait]
impl Protocol for RawPiece {
    fn prefix(&self) -> u8 {
        match *self {
            RawPiece::SimpleString { data: _ } => PREFIX_SIMPLE_STRING,
            RawPiece::Error { typ: _, cause: _ } => PREFIX_ERROR,
            RawPiece::Integer(_) => PREFIX_INTEGER,
            RawPiece::BulkString { data: _ } => PREFIX_BULK_STRING,
            RawPiece::Array(_) => PREFIX_ARRAY,
            RawPiece::Null => PREFIX_BULK_STRING,
        }
    }

    // fn marshal<W: AsyncWrite>(&self, w: &mut BufWriter<W>) -> Result<usize> {
    //     let mut total_size = w.write(&vec![self.prefix(); 1])?;
    //     match *self {
    //         RawPiece::SimpleString { data } => {
    //             total_size += w.write(&data)?;
    //             total_size += w.write(&constants::CRLF)?;
    //         }
    //         RawPiece::Error { typ, cause } => {
    //             total_size += w.write(&typ)?;
    //             if cause.len() > 0 {
    //                 total_size += w.write(&vec![' ' as u8; 1])?;
    //                 total_size += w.write(&cause)?;
    //             }
    //         }
    //         RawPiece::Integer(i) => {
    //             total_size += w.write(i.to_string().as_bytes())?;
    //         }
    //         RawPiece::BulkString { data } => {
    //             total_size += w.write(data.len().to_string().as_bytes())?;
    //             total_size += w.write(&constants::CRLF)?;
    //             total_size += w.write(&data)?;
    //         }
    //         RawPiece::Array(arr) => {
    //             total_size += w.write(arr.len().to_string().as_bytes())?;
    //             let sum: Result<usize> = arr.iter().map(|each| each.marshal(w)).sum();
    //             total_size += sum?;
    //         }
    //         RawPiece::Null => {
    //             total_size += w.write((-1).to_string().as_bytes())?;
    //         }
    //     };
    //     total_size += w.write(&constants::CRLF)?;
    //     Ok(total_size)
    // }

    async fn parse<R>(r: &mut BufReader<R>) -> Result<Self>
    where
        R: AsyncRead + Unpin + Send,
    {
        // let mut wrap = BufReader::new(r);
        match Self::read_unit(r).await? {
            Unit::Leading(LeadingUnit { prefix_char, data }) => match prefix_char {
                PREFIX_SIMPLE_STRING => Ok(Self::SimpleString { data }),
                PREFIX_ERROR => {
                    let offset = Self::space_pos(&data);
                    let (typ, cause) = if let Some(offset) = offset {
                        (data[..offset].to_vec(), data[offset + 1..].to_vec())
                    } else {
                        (data, vec![])
                    };
                    Ok(Self::Error { typ, cause })
                }
                PREFIX_INTEGER => {
                    if let Some(i) = parse_int(&data) {
                        Ok(Self::Integer(i))
                    } else {
                        Err(Error::BrokenProtocol("invalid integer".into()))
                    }
                }
                PREFIX_BULK_STRING => {
                    let len = parse_int(&data);
                    if len.is_none() {
                        return Err(Error::BrokenProtocol(
                            "invalid length of bulk string".into(),
                        ));
                    }
                    let len = len.unwrap();
                    if len == 0 {
                        return Ok(Self::BulkString { data: Vec::new() });
                    } else if len == -1 {
                        return Ok(Self::Null);
                    } else if len < 0 {
                        return Err(Error::BrokenProtocol("invalid length given".into()));
                    }
                    let len = len as usize;
                    let payload_unit = Self::read_unit(r).await?;
                    match payload_unit {
                        Unit::Leading(_) => {
                            return Err(Error::BrokenProtocol("missing payload unit".into()))
                        }
                        Unit::Payload(data) => {
                            if data.len() != len {
                                return Err(Error::BrokenProtocol(
                                    "length of payload mismatches".into(),
                                ));
                            }
                            Ok(Self::BulkString { data })
                        }
                    }
                }
                PREFIX_ARRAY => {
                    let len = parse_int(&data);
                    if len.is_none() {
                        return Err(Error::BrokenProtocol(
                            "invalid length of bulk string".into(),
                        ));
                    }
                    let len = len.unwrap() as usize;
                    let mut arr = Vec::with_capacity(len);
                    if len == 0 {
                        return Ok(Self::Array(arr));
                    }
                    for _ in 0..len {
                        let piece = Self::parse::<R>(r).await?;
                        arr.push(piece);
                    }
                    Ok(Self::Array(arr))
                }
                _ => panic!("never here"),
            },
            Unit::Payload(unit) => Err(Error::BrokenProtocol("missing leading unit".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use crate::protocol::Protocol;

    use super::RawPiece;

    #[test]
    fn marshal() {
        // let input = RawPiece::Array(vec![]);
        // let inner = Vec::new();
        // let mut buffer = BufWriter::new(inner);
        // assert_eq!(input.marshal(&mut buffer).unwrap(), 4);
        // assert_eq!(inner, b"*0\r\n".to_vec());
    }
}
