use std::hash::Hasher;
use std::io::Cursor;
use std::result;

use blake2_rfc::blake2b::{blake2b, Blake2b, Blake2bResult};
use byteorder::{BigEndian, ReadBytesExt};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};
use rand::Rng;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256, Sha512};

pub struct Blake2bHasher {
    context: Blake2b,
}

impl Blake2bHasher {
    pub fn new() -> Self {
        Self {
            context: Blake2b::new(64),
        }
    }
}

impl Hasher for Blake2bHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.context.update(bytes);
    }

    fn finish(&self) -> u64 {
        let context_clone = self.context.clone();
        let result = context_clone.finalize();

        let mut cursor = Cursor::new(result.as_bytes());
        cursor.read_u64::<BigEndian>().unwrap()
    }
}

pub fn generate_keypair() -> Keypair {
  let mut cspring: OsRng = OsRng::new().unwrap();

  Keypair::generate::<Sha512, _>(&mut cspring)
}

pub fn sign_data(
  public_key: &PublicKey,
  secret_key: &SecretKey,
  data: &[u8],
) -> Signature {
  secret_key.expand::<Sha512>().sign::<Sha512>(data, public_key)
}

pub fn verify_data(
  public_key: &PublicKey,
  data: &[u8],
  signature: &Signature,
) -> result::Result<(), ()> {
    if public_key.verify::<Sha512>(data, signature).is_ok() {
        Ok(())
    } else {
        Err(())
    }
}

pub fn generate_random_token() -> String {
    let rnd = format!("{:?}", rand::thread_rng().gen::<f64>());

    base64::encode(&Sha256::digest(rnd.as_bytes()))
}

pub fn generate_discovery_key(public_key: &[u8], name: &[u8]) -> Blake2bResult {
    blake2b(32, public_key, name)
}

#[test]
fn can_verify_signed_data() {
    let keypair = generate_keypair();
    let data = b"Hello, Test!";
    let signature = sign_data(&keypair.public, &keypair.secret, data);

    verify_data(&keypair.public, data, &signature).unwrap();
    verify_data(&keypair.public, b"Wrong Payload", &signature).unwrap_err();
}
