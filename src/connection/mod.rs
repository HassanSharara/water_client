use std::ops::{Add, Deref, DerefMut};
use std::sync::{Arc};

use tokio::{net::TcpStream,sync::Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt, };
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
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
                    &url,
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
    pub (crate) stream:TcpStreamEnum,
    pub (crate) uri:Uri,
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
        if let water_uri::Schema::Https = uri.schema {
            let mut trusted_certificates =
            RootCertStore::empty();
            trusted_certificates.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let  config_builder = ClientConfig::builder()
                .with_root_certificates(trusted_certificates)
                .with_no_client_auth();

            let tls_connector = TlsConnector::from(Arc::new(config_builder));
            let server_name =
            tokio_rustls::rustls::pki_types::ServerName::try_from(
                d
            ).expect("can not connect to given domain or host");

            let connection = check_if_err!(tls_connector.connect(server_name,tcp).await,
                Err(()));

            return  Ok(
                Self {
                    id,
                    stream:TcpStreamEnum::Tls(connection),
                    uri:uri.clone()
                }
            )
        }
       Ok(
           Self {
               id,
               stream:TcpStreamEnum::Stream(tcp),
               uri:uri.clone()
           }
       )
    }


    pub(crate) async fn replicate(&self)->Result<Self,()>{
        let connection = Self::new_connection(self.id.clone().add("_u"),
                                              &self.uri,
        ).await;
        if let Ok(connection) = connection {
            return Ok(
                connection
            )
        }
        Err(())
    }

}


pub (crate) enum TcpStreamEnum {
    Stream(TcpStream),
    Tls(TlsStream<TcpStream>)
}

impl TcpStreamEnum  {



   pub (crate) async fn write_all(&mut self,bytes:&[u8])->std::io::Result<()>{
       match self {
           TcpStreamEnum::Stream(s) => {
               s.write_all(bytes).await
           }
           TcpStreamEnum::Tls(s) => {
               s.write_all(bytes).await
           }
       }
   }


    pub (crate) async fn read_buf(&mut self,bytes:&mut Vec<u8>)->std::io::Result<usize>{
        match self {
            TcpStreamEnum::Stream(s) => {
                s.read_buf(bytes).await
            }
            TcpStreamEnum::Tls(s) => {
                s.read_buf(bytes).await
            }
        }
    }

}


/// defining tcp connection errors
#[derive(Debug)]
pub enum ConnectionsError {
    /// when there is no valid tcp connection to use
    ThereIsNoTcpConnectionValid,
    /// when error happen while connecting vi tls
    CouldNotConnectToHostWithTlsConfigurations,
}
impl<T> Into<Result<T,ConnectionsError>> for  ConnectionsError {
    fn into(self) -> Result<T, ConnectionsError> {
        Err(self)
    }
}
