use crate::{client::Client, server::Server};
use std::thread;

pub struct App {
    client: Client,
    server: Server,
}

impl App {
    pub async fn new() -> Self {
        let (client_tx, client_rx) = flume::unbounded();
        let client = Client::new(client_tx).await;
        let server = Server::new(client.create_proxy(), client_rx);
        Self { client, server }
    }

    pub fn run(self) {
        thread::spawn(move || self.server.run());
        self.client.run();
    }
}
