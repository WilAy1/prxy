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
    state: State,
}

#[derive(Debug)]
pub enum ParseResult<T> {
    Complete(T),
    NeedMoreData,
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

            if !progressed {
                break;
            }
        }

        println!("{:?}", self.state);        

        Ok(())
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
                
            let version = parts
                .next()
                .ok_or(ParseError::InvalidRequestLine)?;
                
            if version != "HTTP/1.1" {
                return Err(ParseError::InvalidRequestLine);
            }
        
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

        for header in headers_parts {
            let header_parts = header.split_once(':');
            let Some((key, value)) = header_parts else {
                return Err(ParseError::InvalidHeader);
            };
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
        };

        self.buffer.drain(..pos + 4);

        Ok(true)
    }

    pub fn parse_body(&mut self) -> Result<bool, ParseError> {
         let State::ReadingBody { method, path, headers
        } = &self.state else {
            unreachable!();
        };

        let method = method.clone();
        let path = path.clone();
        let headers = headers.clone();

        // return body as it is
        let body = str::from_utf8(&self.buffer[..])
            .map_err(|_| ParseError::InvalidBody)?
            .to_owned();

        self.state = State::Complete {
            method,
            path,
            headers,
            body,
        };

        self.buffer.drain(..);
        Ok(true)
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