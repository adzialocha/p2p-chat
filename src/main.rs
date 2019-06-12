//! Local p2p chat program

mod crypto;
mod discovery;

use std::collections::HashMap;

use futures::{Async, Future, Stream};
use tokio_core::reactor::{Core, Handle};

use discovery::{DiscoveryStream, DiscoveryPeer};

const DISCOVERY_NAME: &[u8] = b"p2p-chat";
const URL_PROTOCOL: &str = "chat://";

pub fn run(
    handle: Handle,
    public_key: &[u8],
) -> impl Future<Item = (), Error = ()> {
    // @TODO Get correct port from listening TCP socket
    let port = 12345;

    // Discover peers which are interested in the same channel
    let discovery_key = crypto::generate_discovery_key(&public_key, DISCOVERY_NAME);
    let discovery_stream = DiscoveryStream::new(handle.clone(), &discovery_key.as_bytes(), port);

    let handle_clone = handle.clone();
    let mut peers: HashMap<String, DiscoveryPeer> = HashMap::new();

    let discovery_future = discovery_stream.map(move |stream| {
        let find_peers = stream.for_each(move |peer| {
            if !peers.contains_key(&peer.token()) {
                // @TODO Start replication protocol
                println!(
                    "New peer: {}, {}, {}",
                    peer.addr(),
                    peer.port(),
                    peer.token()
                );

                peers.insert(peer.token(), peer);
            }

            Ok(())
        });

        handle_clone.spawn(find_peers.then(|_| Ok(())));
    })
    .map_err(|err| {
        panic!("Error: Could not start discovery stream. {:?}", err)
    });

    handle.spawn(discovery_future);

    // Never end this future
    futures::future::poll_fn(|| Ok(Async::NotReady))
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();

    let mut opts = getopts::Options::new();
    opts.optopt("c", "channel", "join chat channel with this URL", "<link>");

    // Generate public and secret keypair
    let keypair = crypto::generate_keypair();

    // Create new channel or join existing one depending on given arguments
    let matches = opts.parse(&args[1..]).unwrap();
    let is_channel_given = matches.opt_present("channel");

    // Prepare chat:// URL with public key
    let decoded_key;

    let public_key: &[u8] = if is_channel_given {
        let channel_public_key = matches
            .opt_str("channel")
            .unwrap()
            .replace(URL_PROTOCOL, "");

        decoded_key = hex::decode(channel_public_key).unwrap();
        &decoded_key
    } else {
        keypair.public.as_bytes()
    };

    println!("{}{}", URL_PROTOCOL, hex::encode(public_key));

    // Create event loop to drive the networking I/O
    let mut core = Core::new().unwrap();

    // Create a new chat instance
    let main = run(core.handle(), public_key);

    // ... and add it to event loop
    core.run(main).unwrap();
}
