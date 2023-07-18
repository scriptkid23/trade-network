use libp2p::{core::upgrade::Version, futures::StreamExt};

use libp2p::{
    identity, noise, ping,
    swarm::{keep_alive, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let identity_keys = identity::Keypair::generate_ed25519();

    let local_peer_id = PeerId::from(identity_keys.public());

    println!("Local peer id: {local_peer_id:?}, ");

    let transport = tcp::tokio::Transport::default()
        .upgrade(Version::V1Lazy)
        .authenticate(noise::Config::new(&identity_keys)?)
        .multiplex(yamux::Config::default())
        .boxed();

    let mut swarm =
        SwarmBuilder::with_tokio_executor(transport, Behaviour::default(), local_peer_id).build();

    // Tell the swarm to listen on all interfaces and a random, OS-assigned
    // port.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Dial the peer identified by the multi-address given as the second
    // command-line argument, if any.
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {addr}")
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => println!("{event:?}"),
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour, Default)]
struct Behaviour {
    keep_alive: keep_alive::Behaviour,
    ping: ping::Behaviour,
}
