pub use accept_encoding::Encoding;
use crate::{
    middleware::{Middleware, Next, Request, Response, Body},
    Exception,
    http_client::HttpClient
};
use http::{
    header::CONTENT_ENCODING,
    header::{ACCEPT_ENCODING, HeaderValue}
};
use async_compression::stream::{GzipDecoder, ZstdDecoder, BrotliDecoder, DeflateDecoder};
use futures::{
    stream::StreamExt,
    future::BoxFuture
};


static SUPPORTED_ENCODINGS: &str = "gzip, br, deflate, zstd";


pub fn new() -> Compression {
    Compression::new()
}

/// Middleware for automatically handling incoming response compression.
///
/// This middleware currently supports HTTP compression using `gzip`, `deflate`, `br`, and `zstd`.
#[derive(Debug)]
pub struct Compression;

impl Compression {
    /// Creates the Compression middleware.
    pub fn new() -> Self {
        Self {}
    }

    fn parse_encoding(s: &str) -> Result<Encoding, ()> {
        match s {
            "gzip" => Ok(Encoding::Gzip),
            "deflate" => Ok(Encoding::Deflate),
            "br" => Ok(Encoding::Brotli),
            "zstd" => Ok(Encoding::Zstd),
            "identity" => Ok(Encoding::Identity),
            _ => Err(()),
        }
    }

     async fn decode(&self, req: &mut Response) {
        let encodings = if let Some(hval) = req.headers().get(CONTENT_ENCODING.as_str()) {
            let hval = match hval.to_str() {
                Ok(hval) => hval,
                Err(_) => {
                    return;
                },
            };
            hval.split(',')
                .map(str::trim)
                .rev() // apply decodings in reverse order
                .map(Compression::parse_encoding)
                .collect::<Result<Vec<Encoding>, ()>>().unwrap()//?

        } else {
            return;
        };

        for encoding in encodings {
            match encoding {
                Encoding::Gzip => {
                    let body = std::mem::replace(req.body_mut(), Body::empty());
                    let mut decoded_content:Vec<u8> = Vec::new();
                    let mut decoded_stream = GzipDecoder::new(body);
                    while let Some(bytes) = decoded_stream.next().await {
                        for byte in bytes.unwrap() {
                            decoded_content.push(byte);
                        }
                    }
                    *req.body_mut() = Body::from(decoded_content);
                }
                Encoding::Deflate => {
                    let body = std::mem::replace(req.body_mut(), Body::empty());
                    let mut decoded_content:Vec<u8> = Vec::new();
                    let mut decoded_stream = DeflateDecoder::new(body);
                    while let Some(bytes) = decoded_stream.next().await {
                        for byte in bytes.unwrap() {
                            decoded_content.push(byte);
                        }
                    }
                    *req.body_mut() = Body::from(decoded_content);
                }
                Encoding::Brotli => {
                    let body = std::mem::replace(req.body_mut(), Body::empty());
                    let mut decoded_content:Vec<u8> = Vec::new();
                    let mut decoded_stream = BrotliDecoder::new(body);
                    while let Some(bytes) = decoded_stream.next().await {
                        for byte in bytes.unwrap() {
                            decoded_content.push(byte);
                        }
                    }
                    *req.body_mut() = Body::from(decoded_content);
                }
                Encoding::Zstd => {
                    let body = std::mem::replace(req.body_mut(), Body::empty());
                    let mut decoded_content:Vec<u8> = Vec::new();
                    let mut decoded_stream = ZstdDecoder::new(body);
                    while let Some(bytes) = decoded_stream.next().await {
                        for byte in bytes.unwrap() {
                            decoded_content.push(byte);
                        }
                    }
                    *req.body_mut() = Body::from(decoded_content);
                }
                Encoding::Identity => (),
            }
        }
        // strip the content-encoding header
        req.headers_mut().remove(CONTENT_ENCODING).unwrap();
    }

}


impl<C: HttpClient> Middleware<C> for Compression {
    #[allow(missing_doc_code_examples)]
    fn handle<'a>(
        &'a self,
        mut req: Request,
        client: C,
        next: Next<'a, C>,
    ) -> BoxFuture<'a, Result<Response, crate::Exception>> {
        Box::pin(async move {
            req.headers_mut()
                .insert(ACCEPT_ENCODING, HeaderValue::from_static(SUPPORTED_ENCODINGS));
            let mut res = next.run(req, client).await?;
            self.decode(&mut res).await;
            Ok(res)
        })
    }
}