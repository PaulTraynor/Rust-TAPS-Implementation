use http;
use serde::de;

pub trait Framer {
    type Message;

    fn from_bytes(&self, raw_bytes: &[u8]) -> Self::Message;
    fn to_bytes<'a>(&self, message: &Self::Message) -> &'a [u8];
}

pub struct HttpRequestFramer {}

impl Framer for HttpRequestFramer {
    type Message = http::request::Request<String>;

    fn from_bytes(&self, raw_bytes: &[u8]) -> Self::Message {}

    fn to_bytes(&self, message: &Self::Message) -> &[u8] {}
}
