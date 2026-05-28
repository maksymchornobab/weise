
use libp2p::{
    futures::StreamExt,
    identity, noise, tcp, yamux,
    swarm::{SwarmEvent, NetworkBehaviour, Swarm},
    SwarmBuilder, Multiaddr, gossipsub,
};
use std::error::Error;
use std::time::Duration;
use rand::Rng;

const PACKET_SIZE: usize = 4096;

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub ping: libp2p::ping::Behaviour,
}

pub struct WeiseTransport {
    pub swarm: Swarm<MyBehaviour>,
    pub topic: gossipsub::IdentTopic,
    pub local_peer_id: libp2p::PeerId,
}

// 1. Кажемо компілятору змонтувати файл garlic.rs як модуль
pub mod garlic;

// 2. Робимо структури з нього доступними ззовні (re-export)
pub use garlic::{GarlicPacket, GarlicClove};

impl WeiseTransport {
    /// Ініціалізує захищений стелс-транспорт
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let id_keys = identity::Keypair::generate_ed25519();
        let local_peer_id = identity::PeerId::from_public_key(&id_keys.public());

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) 
            .validation_mode(gossipsub::ValidationMode::Strict) 
            .build()?;

        let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(|key| {
                let message_authenticity = gossipsub::MessageAuthenticity::Signed(key.clone());
                let gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config).unwrap();
                MyBehaviour { gossipsub, ping: libp2p::ping::Behaviour::default() }
            })?
            .build();

        let topic = gossipsub::IdentTopic::new("weise-l1-chat");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        Ok(Self { swarm, topic, local_peer_id })
    }

    /// Публічний метод для відправки повідомлень з авто-пакуванням у стелс-пакет
    pub fn send_secure(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        let mut payload = text.as_bytes().to_vec();
        if payload.len() > PACKET_SIZE - 2 {
            payload.truncate(PACKET_SIZE - 2);
        }
        
        let len = payload.len() as u16;
        let mut packet = len.to_be_bytes().to_vec();
        packet.extend(payload);
        
        let mut rng = rand::thread_rng();
        let current_len = packet.len();
        if current_len < PACKET_SIZE {
            let padding: Vec<u8> = (0..(PACKET_SIZE - current_len)).map(|_| rng.r#gen::<u8>()).collect();
            packet.extend(padding);
        }

        self.swarm.behaviour_mut().gossipsub.publish(self.topic.clone(), packet)?;
        Ok(())
    }

    /// Публічний метод для генерації фонового шуму (Chaffing)
    pub fn send_noise(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_secure("")
    }

    /// Допоміжний метод для парсингу сирих байтів
    pub fn unpack(data: &[u8]) -> Option<String> {
        if data.len() < 2 { return None; }
        let len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if len + 2 > data.len() { return None; }
        String::from_utf8(data[2..len+2].to_vec()).ok()
    }
}