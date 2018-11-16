#[macro_use]
extern crate serde_derive;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::error::Error;

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
    repo_tags: Vec<String>,
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

    pub fn request(&self, method: &str, path: &str) -> Result<Response, Box<Error>> {
        let mut client = UnixStream::connect(&self.socket)?;
        let buf = format!("{} {} HTTP/1.1\r\nConnection: close\r\nHost: {}\r\n\r\n", method, path, self.host_name);
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
        Ok(Response { status, body: Box::new(r) })
    }

    pub fn images(&self) -> Result<Vec<Image>, Box<Error>> {
        let res = self.request(&"GET", &"/images/json")?;
        println!("STATUS: |{}|", res.status);
        let images: Vec<Image> = serde_json::from_reader(res.body)?;
        Ok(images)
    }
}


fn main() -> Result<(), Box<Error>> {
    let docker = Docker::connect_with_defaults()?;

    println!("Images: {:?}", docker.images());
    println!("");

    // let mut r = docker.request(&"GET", &"/images/json")?;
    // let mut body: Vec<u8> = Vec::new();
    // r.read_to_end(&mut body)?;
    // println!("{}", String::from_utf8(body)?);

    Ok(())
}
