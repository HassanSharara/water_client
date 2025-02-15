use std::ops::{Add, Deref, DerefMut};
use std::sync::{Arc};

use tokio::{net::TcpStream,sync::Mutex};
#[cfg(feature = "debugging")]
use tracing::debug;
use water_uri::Uri;
use crate::{check_if_err, check_if_nil};
pub (crate) struct TcpConnectionsPool {
    pub (crate) connections:Vec<Arc<Mutex<TcpConnection>>>,
    pub (crate) next_connection:Mutex<usize>,

}

impl TcpConnectionsPool {
    pub (crate) async fn new(url:&Uri,max_connections:usize) -> Self{
        let mut connections = Vec::with_capacity(max_connections);
        for n in 0..max_connections {
            let id = format!("{}",n+1);
            for _ in 0 .. 3 {
                let connection = TcpConnection::new_connection(
                    id.clone(),
                    &url
                ).await;
                if let Ok(connection)  = connection {
                    connections.push(Arc::new(Mutex::new(connection)));
                    break;
                }
            }

        }
        TcpConnectionsPool {
            connections,
            next_connection:0.into()
        }
    }

    pub (crate) async fn get_connection(&self)->Result<Arc<Mutex<TcpConnection>>,ConnectionsError>{
        if self.connections.is_empty() { return ConnectionsError::ThereIsNoTcpConnectionValid.into()}
        let mut next_connection = self.next_connection.lock().await;
        let next_connection_index = next_connection.deref();
        let index = (self.connections.len()-1) .min(*next_connection_index);

        let connection = check_if_nil!(self.connections.get(index),ConnectionsError::ThereIsNoTcpConnectionValid.into())
            .clone();
        *next_connection.deref_mut() =
        if *next_connection_index +1 > self.connections.len() {
            0
        } else {
            *next_connection_index + 1
        };
        Ok(
            connection
        )
     }
}




pub(crate) struct TcpConnection {
    pub (crate) id:String,
    pub (crate) stream:TcpStream,
}
impl TcpConnection {

    pub(crate) async fn new_connection(id:String,uri:&Uri)->Result<Self,()>{
        let mut d = uri.host.as_ref().unwrap_or(&"".to_string()).to_string();
        if d.is_empty() {
            if let Some(ip)= uri.ip.as_ref() {
                d = ip.to_string();
            }
        }
        if d.is_empty() { return Err(());}
        let target = format!("{}:{}",d,uri.port);
        let tcp  = check_if_err!(TcpStream::connect(target).await,Err(()));
        #[cfg(feature = "debugging")]
        {
            debug!("{} connected to host successfully from {:?}",id, tcp.local_addr());
        }
       Ok(
           Self {
               id,
               stream:tcp
           }
       )
    }


    pub(crate) async fn replicate(&self)->Result<Self,()>{
        if let Ok(peer) = self.stream.peer_addr() {
            let connection = Self::new_connection(self.id.clone().add("_u"),
                                                  &format!("{}:{}",
                                                    peer.ip().to_string(),
                                                      peer.port()
                                                  ).into()
            ).await;
            if let Ok(connection) = connection {
                return Ok(
                    connection
                )
            }
        }
        Err(())
    }

}



/// defining tcp connection errors
#[derive(Debug)]
pub enum ConnectionsError {
    ThereIsNoTcpConnectionValid,
}

impl<T> Into<Result<T,ConnectionsError>> for  ConnectionsError {
    fn into(self) -> Result<T, ConnectionsError> {
        Err(self)
    }
}
