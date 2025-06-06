use arrayref::array_ref;
use base64::{engine::general_purpose, Engine as _};
use log::{debug, error};
use protocol::{
    constants::AES256_NONCE_LENGTH,
    utils::{AssociatedData, DecryptionKey},
};
use serde_json::{json, Value};
use std::fmt::Display;
use serde::{Serialize, Deserialize};
use std::fs;
use std::sync::LazyLock;

pub fn decrypt_request(req: &str, dk: &DecryptionKey) -> Result<(Value, AssociatedData), ()> {
    let enc_req = match general_purpose::STANDARD.decode(req.to_string()) {
        Ok(s) => s,
        Err(_e) => {
            error!("Failed to decode request");
            return Err(());
        }
    };
    let nonce = *array_ref!(enc_req, 0, AES256_NONCE_LENGTH);
    let aad = match AssociatedData::try_from(array_ref!(
        enc_req,
        AES256_NONCE_LENGTH,
        AssociatedData::SIZE
    )) {
        Ok(aad) => aad,
        Err(_) => return Err(()),
    };
    let offset = AES256_NONCE_LENGTH + AssociatedData::SIZE;
    let end = enc_req.len();
    let cipher_text = &enc_req[offset..end];
    let text = match dk.decrypt(cipher_text, &nonce, &aad.clone().to_bytes()) {
        Ok(dec) => dec,
        Err(_) => return Err(()),
    };

    debug!(
        "Decrypted request: {}",
        String::from_utf8(text.clone()).unwrap()
    );
    match String::from_utf8(text) {
        Ok(s) => Ok((
            serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            aad,
        )),
        Err(e) => {
            error!("Failed to parse request: {}", e);
            Err(())
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RequestWrapper {
    pub request_id: String,
    pub body: Value,
}


/// Server -> Client
#[derive(Serialize, Deserialize)]
pub struct ResponseWrapper {
    pub request_id: String,
    pub body: Value,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseCode {
    Ok,
    BadRequest,
    NotFound,
    InternalServerError,
    Conflict,
}

impl Display for ResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseCode::Ok => write!(f, "200"),
            ResponseCode::BadRequest => write!(f, "400"),
            ResponseCode::NotFound => write!(f, "404"),
            ResponseCode::InternalServerError => write!(f, "500"),
            ResponseCode::Conflict => write!(f, "409"),
        }
    }
}


impl TryFrom<&str> for ResponseCode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, ()> {
        match value {
            "200" => Ok(Self::Ok),
            "400" => Ok(Self::BadRequest),
            "404" => Ok(Self::NotFound),
            "500" => Ok(Self::InternalServerError),
            "409" => Ok(Self::Conflict),
            _ => Err(()),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
    pub code: ResponseCode,
    pub text: String,
}

impl ServerResponse {
    pub fn new(code: ResponseCode, text: String) -> Self {
        Self { code, text }
    }

    pub fn from_json(value: String) -> Option<Self>{
        let value = serde_json::from_str::<Value>(&value).ok()?;
        let code = value.get("code")?.as_str()?;
        let text = value.get("message")?.as_str()?;
        let code = ResponseCode::try_from(code).ok()?;
        Some(Self::new(code, text.to_string()))
    }
}


impl Display for ServerResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = json!({
            "code": self.code.to_string(),
            "message": self.text
        })
        .to_string();

        write!(f, "{}", res)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub bundle: String,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub msg_type: String,
    pub from: String,
    pub to: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetPreKeyBundleRequest {
    pub who: String,
}

#[derive(Clone, Deserialize)]
pub struct Config {
    server_ip: String,
    server_port: String,
    private_key_server: String,
    public_key_server: String,
    log_level: String,

    #[serde(skip_deserializing)]
    server_url: String,
}
impl Config {
    fn new(filename: &str) -> Self {
        let config = fs::read_to_string(filename).expect("Unable to read the configuration file");
        let mut config: Config = toml::from_str(&config).expect("Unable to parse the configuration file");

        config.server_url = format!("ws://{}:{}", config.server_ip, config.server_port);

        config
    }

    pub fn get_server_ip(&self) -> String {
        self.server_ip.clone()
    }

    pub fn get_server_port(&self) -> String {
        self.server_port.clone()
    }

    pub fn get_private_key_server(&self) -> String {
        self.private_key_server.clone()
    }

    pub fn get_public_key_server(&self) -> String {
        self.public_key_server.clone()
    }

    pub fn get_log_level(&self) -> String {
        self.log_level.clone()
    }

    pub fn get_server_url(&self) -> String {
        self.server_url.clone()
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::new("./config/config.toml"));