use base58::{FromBase58, ToBase58};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, KeyInit, Nonce};
use rocket::form::validate::Len;
use uuid::Uuid;

#[derive(Debug)]
pub enum EncDecError {
    InvalidFormat,
}
impl std::fmt::Display for EncDecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncDecError::InvalidFormat => write!(f, "InvalidFormat"),
        }
    }
}

#[derive(Debug)]
pub struct EncDec {
    // 随机值，固定12字符，不参与正文加解密
    nonce: [u8; 12],
    // 预留标记1，固定1字节
    s1: u8,
    // 预留标记2，固定1字节
    s2: u8,
    // 需要加密的内容长度
    content_len: usize,
    // 需要加密的内容
    pub content: String,
}

impl EncDec {
    pub fn new<P: Into<String>>(content: P) -> Self {
        let mut nonce = Self::generate_nonce();
        let content = content.into();
        Self {
            nonce,
            s1: 0,
            s2: 0,
            content_len: content.len(),
            content,
        }
    }
    #[inline]
    fn generate_nonce() -> [u8; 12] {
        let uuid = Uuid::new_v4();
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&uuid.as_bytes()[..12]);
        nonce
    }

    /// 加密
    pub fn encrypt(&self, key: &[u8; 32]) -> anyhow::Result<String> {
        #[allow(deprecated)]
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = ChaCha20Poly1305::new(key);
        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&self.nonce);
        let mut data = Vec::new();
        data.extend_from_slice(&[self.s1, self.s2]);
        data.extend_from_slice(&self.content_len.to_be_bytes());
        data.extend_from_slice(self.content.as_bytes());

        //data.extend_from_slice(&self.nonce);
        let ciphertext = cipher
            .encrypt(nonce, data.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        // 密文：nonce + ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&self.nonce);
        result.extend_from_slice(&ciphertext);
        Ok(result.to_base58())
    }

    /// 解密
    pub fn decrypt(key: &[u8; 32], ciphertext: &str) -> Result<EncDec, EncDecError> {
        #[allow(deprecated)]
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = ChaCha20Poly1305::new(key);

        // 截取密文部分
        // let ciphertext = ciphertext
        //     .strip_prefix("DEC(")
        //     .ok_or(EncDecError::InvalidFormat)?
        //     .strip_suffix(")")
        //     .ok_or(EncDecError::InvalidFormat)?;

        // 解码base58，得到12字节nonce+密文字节
        let nonce_ciphertext = ciphertext
            .from_base58()
            .map_err(|_| EncDecError::InvalidFormat)?;

        let nonce = &nonce_ciphertext[..12];
        let ciphertext = &nonce_ciphertext[12..];

        // 解密
        #[allow(deprecated)]
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext.as_ref())
            .map_err(|_| EncDecError::InvalidFormat)?;

        let s1: u8 = plaintext[0];
        let s2: u8 = plaintext[1];
        let principal_len_bytes: [u8; 8] = plaintext[2..10]
            .try_into()
            .map_err(|_| EncDecError::InvalidFormat)?;
        let principal_len = usize::from_be_bytes(principal_len_bytes);
        let principal = String::from_utf8(plaintext[10..10 + principal_len].to_vec()).unwrap();

        let nonce: [u8; 12] = nonce.try_into().map_err(|_| EncDecError::InvalidFormat)?;

        Ok(EncDec {
            s1,
            s2,
            nonce,
            content_len: principal_len,
            content: principal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_enc_dec() {
        let key = &[0; 32];
        let enc_dec = EncDec::new("1234567890");
        println!("{:?}", enc_dec);

        let enc_dec = enc_dec.encrypt(key).unwrap();
        println!("{:?}", enc_dec);
        let enc_dec = EncDec::decrypt(key, &enc_dec);
        println!("{:?}", enc_dec);
    }
}
