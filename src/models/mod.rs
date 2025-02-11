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
    /// The IP address of the host (None if using a domain)
    pub ip: Option<IpAddr>,
    /// Domain name (None if using an IP)
    pub host: Option<String>,
    /// Port number
    pub port: u16,
    /// Path in the URL (e.g., `/test`)
    pub path: Option<String>,
    /// URL schema (HTTP/HTTPS)
    pub schema: Schema,
}
impl Uri {
    /// Parses a URL string into a `Uri` struct
    pub fn new(url: impl IntoUri + std::fmt::Display) -> UriResult {
        let mut url = url.to_string();
        let mut schema = Schema::Http; // Default schema

        // Determine schema, if present
        if url.starts_with("https://") {
            schema = Schema::Https;
            url = url.strip_prefix("https://").unwrap().to_owned();
        } else if url.starts_with("http://") {
            url = url.strip_prefix("http://").unwrap().to_owned();
        } else if !url.contains("://") {} else {
            return Err(UriParsingErr::InvalidSchema);
        }

        // Extract domain/host and path
        let (domain_or_host, path) = match url.split_once('/') {
            Some((host, path)) => (host, Some(format!("/{}", path))),
            None => (url.as_str(), None),
        };

        let mut ip = None;
        let mut host = None;
        let mut port = match schema {
            Schema::Http => 80,
            Schema::Https => 443,
        };

        // Handle IPv6 with port `[IPv6]:port`
        if domain_or_host.starts_with('[') && domain_or_host.contains("]:") {
            if let Some((ip_str, port_str)) = domain_or_host.trim_matches('[').split_once("]:") {
                if let Ok(parsed_ip) = IpAddr::from_str(ip_str) {
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
        // Handle IPv4, IPv6, or domain with port (`127.0.0.1:8000`, `example.com:9000`)
        else if let Some((host_part, port_str)) = domain_or_host.rsplit_once(':') {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                port = parsed_port;
            } else {
                return Err(UriParsingErr::InvalidPortNumber);
            }

            if let Ok(parsed_ip) = IpAddr::from_str(host_part) {
                ip = Some(parsed_ip);
            } else {
                host = Some(host_part.to_owned());
            }
        }
        // Handle pure domain or IP without a port (`example.com`, `192.168.1.1`, `localhost`)
        else {
            if let Ok(parsed_ip) = IpAddr::from_str(domain_or_host) {
                ip = Some(parsed_ip);
            } else {
                host = Some(domain_or_host.to_owned());
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

    /// Checks if the host IP address is initialized
    pub fn is_host_initialized(&self) -> bool {
        self.host.is_some()
    }
}
impl Into<Uri> for String {
    fn into(self) -> Uri {
        Uri::new(self).expect("Invalid URL")
    }
}
impl Into<Uri> for &'_ str {
    fn into(self) -> Uri {
        Uri::new(self).expect("Invalid URL")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_urls() {
        let urls = [
            ("http://37.60.240.202:33523/d4443d17", true),
            ("https://[2a02:c206:2239:411::1]:33523/d4443d17", true),
            ("https://example.com/test", true),
            ("localhost/test", true),
            ("localhost:8084/test", true),
            ("http://localhost:8084/test", true),
            ("http4://example.com/test", false), // Invalid schema
        ];

        for (url, should_be_valid) in urls.iter() {
            let result = Uri::new(*url);
            println!("{:?}: {:?}", url, result);
            assert_eq!(result.is_ok(), *should_be_valid);
        }
    }
}
