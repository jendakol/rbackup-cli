use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerSession {
    #[serde(with = "url_serde")]
    pub url: Url,
    pub session_id: Uuid,
}
