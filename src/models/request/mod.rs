use std::collections::HashMap;
use std::fmt::Display;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use serde::Serialize;
use tokio::sync::{Mutex, MutexGuard};
use water_uri::{IntoUri, Uri};



/// small struct for organizing http request
#[derive(Debug)]
pub struct HttpRequest {
    pub(crate) method:&'static str,
    pub(crate) path:String,
    pub(crate) headers:HashMap<String,String>,
    pub(crate) body: Option<HttpBody>
}
impl   HttpRequest {


    /// creating default request
    pub fn new(into_uri:impl IntoUri ) -> HttpRequest  {
        let mut path = into_uri.to_string();
        if let Ok(uri) = Uri::new(path.clone()) {
            path = uri.path.unwrap();
        }
        HttpRequest {
            method:"GET",
            path,
            headers:HashMap::new(),
            body:None,
        }
    }
    /// creating get request
    pub fn get(into_uri:impl IntoUri) -> HttpRequest  {
        Self::new(into_uri)
    }

    /// creating get request
    pub fn post(into_uri:impl IntoUri) -> HttpRequest  {
        let mut new = Self::new(into_uri);
        new.set_method("POST");
        new
    }

    /// creating get request
    pub fn patch(into_uri:impl IntoUri) -> HttpRequest  {
        let mut new = Self::new(into_uri);
        new.set_method("PATCH");
        new
    }



    /// creating get request
    pub fn delete(into_uri:impl IntoUri) -> HttpRequest  {
        let mut new = Self::new(into_uri);
        new.set_method("PATCH");
        new
    }

    /// for setting custom method for request
    pub fn set_method(&mut self,method:&'static str){
        self.method  = method;
    }

    /// for setting custom body
    pub fn set_body(&mut self,body:HttpBody){
        self.body = Some(body);
    }

    /// for setting custom header
    pub fn set_header(&mut self,key:impl Display,value:impl Display){
        self.headers.insert(format!("{key}"),format!("{value}"));
    }

}


/// Body sender struct for sending body
#[derive(Debug)]
pub enum   HttpBody {
    /// for sending custom bytes
    Bytes(Vec<u8>),

    /// for sending chunks bytes
    Stream(BodyBytesSender)
}


impl  HttpBody {

    /// creating custom bytes from normal strings
    pub  fn from_string(value:impl ToString)->HttpBody{
        Self::from_bytes(value.to_string().as_bytes())
    }

    /// sending custom bytes as results
    pub  fn from_bytes(bytes:&[u8])->HttpBody{
        HttpBody::Bytes(bytes.to_vec())
    }

    /// sending custom json as final results
    pub  fn from_json(json:&impl Serialize)->HttpBody{
        HttpBody::Bytes(serde_json::to_vec(json).unwrap())
    }

    /// creating chunks to send, but you need to set how much data you need to send
    pub  fn send_chunks_stream(length:usize)->(HttpBody,Sender<(BytesSlice,bool)>){
        let (sender,receiver) = channel(length);
        let body = BodyBytesSender{
            receiver:Mutex::new(receiver),
            length
        };
      (  HttpBody::Stream(body),sender)
    }
}

type BytesSlice = Vec<u8>;

/// for generating a channel for receiving events
#[derive(Debug)]
pub  struct BodyBytesSender{
    pub(crate)receiver:Mutex<Receiver<(BytesSlice,bool)>>,
    pub(crate)length:usize
}

impl BodyBytesSender {

    /// returning receiver guard
    pub async fn receiver(&self)->MutexGuard<Receiver<(BytesSlice,bool)>>{
        let r = self.receiver.lock().await;
        r
    }
}

