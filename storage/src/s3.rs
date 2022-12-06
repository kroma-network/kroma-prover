use anyhow::{bail, Result};
use log::info;
use std::path::PathBuf;

use aws_sdk_s3::{config, ByteStream, Client, Credentials, Region};

pub struct S3 {
    pub client: Client,
}

impl S3 {
    pub fn new(access_key: String, secret_key: String, region: String) -> Self {
        let cred = Credentials::new(access_key, secret_key, None, None, "loaded-from-custom-env");

        let region = Region::new(region);
        let conf_builder = config::Builder::new()
            .region(region)
            .credentials_provider(cred);
        let conf = conf_builder.build();

        let client = Client::from_conf(conf);

        Self { client }
    }

    pub async fn list_keys(&self, bucket_name: String) -> Result<Vec<String>> {
        let req = self.client.list_objects_v2().prefix("").bucket(bucket_name);

        let res = req.send().await?;

        let keys = res.contents().unwrap_or_default();
        let keys = keys
            .iter()
            .filter_map(|o| o.key.as_ref())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        Ok(keys)
    }

    pub async fn upload(&self, path: PathBuf, bucket_name: String, aws_path: String) -> Result<()> {
        if !path.exists() {
            bail!("Path {} does not exist", path.display());
        }

        let body = ByteStream::from_path(&path).await?;
        let content_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();

        let req = self
            .client
            .put_object()
            .bucket(bucket_name)
            .key(aws_path)
            .body(body)
            .content_type(content_type);

        req.send().await?;

        info!("finish uploading file {} to S3 bucket", path.display());
        Ok(())
    }
}
