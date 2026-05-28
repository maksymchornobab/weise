// Імпортуємо твій головний транспорт (зміни шлях, якщо структура WeiseTransport лежить в іншому модулі weise)
use crate::WeiseTransport; 
use std::error::Error;

impl WeiseTransport {
    /// Головний метод для zelle: приймає отримувача та сирі дані (наприклад, JSON транзакції)
    pub fn send_garlic(&mut self, receiver_pubkey: &str, data: &str) -> Result<(), Box<dyn Error>> {
        // 1. Створюємо окремий зубчик (Clove)
        // У майбутньому тут encrypted_payload буде шифруватися публічним ключем receiver_pubkey
        let clove = GarlicClove {
            next_hop: receiver_pubkey.to_string(),
            encrypted_payload: data.as_bytes().to_vec(),
        };

        // 2. Збираємо зубчики в одну велику головку часнику (пакет на 4096 байт)
        let cloves = vec![clove]; // Поки що один зубчик, пізніше сюди додамо транзитні
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