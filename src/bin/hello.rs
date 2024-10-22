use std::io::Write;
use std::net::TcpListener;

fn main() {
    // Bind allows us to create a connection on the port
    // and gets it ready to accept connections.
    let listener = TcpListener::bind("127.0.0.1:8081").unwrap();

    // The listener's accept method waits or 'blocks' until
    // we have a connection and then returns a new TcpStream
    // that we can read and write data to.
    let mut stream = listener.accept().unwrap().0;
    let message = "Hello, World!";
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
                              Content-Type: text/html; charset=utf-8\r\n\
                              Content-Length: {}\r\n\
                              \r\n\
                              {}",
        message.len(),
        message
    );
    let _ = stream.write(response.as_bytes());
}
