use serde::{Serialize, Deserialize};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, KeyInit};
use rand::RngCore;
use rand::rngs::OsRng;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GarlicPayload {
    /// Реальні корисні дані для блокчейну (наприклад, JSON транзакції)
    BlockchainData(String),
    /// Інструкція для проміжної ноди (куди переслати далі)
    ForwardTo {
        next_peer_id: String,
        encrypted_inner_layer: Vec<u8>,
    },
    /// Захисний делівери-шум
    DecoyNoise(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarlicPacket {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl GarlicPacket {
    /// Запакувати дані в один шар шифрування для конкретного Peer
    pub fn pack_layer(payload: GarlicPayload, shared_secret_key: &[u8; 32]) -> Vec<u8> {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let chacha_nonce = Nonce::from_slice(&nonce);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(shared_secret_key));
        let raw_bytes = serde_json::to_vec(&payload).unwrap();

        let ciphertext = cipher.encrypt(chacha_nonce, raw_bytes.as_ref())
            .expect("Помилка шифрування Garlic-шару");

        let packet = GarlicPacket {
            nonce,
            ciphertext,
        };

        // Повертаємо серіалізований пакет (байтовий вектор)
        serde_json::to_vec(&packet).unwrap()
    }

    /// Спробувати зняти поточний шар шифрування своїм ключем
    pub fn unpack_layer(packet_bytes: &[u8], my_shared_secret_key: &[u8; 32]) -> Result<GarlicPayload, String> {
        let packet: GarlicPacket = serde_json::from_slice(packet_bytes)
            .map_err(|_| "Некоректний формат пакета".to_string())?;

        let cipher = ChaCha20Poly1305::new(Key::from_slice(my_shared_secret_key));
        let chacha_nonce = Nonce::from_slice(&packet.nonce);

        match cipher.decrypt(chacha_nonce, packet.ciphertext.as_ref()) {
            Ok(decrypted_bytes) => {
                let payload: GarlicPayload = serde_json::from_slice(&decrypted_bytes).unwrap();
                Ok(payload)
            },
            Err(_) => Err("Пакет зашифровано іншим ключем (рухаємось далі)".to_string())
        }
    }
}