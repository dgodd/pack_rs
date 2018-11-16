#[macro_use]
extern crate serde_derive;

use std::error::Error;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;

#[cfg(unix)]
pub const DEFAULT_DOCKER_HOST: &'static str = "unix:///var/run/docker.sock";
#[cfg(windows)]
pub const DEFAULT_DOCKER_HOST: &'static str = "npipe:////./pipe/docker_engine"; // "tcp://localhost:2375";

pub struct Docker {
    socket: String,
    host_name: String,
}
pub struct Response {
    status: i32,
    body: Box<Read>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    #[serde(rename = "Id")]
    id: String,
    created: i32,
    containers: i32,
    repo_tags: Option<Vec<String>>,
    size: i32,
}

impl Docker {
    pub fn connect_with_defaults() -> Result<Docker, Box<Error>> {
        let host = std::env::var("DOCKER_HOST").unwrap_or(DEFAULT_DOCKER_HOST.to_string());
        Docker::connect_with_unix(&host)
    }
    #[cfg(unix)]
    pub fn connect_with_unix(addr: &str) -> Result<Docker, Box<Error>> {
        let socket = addr[7..].to_string();
        let host_name = "localhost".to_string();
        Ok(Docker { socket, host_name })
    }
    #[cfg(not(unix))]
    pub fn connect_with_unix(_addr: &str) -> Result<Docker, Box<Error>> {
        panic!("not implemented")
    }

    pub fn request(&self, method: &str, path: &str) -> Result<Response, Box<Error>> {
        let mut client = UnixStream::connect(&self.socket)?;
        let buf = format!(
            "{} {} HTTP/1.1\r\nConnection: close\r\nHost: {}\r\n\r\n",
            method, path, self.host_name
        );
        client.write_all(buf.as_bytes())?;

        let mut r = BufReader::new(client);
        let mut line = String::new();
        r.read_line(&mut line)?;
        line.truncate(line.len() - 2);
        let status = line[9..12].parse::<i32>()?;
        loop {
            line.clear();
            r.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            // line.truncate(line.len() - 2);
            // println!("HEADER: {}", line);
        }
        Ok(Response {
            status,
            body: Box::new(r),
        })
    }

    pub fn images(&self) -> Result<Vec<Image>, Box<Error>> {
        let res = self.request(&"GET", &"/images/json")?;
        if res.status != 200 {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("Status: {} != 200", res.status),
            )));
        }
        let image: Vec<Image> = serde_json::from_reader(res.body)?;
        Ok(image)
    }

    pub fn pull(&self, repo_name: &str) -> Result<(), Box<Error>> {
        let res = self.request(&"POST", &format!("/images/create?fromImage={}", repo_name))?;
        if res.status != 200 {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("Status: {} != 200", res.status),
            )));
        }

        let mut r = BufReader::new(res.body);
        let mut line = String::new();
        let mut body: Vec<u8> = Vec::new();
        loop {
            line.clear();
            r.read_line(&mut line)?;
            if line.len() >= 2 {
                line.truncate(line.len() - 2);
            }
            if line == "" {
                continue;
            }
            let size = usize::from_str_radix(&line, 16)?;
            if size == 0 {
                break;
            }
            body.resize(size, 0);
            r.read_exact(&mut body)?;
            if body.len() >= 2 {
                body.truncate(body.len() - 2);
            }
            let body = String::from_utf8(body.to_vec())?;
            println!("BODY: |{:?}|", &body);
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<Error>> {
    let docker = Docker::connect_with_defaults()?;

    println!("Images: {:?}", docker.images());
    println!("");
    println!("Pull: redis");
    docker.pull(&"redis:latest")?;

    Ok(())
}
