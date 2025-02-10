/// Defining rules for parsing [IntoUri] traits
pub trait IntoUri {
    fn to_string(self) -> String;
}

impl IntoUri for String {
    fn to_string(self) -> String {
        self
    }
}

impl<'a> IntoUri for &'a str {
    fn to_string(self) -> String {
        self.to_owned()
    }
}
