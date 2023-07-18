use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::OrTransport;
use libp2p::futures::future::Either;
use libp2p::futures::StreamExt;
use libp2p::Transport;
use libp2p::{
    identity, noise, ping,
    swarm::{keep_alive, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId,
};
use libp2p_quic as quic;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let identity_keys = identity::Keypair::generate_ed25519(); // Based on elliptic curve cryptography

    let local_peer_id = PeerId::from(identity_keys.public());

    println!("Local peer id: {local_peer_id:?}, ");

    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(libp2p::core::upgrade::Version::V1Lazy)
        .authenticate(
            noise::Config::new(&identity_keys).expect("signing libp2p-noise static keypair"),
        )
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed();

    /*

    QUIC (Quick UDP Internet Connections) is a transport protocol designed to provide secure, reliable, and low-latency communication over the Internet.
    It is built on top of UDP (User Datagram Protocol) and offers several advantages over traditional protocols like TCP (Transmission Control Protocol).

    QUIC is designed to address the limitations of TCP, such as the high latency introduced by the TCP handshake and the head-of-line blocking problem.
    It achieves this by incorporating features like connection establishment and encryption directly into the protocol, reducing the number of round trips required for establishing a connection.

    QUIC also supports multiplexing, allowing multiple streams of data to be sent over a single connection. This enables concurrent communication and reduces the impact of network latency.
    In addition, QUIC includes built-in congestion control and error correction mechanisms, further enhancing its reliability and performance.

    In the context of libp2p, the term "quic_transport" refers to the transport implementation that utilizes the QUIC protocol for communication between libp2p nodes.
    It enables libp2p applications to benefit from the advantages of QUIC, such as improved latency, multiplexing, and security.

    By using the quic_transport in libp2p, applications can establish secure and efficient communication channels over the Internet, making it suitable for various peer-to-peer and decentralized applications.

    */
    let quic_transport = quic::tokio::Transport::new(quic::Config::new(&identity_keys));

    let transport = OrTransport::new(quic_transport, tcp_transport)
        .map(|either_output, _| match either_output {
            Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            Either::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        })
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
        match swarm.next().await {
            Some(event) => match event {
                SwarmEvent::NewListenAddr {
                    listener_id,
                    address,
                } => {
                    println!("{listener_id:?} = adress = {address:?}");
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour, Default)]
struct Behaviour {
    keep_alive: keep_alive::Behaviour,
    ping: ping::Behaviour,
}
