use serde::{Serialize, Deserialize};
use std::error::Error;
use crate::WeiseTransport; 

/// Окремий зубчик часнику (конкретне повідомлення всередині пакету)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarlicClove {
    /// Ключ шифрування або інструкція для наступного вузла (Routing Directive)
    pub next_hop: String, 
    /// Зашифроване корисне навантаження (сама транзакція або наступний вкладений зубчик)
    pub encrypted_payload: Vec<u8>,
}

/// Повна головка часнику (пакет константного розміру, який летить в мережу)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarlicPacket {
    /// Список зубчиків (твоя транзакція + транзитні пакети + фейковий шум)
    pub cloves: Vec<GarlicClove>,
    /// Випадкова сіль (padding), щоб вирівняти розмір пакета до константи
    pub padding: Vec<u8>,
}

impl GarlicPacket {
    /// Функція збору зубчиків в один пакет
    pub fn build(cloves: Vec<GarlicClove>, target_size: usize) -> Self {
        let mut packet = GarlicPacket {
            cloves,
            padding: Vec::new(),
        };
        
        // Розраховуємо, скільки фейкових байтів треба докинути, 
        // щоб пакет ЗАВЖДИ мав однаковий розмір у Wireshark
        let serialized_size = serde_json::to_vec(&packet).unwrap().len();
        if serialized_size < target_size {
            let needed_padding = target_size - serialized_size;
            packet.padding = vec![0u8; needed_padding]; // Забиваємо нулями
        }
        
        packet
    }
}

impl WeiseTransport {
    /// Головний метод для zelle: приймає отримувача та сирі дані (наприклад, JSON транзакції)
    pub fn send_garlic(&mut self, receiver_pubkey: &str, data: &str) -> Result<(), Box<dyn Error>> {
        // 1. Створюємо окремий зубчик (Clove)
        let clove = GarlicClove {
            next_hop: receiver_pubkey.to_string(),
            encrypted_payload: data.as_bytes().to_vec(),
        };

        // 2. Збираємо зубчики в одну велику головку часнику (пакет на 4096 байт)
        let cloves = vec![clove]; 
        let garlic_packet = GarlicPacket::build(cloves, 4096);

        // 3. Серіалізуємо часниковий пакет в JSON-рядок
        let garlic_json = serde_json::to_string(&garlic_packet)?;

        // 4. Додаємо префікс, щоб нода-отримувач знала, що це часниковий пакет
        let final_payload = format!("GARLIC:{}", garlic_json);

        // 5. Вистрілюємо в мережу через твій Noise + Gossipsub транспорт
        self.send_secure(&final_payload)?;

        Ok(())
    }
}