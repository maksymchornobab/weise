use serde::{Serialize, Deserialize};
use std::error::Error;
use crate::WeiseTransport; 


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarlicClove {

    pub next_hop: String, 

    pub encrypted_payload: Vec<u8>,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GarlicPacket {

    pub cloves: Vec<GarlicClove>,

    pub padding: Vec<u8>,
}

impl GarlicPacket {

    pub fn build(cloves: Vec<GarlicClove>, target_size: usize) -> Self {
        let mut packet = GarlicPacket {
            cloves,
            padding: Vec::new(),
        };
        


        let serialized_size = serde_json::to_vec(&packet).unwrap().len();
        if serialized_size < target_size {
            let needed_padding = target_size - serialized_size;
            packet.padding = vec![0u8; needed_padding];
        }
        
        packet
    }
}

impl WeiseTransport {

    pub fn send_garlic(&mut self, receiver_pubkey: &str, data: &str) -> Result<(), Box<dyn Error>> {

        let clove = GarlicClove {
            next_hop: receiver_pubkey.to_string(),
            encrypted_payload: data.as_bytes().to_vec(),
        };


        let cloves = vec![clove]; 
        let garlic_packet = GarlicPacket::build(cloves, 4080);


        let garlic_json = serde_json::to_string(&garlic_packet)?;


        let final_payload = format!("GARLIC:{}", garlic_json);

        self.send_secure(&final_payload)?;

        Ok(())
    }
}