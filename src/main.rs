use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use std::error::Error;
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt};
use libp2p::futures::StreamExt;

// Імпортуємо модуль Weise, який ми щойно створили в lib.rs
use weise::{WeiseTransport, MyBehaviourEvent}; 

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().init();

    // ВСЯ ІНІЦІАЛІЗАЦІЯ В ОДИН РЯДОК!
    let mut transport = WeiseTransport::new()?;
    println!("--- WEISE SDK NODE STARTED ---");
    println!("Мій ID: {}", transport.local_peer_id);

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut noise_timer = tokio::time::interval(Duration::from_secs(20));

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                if let Ok(Some(text)) = line {
                    let text = text.trim();
                    if text.is_empty() { continue; }

                    if text.starts_with("/ip4") {
                        if let Ok(remote_addr) = text.parse::<Multiaddr>() {
                            let _ = transport.swarm.dial(remote_addr);
                        }
                    } else {
                        // Просто викликаємо метод нашого SDK
                        let _ = transport.send_secure(text);
                    }
                }
            }

            _ = noise_timer.tick() => {
                // Фоновий стелс-шум однією командою
                let _ = transport.send_noise();
            }

            event = transport.swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Твоя адреса: {}", address);
                }
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(libp2p::gossipsub::Event::Message { propagation_source, message, .. })) => {
                    // Використовуємо наш вбудований розпакувальник
                    if let Some(decoded_text) = WeiseTransport::unpack(&message.data) {
                        if !decoded_text.is_empty() {
                            println!("\n[MSG] {}: {}", propagation_source, decoded_text);
                        }
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("Підключено до піра: {}", peer_id);
                }
                _ => {}
            }
        }
    }
}