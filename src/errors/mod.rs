use crate::Uri;

/// Uri Result
pub type UriResult = Result<Uri,UriParsingErr>;

/// defining parsing errors
#[derive(Debug)]
pub enum UriParsingErr {
    /// if url start with invalid schema
    InvalidSchema,
    /// if url contains with invalid ip address
    InvalidIp,
    /// if url contains invalid port number format
    InvalidPortNumber,
    /// if url contains invalid domain name or port number format
    InvalidDomainName

}


impl Into<UriResult> for UriParsingErr {
    fn into(self) -> UriResult {
        return Err(self)
    }
}