use std::{io, thread};
use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use actix::{Actor, Context, Handler, Recipient};
use actix::Message;
use rand::Rng;
use rand::rngs::ThreadRng;
use serialport::SerialPortSettings;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct SerialServer {
    pub rng: ThreadRng,
    pub hosts: Arc<Mutex<HashMap<usize, Recipient<SerialMessage>>>>,
}

#[derive(Clone, Message)]
pub struct SerialMessage {
    pub data: Vec<u8>,
}

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<SerialMessage>,
}

#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
}

impl Actor for SerialServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let mut settings: SerialPortSettings = Default::default();
        settings.timeout = Duration::from_millis(10);
        settings.baud_rate = 9600;

        match serialport::open_with_settings("/dev/ttyUSB0", &settings) {
            Ok(mut port) => {
                let mut serial_buf: Vec<u8> = vec![0; 30];
                println!("Receiving data on \"/dev/ttyUSB0\" at 9600 baud.");
                let hosts = self.hosts.clone();
                thread::spawn(move || {
                    loop {
                        match port.read(serial_buf.as_mut_slice()) {
                            Ok(t) => {
                                let data = &serial_buf[..t];
                                io::stdout().write_all(&data).unwrap();
                                hosts.lock().unwrap().values().for_each(|r|
                                    r.do_send(SerialMessage { data: data.to_vec() }).unwrap());
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to open \"/dev/ttyUSB0\". Error: {}", e);
                ::std::process::exit(1);
            }
        }
    }
}

impl Handler<Connect> for SerialServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.rng.gen::<usize>();
        self.hosts.lock().unwrap().insert(id.clone(), msg.addr);
        id
    }
}

impl Handler<Disconnect> for SerialServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Self::Context) -> Self::Result {
        self.hosts.lock().unwrap().remove(&msg.id);
    }
}