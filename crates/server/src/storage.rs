use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::Region;

pub async fn create_bucket(
    endpoint: &str,
    bucket_name: &str,
    access_key: &str,
    secret_key: &str,
) -> Box<Bucket> {
    let region = Region::Custom {
        region: "us-east-1".to_string(),
        endpoint: endpoint.to_string(),
    };

    let credentials = Credentials::new(Some(access_key), Some(secret_key), None, None, None)
        .expect("Failed to create S3 credentials");

    let bucket = Bucket::new(bucket_name, region.clone(), credentials.clone())
        .expect("Failed to create S3 bucket handle")
        .with_path_style();

    // Create bucket if it doesn't exist (MinIO)
    match s3::bucket::Bucket::create_with_path_style(
        bucket_name,
        region,
        credentials,
        s3::BucketConfiguration::default(),
    )
    .await
    {
        Ok(_) => tracing::info!("S3 bucket '{}' created or already exists", bucket_name),
        Err(e) => tracing::warn!("S3 bucket creation: {e} (may already exist)"),
    }

    // Verify connectivity with a list call
    match bucket.list("__ping__".to_string(), Some("/".to_string())).await {
        Ok(_) => tracing::info!("S3 connectivity verified"),
        Err(e) => tracing::error!("S3 connectivity check FAILED: {e}"),
    }

    bucket
}

/// Delete all objects with a given prefix.
pub async fn delete_prefix(bucket: &Bucket, prefix: &str) -> Result<(), String> {
    let list = bucket
        .list(prefix.to_string(), None)
        .await
        .map_err(|e| format!("S3 list failed: {e}"))?;

    for item in list {
        for obj in item.contents {
            bucket
                .delete_object(&obj.key)
                .await
                .map_err(|e| format!("S3 delete failed for {}: {e}", obj.key))?;
        }
    }
    Ok(())
}

/// Upload a file to S3/MinIO.
/// Returns the object key.
pub async fn upload(
    bucket: &Bucket,
    key: &str,
    data: &[u8],
    content_type: &str,
) -> Result<(), String> {
    bucket
        .put_object_with_content_type(key, data, content_type)
        .await
        .map_err(|e| format!("S3 upload failed: {e}"))?;
    Ok(())
}

/// Download a file from S3/MinIO.
/// Returns (data, content_type).
pub async fn download(bucket: &Bucket, key: &str) -> Result<Vec<u8>, String> {
    let response = bucket
        .get_object(key)
        .await
        .map_err(|e| format!("S3 download failed: {e}"))?;

    if response.status_code() != 200 {
        return Err(format!("S3 returned status {}", response.status_code()));
    }

    Ok(response.to_vec())
}
