use httparse;
use std::str;

pub enum FramerError {
    Incomplete(String),
    ParseError(String),
}

pub trait Framer {
    fn from_bytes(&self, raw_bytes: &[u8]) -> Result<Self, FramerError>
    where
        Self: Sized;
    fn to_bytes(&self) -> Vec<u8>;
}

pub struct HttpHeader {
    name: String,
    value: String,
}

pub struct HttpRequest {
    headers: Vec<HttpHeader>,
    method: String,
    path: String,
    pub version: u8,
}

pub struct HttpResponse {
    headers: Vec<HttpHeader>,
    version: u8,
    code: u16,
    reason: String,
}

//impl ParserHandler for HttpRequestFramer {}

impl Framer for HttpRequest {
    fn from_bytes(&self, raw_bytes: &[u8]) -> Result<Self, FramerError>
    where
        Self: Sized,
    {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let res = req.parse(raw_bytes);

        match res {
            Ok(status) => {
                if status.is_complete() {
                    let mut header_vec = Vec::new();
                    for header in req.headers {
                        let (header_name, header_value) = (
                            header.name.to_string(),
                            str::from_utf8(header.value).unwrap().to_string(),
                        );
                        //println!("{}: {}", header_name, header_value);
                        header_vec.push(HttpHeader {
                            name: header_name,
                            value: header_value,
                        });
                    }
                    return Ok(HttpRequest {
                        headers: header_vec,
                        method: req.method.unwrap().to_string(),
                        path: req.path.unwrap().to_string(),
                        version: req.version.unwrap(),
                    });
                } else {
                    return Err(FramerError::Incomplete(
                        "Error: incomplete request".to_string(),
                    ));
                }
            }
            _ => {
                return Err(FramerError::ParseError("Error parsing request".to_string()));
            }
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.method.as_bytes());
        bytes.extend_from_slice(self.path.as_bytes());
        bytes.extend_from_slice(b" HTTP/");
        bytes.extend_from_slice(self.version.to_string().as_bytes());
        bytes.extend_from_slice(b"\r\n");
        for header in &self.headers {
            bytes.extend_from_slice(header.name.as_bytes());
            bytes.extend_from_slice(b" ");
            bytes.extend_from_slice(header.value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"\r\n");
        bytes
    }
}

impl Framer for HttpResponse {
    fn from_bytes(&self, raw_bytes: &[u8]) -> Result<Self, FramerError>
    where
        Self: Sized,
    {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut response = httparse::Response::new(&mut headers);

        let res = response.parse(raw_bytes);

        match res {
            Ok(status) => {
                if status.is_complete() {
                    let mut header_vec = Vec::new();
                    for header in response.headers {
                        let (header_name, header_value) = (
                            header.name.to_string(),
                            str::from_utf8(header.value).unwrap().to_string(),
                        );
                        //println!("{}: {}", header_name, header_value);
                        header_vec.push(HttpHeader {
                            name: header_name,
                            value: header_value,
                        });
                    }
                    return Ok(HttpResponse {
                        headers: header_vec,
                        version: response.version.unwrap(),
                        code: response.code.unwrap(),
                        reason: response.reason.unwrap().to_string(),
                    });
                } else {
                    return Err(FramerError::Incomplete(
                        "Error: incomplete response".to_string(),
                    ));
                }
            }
            _ => {
                return Err(FramerError::ParseError(
                    "Error parsing response".to_string(),
                ));
            }
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"HTTP");
        bytes.extend_from_slice(b"\\");
        bytes.extend_from_slice(self.version.to_string().as_bytes());
        bytes.extend_from_slice(b" ");
        bytes.extend_from_slice(self.code.to_string().as_bytes());
        bytes.extend_from_slice(b" ");
        bytes.extend_from_slice(self.reason.as_bytes());
        bytes.extend_from_slice(b"\r\n");
        for header in &self.headers {
            bytes.extend_from_slice(header.name.as_bytes());
            bytes.extend_from_slice(b" ");
            bytes.extend_from_slice(header.value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"\r\n");
        bytes
    }
}
