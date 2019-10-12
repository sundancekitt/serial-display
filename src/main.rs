#[macro_use]
extern crate log;
extern crate serialport;

use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, ActorFuture, Addr, AsyncContext, ContextFutureSpawner, fut, Handler, Running, StreamHandler, System, WrapFuture};
use actix_files as fs;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, middleware, web};
use actix_web_actors::ws;

use crate::serial::SerialMessage;

mod serial;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

fn ws_index(req: HttpRequest, stream: web::Payload, srv: web::Data<Addr<serial::SerialServer>>) -> Result<HttpResponse, Error> {
    ws::start(
        WsSession {
            id: 0,
            heartbeat: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct WsSession {
    id: usize,
    heartbeat: Instant,
    addr: Addr<serial::SerialServer>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        let addr = ctx.address();
        self.addr.send(serial::Connect { addr: addr.recipient() })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(id) => act.id = id,
                    _ => ctx.stop(),
                }
                fut::ok(())
            }).wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(serial::Disconnect { id: self.id });
        Running::Stop
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.heartbeat = Instant::now();
            }
            ws::Message::Text(_) => (),
            ws::Message::Binary(_) => (),
            ws::Message::Close(_) => ctx.stop(),
            ws::Message::Nop => (),
        }
    }
}

impl Handler<serial::SerialMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: SerialMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(String::from_utf8(msg.data).unwrap());
    }
}

impl WsSession {
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                info!("WebSocket client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let sys = System::new("serial_display");

    let serial_address = serial::SerialServer::default().start();

    HttpServer::new(move || {
        App::new()
            .data(serial_address.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            .service(fs::Files::new("/", "static/").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .start();

    info!("Started http server: 127.0.0.1:8080");
    sys.run()
}