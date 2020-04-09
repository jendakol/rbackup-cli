use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub session_id: Uuid,
}
