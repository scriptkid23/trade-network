use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::OrTransport;
use libp2p::futures::future::Either;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, ConfigBuilder, Message, MessageId, ValidationMode};
use libp2p::{
    identity, noise, ping,
    swarm::{keep_alive, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId,
};
use libp2p::{mdns, Transport};
use libp2p_quic as quic;
use std::any::type_name;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::{io, select};

fn message_id_fn(message: &Message) -> MessageId {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    return MessageId::from(s.finish().to_string());
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

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

    let gossipsub_config = ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
        .build()
        .expect("Valid config");

    // Create gossip behavior
    let mut gossipsub: gossipsub::Behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(identity_keys),
        gossipsub_config,
    )
    .expect("Correct configuration");

    let topic = gossipsub::IdentTopic::new("test-net");

    gossipsub.subscribe(&topic)?;

    let mut swarm = {
        let mdns = libp2p::mdns::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        let behaviour = MyBehaviour { gossipsub, mdns };
        SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
    };

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin());

    // Tell the swarm to listen on all interfaces and a random, OS-assigned
    // port.
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");

    // Dial the peer identified by the multi-address given as the second
    // command-line argument, if any.
    loop {
        let mut buffer = String::new();
        let read_stdin_task = async {
            stdin.read_line(&mut buffer).await?;
            /*
            "cannot infer type for type parameter E declared on the enum Result,"
            typically occurs when the Rust compiler is unable to infer the type of a Result variant within an async block.
             */
            Ok::<_, io::Error>(())
        };
        select! {
            _  = read_stdin_task => {
                println!("Buffer: {buffer}");

                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), buffer.as_bytes()) {
                    println!("Publish error: {e:?}");
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => println!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    ),
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
}

#[derive(NetworkBehaviour, Default)]
struct Behaviour {
    keep_alive: keep_alive::Behaviour,
    ping: ping::Behaviour,
}
