use chrono::{DateTime, Utc};
use jsonwebtoken::DecodingKey;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GoogleCertificate {
    pub kids: HashMap<String, DecodingKey<'static>>,
    pub expires: DateTime<Utc>,
}

impl GoogleCertificate {
    pub fn convert_kids(input: HashMap<String, String>) -> HashMap<String, DecodingKey<'static>> {
        let mut output = HashMap::<String, DecodingKey<'static>>::with_capacity(input.len());
        for (key, value) in input {
            let pem = openssl::x509::X509::from_pem(value.as_bytes())
                .expect("Unable to convert KID into PEM")
                .public_key()
                .expect("Unable to convert KID into PEM")
                .rsa()
                .expect("Unable to convert KID into PEM")
                .public_key_to_pem()
                .expect("Unable to convert KID into PEM");
            output.insert(
                key,
                DecodingKey::from_rsa_pem(&pem)
                    .expect("Unable to get decoding key")
                    .into_static(),
            );
        }
        output
    }
}
