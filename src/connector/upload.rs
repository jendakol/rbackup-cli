use err_context::AnyError;
use log::{debug, error};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub struct UploadedFile {
    path: PathBuf,
    metadata: Metadata,
}

impl UploadedFile {
    pub fn open(path: PathBuf) -> Result<Self, AnyError> {
        let path = std::fs::canonicalize(path)?;

        let metadata = std::fs::metadata(&path)?;

        let _ = metadata.modified()?; // just to check it's readable

        Ok(UploadedFile { path, metadata })
    }

    pub fn as_query(&self) -> Result<Vec<(String, String)>, AnyError> {
        let mtime = self
            .metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();

        Ok(vec![
            (
                "file_path".to_string(),
                self.path.to_str().unwrap().to_string(),
            ),
            ("size".to_string(), format!("{}", self.metadata.len())),
            ("mtime".to_string(), format!("{}", mtime)),
        ])
    }

    pub async fn into_multipart_form(self) -> Result<Form, AnyError> {
        let hasher = Arc::new(Mutex::new(sha2::Sha256::new()));

        let body: Part = self.get_body(Arc::clone(&hasher)).await?;
        let hash = self.get_hash_part(hasher)?;

        Ok(Form::new().part("file", body).part("file-hash", hash))
    }

    async fn get_body(&self, hasher: Arc<Mutex<Sha256>>) -> Result<Part, AnyError> {
        use tokio::fs::File as TokioFile;
        use tokio::io::AsyncReadExt;

        let stream =
            futures::stream::unfold(TokioFile::open(&self.path).await?, move |mut file| {
                let hasher = Arc::clone(&hasher);

                async move {
                    let mut buff = Vec::with_capacity(4096);

                    if file.read(&mut buff).await.unwrap() > 0 {
                        let hasher = &mut *hasher.lock().expect("Poisoned mutex!");
                        debug!("Streaming chunk of {} bytes of", buff.len());
                        sha2::digest::Input::input(hasher, &buff);
                        Some((Ok::<_, AnyError>(buff), file))
                    } else {
                        None
                    }
                }
            });

        Ok(Part::stream(Body::wrap_stream(stream)))
    }

    fn get_hash_part(&self, hasher: Arc<Mutex<Sha256>>) -> Result<Part, AnyError> {
        use HashingPartState::*;

        let stream = futures::stream::unfold(Init(hasher), move |state| async move {
            match state {
                Init(arc) => match UploadedFile::unwrap_hasher(arc) {
                    Ok(hasher) => Some((Ok::<_, AnyError>(String::new()), Hashing(hasher))),
                    Err(e) => {
                        error!("Could not unwrap hasher: {:?}", e);
                        Some((Err(e), Closed)) // Closed here won't be ever read, because we return Err
                    }
                },
                Hashing(hasher) => {
                    let hash = hex::encode(hasher.result());
                    debug!("Calculated hash {}", hash);
                    Some((Ok::<_, AnyError>(hash), Closed))
                }
                Closed => None,
            }
        });

        Ok(Part::stream(Body::wrap_stream(stream)))
    }

    fn unwrap_hasher(arc: Arc<Mutex<Sha256>>) -> Result<Sha256, AnyError> {
        Arc::try_unwrap(arc)
            .map_err(|_| AnyError::from("Could not unwrap Arc!"))
            .and_then(|m| {
                m.into_inner()
                    .map_err(|_| AnyError::from("Could not unwrap Mutex!"))
            })
    }
}

enum HashingPartState {
    Init(Arc<Mutex<Sha256>>),
    Hashing(Sha256),
    Closed,
}
