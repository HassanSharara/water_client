use std::collections::HashMap;
use std::fmt::Display;
use std::ops::DerefMut;
use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::models::request:: HttpBody;

/// Represents an HTTP response
#[derive(Debug)]
pub struct HttpResponse{
    http_version: String,
    status_code: u16,
    status_label: String,
    headers: HashMap<String, String>,
    pub(crate)content_length: Option<usize>,
    pub(crate) body: Option<HttpBody>,
    pub(crate) dropped:Arc<Mutex<bool>>,
    pub(crate) size_of_head:usize,
}

impl Drop for HttpResponse {
    fn drop(&mut self) {
        let dropped = self.dropped.clone();
        tokio::spawn(async move {
            let mut v = dropped.lock().await;
            *(v.deref_mut()) = true;
        });
    }
}
impl HttpResponse {
    /// Parses raw HTTP response bytes
    pub(crate) async fn read(bytes: &[u8]) -> Result<HttpResponse, ()> {
        // Find the end of the headers
        let Some(index) = twoway::find_bytes(bytes, b"\r\n\r\n") else {
            return Err(());
        };

        let size_of_head = index + 4;
        // Separate headers and body
        let header_section = &bytes[..index];

        // Convert header section to &str without unnecessary allocations
        let header_str = std::str::from_utf8(header_section).map_err(|_| ())?;
        let mut headers = HashMap::new();

        let mut lines = header_str.lines();

        // Parse status line
        let Some(status_line) = lines.next() else {
            return Err(());
        };

        let mut parts = status_line.split_whitespace();
        let Some(http_version) = parts.next() else { return Err(()); };
        let Some(status_code) = parts.next().and_then(|s| s.parse::<u16>().ok()) else { return Err(()); };
        let status_label = parts.collect::<Vec<&str>>().join(" ");

        // Parse headers
        let mut content_length = None;
        for line in lines {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
                if key.eq_ignore_ascii_case("Content-Length") {
                    content_length = value.parse::<usize>().ok();
                }
            }
        }



        Ok(HttpResponse {
            http_version: http_version.to_string(),
            status_code,
            status_label,
            headers,
            content_length,
            body:None,
            dropped:Arc::new(Mutex::new(false)),
            size_of_head
        })
    }


    /// getting http response header item
    pub fn get(&self,key:impl Display)->Option<&String>{
        self.headers.get(&format!("{key}"))
    }





    pub async fn get_full_body_bytes(&self)->Result<Vec<u8>,GettingBodyErrors>{
        match & self.body {
            None => {GettingBodyErrors::None.into()}
            Some(b) => {
                match b {
                    HttpBody::Bytes(b) => { Ok(b.to_vec())}
                    HttpBody::Stream(b) => {
                        let mut body = vec![];
                        let mut receiver = b.receiver().await;
                        loop {
                            if let Some((data,end)) = receiver.recv().await {
                                body.extend_from_slice(data.as_slice());
                                if end { break;}
                            } else {
                                return GettingBodyErrors::ConnectionError(Some(body)).into()
                            }
                        }
                        Ok(body)
                    }
                }
            }
        }
    }
}

/// getting body errors
#[derive(Debug)]
pub enum GettingBodyErrors {
    /// when there is no Body expected
    None,
    /// when encountering error while communicating with tcp stream
    ConnectionError(Option<Vec<u8>>)
}

impl<T> Into<Result<T,GettingBodyErrors>> for  GettingBodyErrors {
    fn into(self) -> Result<T, GettingBodyErrors> {
        Err(self)
    }
}
