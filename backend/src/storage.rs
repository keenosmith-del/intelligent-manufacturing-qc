use crate::{config::Config, firestore::access_token, models::valid_id};
use anyhow::{bail, Result};
use reqwest::Client;
use std::path::PathBuf;

pub struct Storage {
    root: PathBuf,
    bucket: Option<String>,
    client: Client,
}
impl Storage {
    pub fn new(config: &Config) -> Result<Self> {
        if !["local", "gcs"].contains(&config.storage_provider.as_str()) {
            bail!("unsupported storage provider");
        }
        let bucket = if config.storage_provider == "gcs" {
            Some(
                config
                    .bucket
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("GCS_BUCKET is required"))?,
            )
        } else {
            None
        };
        Ok(Self {
            root: config.local_storage.clone(),
            bucket,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
        })
    }
    fn check(key: &str) -> Result<()> {
        if key.split('/').any(|s| !valid_id(s)) || key.starts_with('/') {
            bail!("invalid storage key");
        }
        Ok(())
    }
    pub async fn put(&self, key: &str, bytes: &[u8], media: &str) -> Result<()> {
        Self::check(key)?;
        if let Some(bucket) = &self.bucket {
            self.client
                .post(format!(
                    "https://storage.googleapis.com/upload/storage/v1/b/{bucket}/o"
                ))
                .query(&[("uploadType", "media"), ("name", key)])
                .bearer_auth(access_token(&self.client).await?)
                .header("Content-Type", media)
                .body(bytes.to_vec())
                .send()
                .await?
                .error_for_status()?;
        } else {
            let path = self.root.join(key);
            tokio::fs::create_dir_all(path.parent().unwrap()).await?;
            let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
            tokio::fs::write(&temp, bytes).await?;
            tokio::fs::rename(temp, path).await?;
        }
        Ok(())
    }
    pub async fn get(&self, key: &str) -> Result<Vec<u8>> {
        Self::check(key)?;
        if let Some(bucket) = &self.bucket {
            let mut url = reqwest::Url::parse(&format!(
                "https://storage.googleapis.com/storage/v1/b/{bucket}/o/"
            ))?;
            url.path_segments_mut().unwrap().pop_if_empty().push(key);
            Ok(self
                .client
                .get(url)
                .query(&[("alt", "media")])
                .bearer_auth(access_token(&self.client).await?)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?
                .to_vec())
        } else {
            Ok(tokio::fs::read(self.root.join(key)).await?)
        }
    }
    pub async fn delete(&self, key: &str) -> Result<()> {
        Self::check(key)?;
        if let Some(bucket) = &self.bucket {
            let mut url = reqwest::Url::parse(&format!(
                "https://storage.googleapis.com/storage/v1/b/{bucket}/o/"
            ))?;
            url.path_segments_mut().unwrap().pop_if_empty().push(key);
            self.client
                .delete(url)
                .bearer_auth(access_token(&self.client).await?)
                .send()
                .await?
                .error_for_status()?;
        } else {
            tokio::fs::remove_file(self.root.join(key)).await?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_rejected() {
        assert!(Storage::check("../secret").is_err());
        assert!(Storage::check("images/a.png").is_ok());
        assert!(Storage::check("/etc/passwd").is_err());
    }
}
