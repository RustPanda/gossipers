use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex, noise,
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    Multiaddr, NetworkBehaviour, PeerId, Transport,
};
use log::*;
use std::{error::Error, net::SocketAddrV4, time::Duration};
use structopt::StructOpt;
use tokio::time;

#[derive(Debug, StructOpt)]
#[structopt(name = "gossipers", about = "Дорогие сплетники...")]
struct Opt {
    #[structopt(long = "period", default_value = "5.")]
    period: f32,
    #[structopt(long = "port", default_value = "8080")]
    port: u16,
    #[structopt(long = "connect")]
    connect: Option<SocketAddrV4>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = Opt::from_args();

    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    info!("Peer Id: {}", &peer_id);

    let topic = Topic::new("chat");

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mdns = Mdns::new(Default::default()).await?;
    let mut behaviour = Behaviour {
        floodsub: Floodsub::new(peer_id.clone()),
        mdns,
    };

    behaviour.floodsub.subscribe(topic.clone());

    let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    swarm.listen_on(
        format!("/ip4/127.0.0.1/tcp/{}", opt.port)
            .parse()
            .expect("can get a local socket"),
    )?;

    if let Some(add) = opt.connect {
        let multiaddr = format!("/ip4/{}/tcp?{}", add.ip(), add.port()).parse()?;
        swarm.dial_addr(multiaddr)?;
    }

    let mut interval = time::interval(Duration::from_secs_f32(opt.period));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                swarm.behaviour_mut().floodsub.publish(topic.clone(), "MikuMikuDance");
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("Listening on {:?}", address);
                }
            }
        }
    }
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    floodsub: Floodsub,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            println!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}
