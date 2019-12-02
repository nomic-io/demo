fn main() {
    let socket = ws::WebSocket::new(NullFactory).unwrap();
    let broadcast = socket.broadcaster();
    
    std::thread::spawn(move || loop {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let message = format!("{}", time);
        broadcast.send(message).unwrap();
        std::thread::sleep_ms(1000);
    });

    socket.listen("localhost:8080").unwrap();
}

struct NullHandler;
impl ws::Handler for NullHandler {}

struct NullFactory;
impl ws::Factory for NullFactory {
    type Handler = NullHandler;
    
    fn connection_made(&mut self, conn: ws::Sender) -> NullHandler {
        NullHandler
    }
}
