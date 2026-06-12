use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use std::error::Error;
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt};
use libp2p::futures::StreamExt;

use weise::{WeiseTransport, MyBehaviourEvent}; 

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();

    let mut transport = WeiseTransport::new()?;
    println!("--- WEISE SDK NODE STARTED ---");
    println!("Мій ID: {}", transport.local_peer_id);

    // Слухаємо динамічні порти (0 означає, що ОС сама виділить вільний порт)
    transport.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    println!("[📡 TCP ONLINE] Відкрито TCP порт для вхідних з'єднань");

    transport.swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    println!("[⚡ QUIC ONLINE] Відкрито UDP порт для Hole Punching (QUIC)");

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut noise_timer = tokio::time::interval(Duration::from_secs(10));

    // 🔥 ПІДКЛЮЧЕННЯ ДО ГЛОБАЛЬНОЇ ТОЧКИ ЗУСТРІЧІ (BOOTSTRAP)
    let bootstrap_str = "/ip4/147.75.109.213/udp/4001/quic-v1/p2p/QmNnooDu7bfj99oddSg1Z1Yu1v5gREeXgW36RUpw3eaYXY";
    if let Ok(bootstrap_addr) = bootstrap_str.parse::<libp2p::Multiaddr>() {
        println!("[🌐 GLOBAL] Стукаємо в глобальний шлюз для обходу NAT...");
        if let Some(libp2p::multiaddr::Protocol::P2p(bootstrap_peer_id)) = bootstrap_addr.iter().last() {
            // Додаємо в Kademlia, щоб ноди знали, де шукати одна одну
            transport.swarm.behaviour_mut().kademlia.add_address(&bootstrap_peer_id, bootstrap_addr.clone());
            let _ = transport.swarm.dial(bootstrap_addr);
        }
    }

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                if let Ok(Some(text)) = line {
                    let text = text.trim();
                    if text.is_empty() { continue; }

                    if text.starts_with('/') {
                        if let Ok(remote_addr) = text.parse::<Multiaddr>() {
                            let _ = transport.swarm.dial(remote_addr);
                        }
                    } else {
                        let _ = transport.send_secure(text);
                    }
                }
            }

            _ = noise_timer.tick() => {
                let _ = transport.send_noise();
            }

            event = transport.swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Твоя адреса: {}", address);
                }
                
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    println!("[🤝 CONNECTED] Підключено до піра: {}", peer_id);
                    
                    // 🔥 НАПОВНЕННЯ KADEMLIA DHT
                    // Отримуємо від endpoint фізичну адресу, з якою було встановлено зв'язок
                    let remote_addr = endpoint.get_remote_address().clone();
                    
                    // Додаємо пару (PeerId, Multiaddr) в таблицю маршрутизації Kademlia
                    transport.swarm.behaviour_mut().kademlia.add_address(&peer_id, remote_addr.clone());
                    println!("[🌌 KADEMLIA] Ноду {} успішно інтегровано в таблицю DHT за адресою: {}", peer_id, remote_addr);
                }

                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(libp2p::gossipsub::Event::Message { propagation_source, message, .. })) => {
                    if let Some(decoded_text) = WeiseTransport::unpack(&message.data) {
                        if !decoded_text.is_empty() {
                            println!("\n[MSG] {}: {}", propagation_source, decoded_text);
                        }
                    }
                }

                // 🌌 Обробка подій Kademlia DHT без використання конфліктних назв шляхів
                SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(kad_event)) => {
                    if let libp2p::kad::Event::RoutingUpdated { peer, is_new_peer, .. } = kad_event {
                        if is_new_peer {
                            println!("[🌌 KADEMLIA DHT] Знайдено нову глобальну ноду за XOR-метрикою: {}", peer);
                        }
                    }
                }

                SwarmEvent::Behaviour(MyBehaviourEvent::Dcutr(dcutr_event)) => {
                    let event_debug = format!("{:?}", dcutr_event);
                    
                    if event_debug.contains("RemoteInitiatedDirectConnectionUpgrade") {
                        if event_debug.contains("Ok") {
                            println!("[⚡ HOLE PUNCHING SUCCESS] Стіну роутера успішно пробито! Прямий тунель встановлено.");
                        } else if event_debug.contains("Err") {
                            println!("[⚠️ HOLE PUNCHING FAIL] Не вдалося автоматично пробити NAT для вхідного з'єднання.");
                        }
                    }
                }
                
                _ => {}
            }
        }
    }
}