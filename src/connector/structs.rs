use chrono::NaiveDateTime;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub session_id: Uuid,
}

#[derive(Deserialize, Debug)]
pub struct DevicesListResponse(pub Vec<String>);

#[derive(Deserialize, Debug)]
pub enum UploadFileResponse {
    Success(UploadedFile),
    HashMismatch(String),
    BadRequest(String),
}

#[derive(Deserialize, Debug)]
pub struct UploadedFile {
    pub id: u64,
    pub device_id: String,
    pub original_name: String,
    pub versions: Vec<FileVersion>,
}

#[derive(Deserialize, Debug)]
pub struct FileVersion {
    pub version: u64,
    pub size: u64,
    pub hash: String,
    pub created: NaiveDateTime,
    pub mtime: NaiveDateTime,
    pub storage_name: String,
}
