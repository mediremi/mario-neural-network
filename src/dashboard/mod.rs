use crate::utils::{Tile, Screen};

use crossbeam::channel::{unbounded, Receiver, Sender};
use simple_server::{Method, Server, StatusCode};
use tungstenite::server::accept;

use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::net::TcpListener;
use std::path::Path;
use std::thread::spawn;

#[derive(Copy, Clone)]
pub struct DashboardOptions {
    pub host: &'static str,
    pub port: u64,
}

impl fmt::Debug for Tile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_fmt(format_args!("{}", *self as u64))
    }
}

enum Message {
    UpdateScreen(Screen),
}

impl Message {
    fn serialise_screen(screen: Screen) -> String {
        let data = screen
            .iter()
            .map(|row| format!("{:?}", row))
            .collect::<Vec<String>>()
            .join(",");
        format!("{{ \"event\": \"update_screen\", \"data\": [{}] }}", data)
    }

    fn serialise(&self) -> String {
        match self {
            Self::UpdateScreen(screen) => Self::serialise_screen(*screen)
        }
    }
}

pub struct Dashboard {
    sender: Sender<Message>,
}

impl Dashboard {
    pub fn new(options: DashboardOptions) -> Dashboard {
        let (tx, rx) = unbounded();
        spawn(move || Dashboard::run_http_server(options.host, options.port));
        spawn(move || Dashboard::run_websockets_server(options.host, rx));
        Dashboard { sender: tx }
    }

    fn run_http_server(host: &'static str, port: u64) {
        let mut server =
            Server::new(
                |request, mut response| match (request.method(), request.uri().path()) {
                    (&Method::GET, "/") => {
                        let path = Path::new("dashboard/index.html");
                        let mut file = match File::open(&path) {
                            Err(err) => panic!("couldn't open index.html: {}", err),
                            Ok(file) => file,
                        };
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)?;
                        Ok(response.body(buffer)?)
                    }
                    (_, _) => {
                        response.status(StatusCode::NOT_FOUND);
                        Ok(response.body(b"Error 404 - Not found".to_vec())?)
                    }
                },
            );
        server.set_static_directory("dashboard/");
        println!("Dashboard listening on http://{}:{}", host, port);
        let port = format!("{}", port);
        server.listen(host, &port);
    }

    fn run_websockets_server(host: &'static str, rx: Receiver<Message>) {
        use tungstenite::Message::Text;
        let port = 9000;
        let addr = format!("{}:{}", host, port);
        let server = TcpListener::bind(addr).unwrap();
        for stream in server.incoming() {
            let rx = rx.clone();
            spawn(move || {
                let mut websocket = accept(stream.unwrap()).unwrap();
                loop {
                    let message = rx.recv().unwrap();
                    websocket.write_message(Text(message.serialise())).unwrap();
                }
            });
        }
    }

    pub fn update_screen(&self, screen: Screen) {
        self.sender.send(Message::UpdateScreen(screen)).unwrap();
    }
}
