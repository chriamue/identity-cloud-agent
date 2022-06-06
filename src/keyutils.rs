use didcomm_mediator::keybytes::KeyBytes;
use identity::prelude::KeyPair;

pub struct KP(pub KeyPair);

impl KeyBytes for KP {
    fn private_key(&self) -> Vec<u8> {
        self.0.private().as_ref().to_vec()
    }
    fn public_key(&self) -> Vec<u8> {
        self.0.public().as_ref().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base58::FromBase58;
    use did_key::{generate, Ed25519KeyPair, KeyMaterial, X25519KeyPair};
    use didcomm_mediator::keybytes::KeyBytes;
    use identity::prelude::*;

    #[test]
    fn test_private_key() {
        let seed = "HBTcN2MrXNRj9xF9oi8QqYyuEPv3JLLjQKuEgW9oxVKP";

        let private = seed.from_base58().unwrap();

        let keypair = generate::<Ed25519KeyPair>(Some(&private));

        let keypair_ed =
            KP(KeyPair::try_from_private_key_bytes(KeyType::Ed25519, &private).unwrap());

        assert_eq!(keypair.private_key_bytes(), keypair_ed.private_key());
    }

    #[test]
    fn test_public_key() {
        let seed = "HBTcN2MrXNRj9xF9oi8QqYyuEPv3JLLjQKuEgW9oxVKP";

        let private = seed.from_base58().unwrap();

        let keypair = generate::<X25519KeyPair>(Some(&private));
        let keypair_key_exchange =
            KP(KeyPair::try_from_private_key_bytes(KeyType::X25519, &private).unwrap());
        assert_eq!(
            keypair.public_key_bytes(),
            keypair_key_exchange.public_key()
        );
    }
}
