use std::io::{self, BufRead as _, BufReader, Write as _};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;

/// Client connects to this socket path.
const SOURCE_PATH: &str = "/tmp/elci-proxy";
/// Address of ELCI server, which requests are proxied to.
const DESTINATION_ADDRESS: &str = "127.0.0.1:4711";

/// Commmand names which expect a response.
const RESPONSE_COMMANDS: &[&str] = &[
    "player.getPos",
    "world.getBlockWithData",
    "world.getHeight",
    "world.getBlocksWithData",
    "world.getHeights",
];

fn main() -> io::Result<()> {
    let path = Path::new(SOURCE_PATH);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    println!(" LISTENING: {}", SOURCE_PATH);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_client(stream).expect("Error in worker thread");
                });
            }
            Err(error) => {
                eprintln!("{}", error);
                break;
            }
        }
    }
    Ok(())
}

fn handle_client(client: UnixStream) -> io::Result<()> {
    println!("CONNECTION: OPENED");

    // Each client has own server connection
    let server = TcpStream::connect(DESTINATION_ADDRESS)?;

    let mut server_writer = server.try_clone()?;
    let mut server_reader = BufReader::new(server);
    let mut client_writer = client.try_clone()?;
    let mut client_reader = BufReader::new(client);

    loop {
        // Read request from client and pass to server
        let mut request = String::new();
        let bytes_read = client_reader.read_line(&mut request)?;
        if bytes_read == 0 {
            break;
        }
        println!("   COMMAND: {}", request.trim());
        // Pass request to server
        server_writer.write_all(request.as_bytes())?;

        // If client expects response, read response from server and pass to client
        let command = match request.split_once("(") {
            Some((command, _)) => command,
            None => &request,
        };
        if !RESPONSE_COMMANDS.contains(&command) {
            continue;
        }
        let mut response = String::new();
        server_reader.read_line(&mut response)?;
        client_writer.write_all(response.as_bytes())?;
    }

    println!("CONNECTION: CLOSED");

    Ok(())
}
