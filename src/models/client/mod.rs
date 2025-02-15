use std::ops::{Add, Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use water_uri::Uri;
use crate::check_if_err;
use crate::connection::{ConnectionsError, TcpConnection, TcpConnectionsPool};
use crate::models::request::{BodyBytesSender, HttpBody, HttpRequest};
use crate::models::response::HttpResponse;

/// a http Client Builder for generating new ['Client']
pub struct  ClientBuilder {
    /// setting up max connections count to connect to single host
    pub max_connections:usize,
    /// setting up max body reading size to accept
    pub max_body_size:Option<usize>,
}

impl ClientBuilder {

    /// for creating default client builder configurations
    /// Note  :
    ///
    /// using default configuration has unrecommended use of max body size
    pub fn default()->Self{
        ClientBuilder {
            max_connections:1,
            max_body_size:None
        }
    }
}


/// Http Client Configuration setup
pub struct HttpClient {
    uri:Uri,
    configurations:ClientBuilder,
    pool:Option<TcpConnectionsPool>
}




macro_rules! refresh_connection {
    ($connection_arc:ident,$uri:expr) => {
        let cc = $connection_arc.clone();
        tokio::spawn(async move {
                     let uri  = $uri;
                     let connection_arc = cc;
                     let mut connection = connection_arc.lock().await;
                     let new_connection = connection.replicate().await;
                     match new_connection {
                         Ok(c) => {
                             *connection.deref_mut() = c;
                         }
                         Err(_) => {
                             let new_connection = TcpConnection::new_connection(
                                 connection.id.clone().add("_uo"),
                                 &uri

                             ).await;
                             if let Ok(new_connection) = new_connection {
                                 *connection.deref_mut() = new_connection;
                             }
                         }
                     }

                 });
    };
}


macro_rules! read_bytes {
    ($arc:ident,$connection:ident,$self:ident) => {

  let connection_arc = $arc.clone();
  let mut read_bytes = Vec::with_capacity(4800);

       loop {

           if let Ok(Ok(_)) = tokio::time::timeout(
               Duration::from_secs(20),
               $connection.stream.read_buf(&mut read_bytes)
           ).await {
               let response = HttpResponse::read(&read_bytes).await;
               if let Ok(mut response ) = response {
                   if let Some(content_length) = response.content_length.as_ref() {
                       if content_length <= $self.configurations.max_body_size.as_ref()
                           .unwrap_or(&(1024*8)) {

                          let mut body_bytes = Vec::with_capacity(*content_length);
                          let left_bytes = &read_bytes[response.size_of_head.min(read_bytes.len())..];
                          let mut left_to_parse_as_body = *content_length;
                          let to_extend =  left_to_parse_as_body.min(left_bytes.len());
                          body_bytes.extend_from_slice(
                              &left_bytes[..to_extend]
                          );
                          left_to_parse_as_body -= to_extend;
                          if left_to_parse_as_body == 0 {
                               body_bytes.truncate(*content_length);
                               response.body = Some(HttpBody::Bytes(body_bytes));
                               return Ok(response.into())
                           }

                           loop {
                              if let Ok(s) = $connection.stream.read_buf(&mut body_bytes).await {
                                  if s >= left_to_parse_as_body {
                                      body_bytes.truncate(*content_length);
                                      response.body = Some(
                                          HttpBody::Bytes(body_bytes)
                                      );
                                      return Ok(response);
                                  }
                                  left_to_parse_as_body -= s;
                              }
                           }
                       }
                   }
                   else {
                       let left_bytes = &read_bytes[response.size_of_head.min(read_bytes.len())..];
                       if left_bytes.is_empty() { return Ok(response);}
                       let (sender,receiver) = channel::<(Vec<u8>,bool)>(
                           *$self.configurations.max_body_size.as_ref()
                               .unwrap_or(&(1024*8))
                       );
                       response.body = Some(HttpBody::Stream(
                           BodyBytesSender{
                               receiver:Mutex::new(receiver),
                               length:0
                           }
                       ));
                       let is_response_dropped = response.dropped.clone();
                       let connection_arc = connection_arc.clone();
                       tokio::spawn(async move {

                           let mut connection = connection_arc.lock().await;
                           let mut body = vec![];
                           let  sender = sender;


                           loop {
                            let is_response_dropped = is_response_dropped.clone();
                            let check_if_dropped = is_response_dropped.lock().await.deref() == &true;
                            if check_if_dropped {return;}
                               if let Ok(Ok(s)) = tokio::time::timeout(
                                   Duration::from_secs(10),
                                   connection.stream.read_buf(&mut body)
                               ).await  {
                                   if s==0 {
                                       if sender.send((vec![],true)).await.is_err() {return ;}
                                       break;}
                                   if sender.send(((&body[..s]).to_vec(),false)).await.is_err() {return ;}
                                   body.clear();
                                   continue;
                               }
                               if sender.send((vec![],true)).await.is_err() {return ;}
                               break;
                           }
                       });
                   }
                   break;
               }
           } else {
               let uri = $self.uri.clone();
               refresh_connection!(connection_arc,uri);
               return SendingRequestError::ReadingErrors.into();
           }
       }
    };
}

macro_rules! send_bytes {
    ($connection:ident,$bytes:ident,$arc:ident,$self:ident) => {

        if let Ok(_) = $connection.stream.write_all(&$bytes).await {
            read_bytes!($arc,$connection,$self);
        } else {
            let uri = $self.uri.clone();
            refresh_connection!($arc,uri);
            return SendingRequestError::WritingErrors.into();
        }
    };

     ($connection:ident,$bytes:ident,$arc:ident,$self:ident,$on_send:block) => {

        if let Ok(_) = $connection.stream.write_all(&$bytes).await {
            $on_send;
        } else {
            let uri = $self.uri.clone();
            refresh_connection!($arc,uri);
            return SendingRequestError::WritingErrors.into();
        }
    };
}
impl HttpClient {

    /// creating new http client wrapper for custom domain or host
    pub fn new(uri:Uri)->Self{

        Self {
            uri,
            configurations:ClientBuilder::default(),
            pool:None
        }
    }

    /// connect to the host server using max connections property
    pub async fn init_connection(&mut self){
        let pool = TcpConnectionsPool::new(
            &self.uri,
            self.configurations.max_connections
        ).await;
        self.pool = Some(pool);
    }



    pub (crate) async fn get_connection(&self)->Result<Arc<Mutex<TcpConnection>>,ConnectionsError>{
        let connection = self.pool.as_ref();
        if let Some(connection) = connection {
            let connection = connection.get_connection().await;
            return connection
        }
        return ConnectionsError::ThereIsNoTcpConnectionValid.into()
    }
   /// for sending http request using connections pool

   pub async fn send_request(&mut self,mut request:HttpRequest)->Result<HttpResponse,SendingRequestError>{
       let  connection_arc = check_if_err!(self.get_connection().await,
         ConnectionsError::ThereIsNoTcpConnectionValid.into()
       );
       let mut connection = connection_arc.lock().await;
       match request.body {
           None => {
               let head_bytes = request.writeable_head_bytes();
               send_bytes!(connection,head_bytes,connection_arc,self);
           }
           Some(ref body) => {
               match body {
                   HttpBody::Bytes(data) => {
                       let data = data.clone();
                       request.set_header("Content-Length",data.len());
                       let head_bytes = request.writeable_head_bytes();
                       send_bytes!(connection,head_bytes,connection_arc,self,{});
                       send_bytes!(connection,data,connection_arc,self);
                   }
                   HttpBody::Stream(body_stream) => {
                       let content_length = body_stream.length;
                       request.set_header("Content-Length",content_length);
                   }
               }
           }
       }

       if let  Some(HttpBody::Stream(body_stream)) = request.body.as_ref() {
           let head_bytes = request.writeable_head_bytes();
           send_bytes!(connection,head_bytes,connection_arc,self, {
                           let mut receiver = body_stream.receiver().await;
                           while let Some((bytes,end)) = receiver.deref_mut().recv().await {
                               send_bytes!(connection,bytes,connection_arc,self,{
                                  if end { break; }
                                   continue;});

                           }
                       });
           read_bytes!(connection_arc,connection,self);
       }

       SendingRequestError::WritingErrors.into()
   }

}





impl HttpRequest {

    pub (crate) fn writeable_head_bytes(&self)->Vec<u8>{
        let mut to_send = Vec::with_capacity(1000);
        to_send.extend_from_slice(format!("{} {} HTTP/1.1\r\n",self.method,self.path.replace("//","/")).as_bytes());
        for (key,value) in &self.headers {
            to_send.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
        }
        to_send.extend_from_slice(b"\r\n");
        to_send
    }
}
#[derive(Debug)]
pub enum SendingRequestError {
    TcpCErrors(ConnectionsError),
    WritingErrors,
    ReadingErrors
}
impl <T> Into<Result<T,SendingRequestError>> for ConnectionsError {
    fn into(self) -> Result<T, SendingRequestError> {
        SendingRequestError::TcpCErrors(self).into()
    }
}
impl<T> Into<Result<T,SendingRequestError>> for SendingRequestError {
    fn into(self) -> Result<T,SendingRequestError> {
        Err(self)
    }
}

#[test]
mod tests {

    #[test]
    fn test_request(){

    }
}