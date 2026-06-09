use libp2p::{
    futures::StreamExt,
    identity, noise, tcp, yamux,
    swarm::{SwarmEvent, NetworkBehaviour, Swarm},
    SwarmBuilder, Multiaddr, gossipsub,
};

use libp2p::kad::{Behaviour as KadBehaviour, store::MemoryStore};
use libp2p::autonat::Behaviour as AutoNatBehaviour;
use libp2p::dcutr::Behaviour as DcutrBehaviour;
use std::error::Error;
use std::time::Duration;
use rand::Rng;

const PACKET_SIZE: usize = 8192;

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub ping: libp2p::ping::Behaviour,
    pub kademlia: KadBehaviour<MemoryStore>,
    pub autonat: AutoNatBehaviour,
    pub dcutr: DcutrBehaviour,
    pub mdns: libp2p::mdns::tokio::Behaviour,
}

pub struct WeiseTransport {
    pub swarm: Swarm<MyBehaviour>,
    pub topic: gossipsub::IdentTopic,
    pub local_peer_id: libp2p::PeerId,
}

pub mod garlic;

pub use garlic::{GarlicPacket, GarlicClove};

impl WeiseTransport {

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let id_keys = identity::Keypair::generate_ed25519();
        let local_peer_id = identity::PeerId::from_public_key(&id_keys.public());

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) 
            .validation_mode(gossipsub::ValidationMode::Strict) 
            .build()?;

        let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
            .with_tokio()
            // 🔄 ЗМІНА: Додаємо підтримку QUIC (UDP) паралельно з TCP для пробиття NAT
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_quic() 
            .with_behaviour(|key| {
                let message_authenticity = gossipsub::MessageAuthenticity::Signed(key.clone());
                let gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config).unwrap();
                
                // 🧠 Ініціалізуємо Kademlia DHT зі сховищем у RAM пам'яті
                let peer_id = key.public().to_peer_id();
                let store = MemoryStore::new(peer_id);
                let kademlia = KadBehaviour::new(peer_id, store);

                // 📡 Ініціалізуємо AutoNAT (визначення типу NAT за роутером)
                let autonat = AutoNatBehaviour::new(peer_id, libp2p::autonat::Config::default());

                // ⚡ Ініціалізуємо DCUtR (рушій автоматичного пробиття дірок у роутері)
                let dcutr = DcutrBehaviour::new(peer_id);

                // 🌐 Ініціалізуємо mDNS для локального пошуку Mac/Windows в одній мережі
                let mdns = libp2p::mdns::tokio::Behaviour::new(
                    libp2p::mdns::Config::default(), 
                    peer_id
                ).unwrap();

                // ✨ Повертаємо повністю укомплектовану структуру без пропущених полів
                MyBehaviour { 
                    gossipsub, 
                    ping: libp2p::ping::Behaviour::default(),
                    kademlia,
                    autonat,
                    dcutr,
                    mdns,
                }
            })?
            .build();

        let topic = gossipsub::IdentTopic::new("weise-l1-chat");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        
        // Видаляємо звідси фіксований listen_on, бо тепер ми динамічно керуємо 
        // портами TCP та UDP (QUIC) прямо у твоєму файлі main.rs

        Ok(Self { swarm, topic, local_peer_id })
    }

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

    pub fn send_noise(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_secure("")
    }

    pub fn unpack(data: &[u8]) -> Option<String> {
        if data.len() < 2 { return None; }
        let len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if len + 2 > data.len() { return None; }
        String::from_utf8(data[2..len+2].to_vec()).ok()
    }
}