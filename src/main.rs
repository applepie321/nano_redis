use std::env;

mod client;
mod server;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} [client|server]", args[0]);
        return;
    }

    match args[1].as_str() {
        "client" => client::main().unwrap(),
        "server" => server::main().unwrap(),
        _ => println!("Invalid argument. Use 'client' or 'server'."),
    }
}
