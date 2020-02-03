use crate::{config::Config, http::controller::ServerController};
use log4rs::Handle;
use saphir::server::Server as SaphirServer;
use tokio::runtime::Runtime;

pub struct HttpServer {
    pub server: SaphirServer,
}

impl HttpServer {
    pub fn new(config: Config, log_handle: Handle) -> Self {
        let controller = match ServerController::new(config, log_handle) {
            Ok(controller) => controller,
            Err(e) => panic!("Couldn't build server controller: {}", e),
        };

        let server = SaphirServer::builder()
            .configure_router(|r| r.controller(controller))
            .configure_listener(|l| l.interface("0.0.0.0:12345"))
            .build();

        HttpServer { server }
    }

    pub fn run(self) {
        // Create the runtime
        let mut rt = Runtime::new().expect("create tokio Runtime");

        rt.block_on(async {
            if let Err(e) = self.server.run().await {
                log::error!("{:?}", e);
            }
        });
    }
}
