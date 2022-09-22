use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};

use crate::error::Result;

mod base;
pub(self) mod constants;
pub mod resp2;
pub mod resp3;
pub use base::*;

#[async_trait]
pub trait Protocol: Sized {
    fn prefix(&self) -> u8;
    // async fn marshal<W: AsyncWrite>(&self, w: &mut BufWriter<W>) -> Result<usize>;
    async fn parse<R>(r: &mut BufReader<R>) -> Result<Self>
    where
        R: AsyncRead + Unpin + Send;
}
