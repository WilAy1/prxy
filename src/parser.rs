use std::{io::Write, sync::LazyLock};

use regex::Regex;

#[derive(Debug)]
pub enum ParseError {
    InvalidRequestLine,
    InvalidHeader,
    InvalidUtf8,
    InvalidBody,
}

#[derive(Debug)]
pub enum State {
    ReadingStartLine,
    ReadingHeaders {
        method: String,
        path: String,
    },
    ReadingBody {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        content_length: usize,
    },
    Complete {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: String,
    },
}


#[derive(Debug)]
pub struct Parser {
    buffer: Vec<u8>,
    pub state: State,
}


impl Parser {
    pub fn feed(&mut self, chunk: &[u8]) -> Result<(), ParseError> {
        self.buffer.extend_from_slice(chunk);

        loop {
            let progressed = match &self.state {
                State::ReadingStartLine => self.parse()?,
                State::ReadingHeaders { .. } => self.parse_headers()?,
                State::ReadingBody { .. } => self.parse_body()?,
                State::Complete { .. } => false,
            };

            // collect all chunks before breaking
            if !progressed {
                break;
            }
        }

        // println!("{:?}", self.state);

        Ok(())
    }

    pub fn rewrite_request(&mut self) -> Result<Vec<u8>, ParseError> {
        let State::Complete { method, path, headers, body } = &self.state else {
            unreachable!();
        };
        
        let (_domain, _port, uri) = extract_url_parts(&path.as_str()).unwrap();
        // write start line
        let start_line = format!("{} {} HTTP/1.1", method, uri);

        let headers_str = headers
            .iter()
            .filter(|(key, _)| *key != "Proxy-Connection")
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect::<Vec<String>>()
            .join("\r\n");

        let request = format!("{}\r\n{}\r\n\r\n{}", start_line, headers_str, body);
        
        let mut buffer: Vec<u8> = Vec::new();
        
        buffer.write_all(request.as_bytes()).map_err(|_| ParseError::InvalidUtf8)?;

        Ok(buffer)
    }

    pub fn new() -> Self {
        Parser { state: State::ReadingStartLine, buffer: Vec::new() }
    }

    pub fn parse(&mut self) -> Result<bool, ParseError> {
        let Some(pos) = find_crlf(&self.buffer) else {
            return Ok(false);
        };

        let (method, path) = {
            // convert line to readable string
            let line = std::str::from_utf8(&self.buffer[..pos])
                .map_err(|_| ParseError::InvalidUtf8)?;
                
            let mut parts = line.split_whitespace();
                
            let method = parts
                .next()
                .ok_or(ParseError::InvalidRequestLine)?
                .to_owned();
                
            let path = parts
                .next()
                .ok_or(ParseError::InvalidRequestLine)?
                .to_owned();
                
            let _version = parts
                .next()
                .ok_or(ParseError::InvalidRequestLine)?;
                
            // if version != "HTTP/1.1" {
            //     return Err(ParseError::InvalidRequestLine);
            // }
        
            if parts.next().is_some() {
                return Err(ParseError::InvalidRequestLine);
            }
        
            (method, path)
        };

        self.buffer.drain(..pos + 2);

        self.state =  
            State::ReadingHeaders { method: method.to_owned(), path: path.to_owned() };

        Ok(true)
    }

    pub fn parse_headers(&mut self) -> Result<bool, ParseError> {
        let Some(pos) = find_empty_line(&self.buffer) else {
            return Ok(false);
        };

        let mut headers: Vec<(String, String)> = Vec::new();
        let headers_parts = std::str::from_utf8(&self.buffer[..pos])
            .map_err(|_| ParseError::InvalidHeader)?
            .split("\r\n")
            .into_iter();

        let mut content_length = 0;

        for header in headers_parts {
            let header_parts = header.split_once(':');
            let Some((key, value)) = header_parts else {
                return Err(ParseError::InvalidHeader);
            };
            if key == "Content-Length" {
                content_length = value.parse().map_err(|_| ParseError::InvalidHeader)?;
            }
            headers.push((key.trim().to_owned(), value.trim().to_owned()));
        }

        let State::ReadingHeaders { method, path } = &self.state else {
            unreachable!();
        };

        let method = method.clone();
        let path = path.clone();

        self.state = State::ReadingBody {
            method,
            path,
            headers,
            content_length
        };

        self.buffer.drain(..pos + 4);

        Ok(true)
    }

    pub fn parse_body(&mut self) -> Result<bool, ParseError> {
         let State::ReadingBody { method, path, headers,
            content_length,
        } = &self.state else {
            unreachable!();
        };


        if self.buffer.len() < content_length.clone() {
            return Ok(false);
        }

        let body = self.buffer[..content_length.clone()].to_vec();

        self.buffer.drain(..content_length);

        let method = method.clone();
        let path = path.clone();
        let headers = headers.clone();

        // return body as it is
        let body = str::from_utf8(&body)
            .map_err(|_| ParseError::InvalidBody)?
            .to_owned();

        self.state = State::Complete {
            method,
            path,
            headers,
            body,
        };

        Ok(true)
    }

    pub fn get_header(&self, key: String) -> Option<String> {
        let State::Complete { method: _, path: _, headers, body: _ } = &self.state else {
            unreachable!();
        };

        let headers = headers.clone();

        let Some((_, value)) = headers.iter().find(|&(h_key, _)| *h_key == key) else {
            return None;
        };
        
        Some(value.to_owned())
    }
}

// find \r\n
fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(2)
        .position(|window| window == b"\r\n")
}

fn find_empty_line(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
}

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?P<domain>[^:/\s]+)(?::(?P<port>\d+))?(?P<uri>/[^\s]*)?").unwrap()
});

pub fn extract_url_parts(url: &str) -> Result<(String, String, String), &'static str> {
    if let Some(caps) = URL_REGEX.captures(url) {
        let domain = &caps["domain"].to_string();
        
        let port = &caps.name("port")
                       .map(|m| m.as_str().to_owned())
                       .unwrap_or("80".to_owned());

        let uri = caps.name("uri")
                      .map(|m| m.as_str().to_owned())
                      .unwrap_or("/".to_owned());

        Ok((domain.clone(), port.clone(), uri.clone()))
    } else {
        Err("Invalid url")
    }
}