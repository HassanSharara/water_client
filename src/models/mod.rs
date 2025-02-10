use std::net::IpAddr;
use std::str::FromStr;
use crate::errors::{UriParsingErr, UriResult};
pub use crate::traits::*;

/// Supported URI schemas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schema {
    /// HTTP protocol
    Http,
    /// HTTPS protocol
    Https,
}

/// Struct for parsing URLs into components
#[derive(Debug, Clone)]
pub struct Uri {
    /// the ip address of host
    /// notice that its return [None] When you give a domain without initializing
    pub ip: Option<IpAddr>,
    /// returns domain name
    pub host: Option<String>,
    /// returns the port where using
    pub port: u16,
    /// path of http
    pub path: Option<String>,
    /// url schema
    pub schema: Schema,
}

impl Uri {
    /// Parses a URL string into a `Uri` struct
    pub fn new(url: impl IntoUri + std::fmt::Display) -> UriResult {
        let mut schema = Schema::Http;
        let mut url = ToString::to_string(&url);

        // Determine schema
        if url.starts_with("https://") {
            schema = Schema::Https;
            url = ToString::to_string(url.strip_prefix("https://").unwrap());
        } else if url.starts_with("http://") {
            url = ToString::to_string(url.strip_prefix("http://").unwrap());
        } else {
            return Err(UriParsingErr::InvalidSchema);
        }

        // Extract domain/host and path
        let (domain_or_host, path) = match url.split_once('/') {
            Some((host, path)) => (host, Some(ToString::to_string(path))),
            None => (url.as_str(), None),
        };

        let mut ip = None;
        let mut host = None;
        let mut port = match schema {
            Schema::Http => 80,
            Schema::Https => 443,
        };

        // Handle IPv6 address with port `[IPv6]:port`
        if domain_or_host.starts_with('[') && domain_or_host.contains("]:") {
            if let Some((ip_s, port_str)) = domain_or_host.trim_matches('[').split_once("]:") {
                if let Ok(parsed_ip) = IpAddr::from_str(ip_s) {
                    ip = Some(parsed_ip);
                    if let Ok(parsed_port) = port_str.parse::<u16>() {
                        port = parsed_port;
                    } else {
                        return Err(UriParsingErr::InvalidPortNumber);
                    }
                } else {
                    return Err(UriParsingErr::InvalidIp);
                }
            }
        }
        // Handle IPv4 address with port `127.0.0.1:8000`
        else if let Some((host_part, port_str)) = domain_or_host.split_once(':') {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                port = parsed_port;
            } else {
                return Err(UriParsingErr::InvalidPortNumber);
            }

            if let Ok(parsed_ip) = IpAddr::from_str(host_part) {
                ip = Some(parsed_ip);
            } else {
                host = Some(ToString::to_string(host_part));
            }
        }
        // Handle pure domain or IP (`example.com` or `192.168.1.1`)
        else {
            if let Ok(parsed_ip) = IpAddr::from_str(domain_or_host) {
                ip = Some(parsed_ip);
            } else {
                host = Some(ToString::to_string(domain_or_host));
            }
        }

        Ok(Uri {
            ip,
            host,
            port,
            path,
            schema,
        })
    }

    /// checking if host ip address initialized
    pub fn is_host_initialized(&self)->bool {
        self.host.is_some()
    }
}


impl Into<Uri> for String  {
    fn into(self) -> Uri {
         Uri::new(self).expect("not valid url")
    }
}
impl Into<Uri> for &'_ str  {
    fn into(self) -> Uri {
        Uri::new(self).expect("not valid url")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_urls() {
        let url1 = "http://37.60.240.202:33523/d4443d17";
        let url2 = "https://[2a02:c206:2239:411::1]:33523/d4443d17";
        let url3 = "https://example.com/test";
        let url4 = "http4://example.com/test"; // This should fail

        let uri1 = Uri::new(url1);
        let uri2 = Uri::new(url2);
        let uri3 = Uri::new(url3);
        let uri4 = Uri::new(url4);

        println!("{:?}", uri1);
        println!("{:?}", uri2);
        println!("{:?}", uri3);
        println!("{:?}", uri4);

        assert!(uri1.is_ok());
        assert!(uri2.is_ok());
        assert!(uri3.is_ok());
        assert!(uri4.is_err()); // Expecting an error for invalid schema
    }
}
