#![allow(clippy::too_many_arguments)]

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

#[tauri::command]
pub async fn test_connection(
    typ: String,
    path: Option<String>,
    bucket: Option<String>,
    region: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    endpoint: Option<String>,
    webdav_endpoint: Option<String>,
    root: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<TestConnectionResult, String> {
    match typ.as_str() {
        "local" => test_local_connection(&path).await,
        "s3" => test_s3_connection(&bucket, &region, &access_key, &secret_key, &endpoint).await,
        "webdav" => test_webdav_connection(&webdav_endpoint, &root, &username, &password).await,
        _ => Ok(TestConnectionResult {
            success: false,
            message: "不支持的存储类型".to_string(),
            details: None,
        }),
    }
}

async fn test_local_connection(path: &Option<String>) -> Result<TestConnectionResult, String> {
    let path = path
        .as_ref()
        .ok_or_else(|| "本地路径不能为空".to_string())?;

    let std_path = std::path::Path::new(path);

    if !std_path.exists() {
        return Ok(TestConnectionResult {
            success: false,
            message: "路径不存在".to_string(),
            details: Some(format!("路径 '{}' 不存在", path)),
        });
    }

    if !std_path.is_dir() {
        return Ok(TestConnectionResult {
            success: false,
            message: "路径不是文件夹".to_string(),
            details: Some(format!("'{}' 不是一个文件夹", path)),
        });
    }

    let metadata = std::fs::metadata(std_path).map_err(|e| format!("无法访问路径: {}", e))?;

    let readonly = metadata.permissions().readonly();

    Ok(TestConnectionResult {
        success: true,
        message: "连接成功".to_string(),
        details: Some(
            if readonly {
                "只读访问"
            } else {
                "读写访问"
            }
            .to_string(),
        ),
    })
}

async fn test_s3_connection(
    bucket: &Option<String>,
    region: &Option<String>,
    access_key: &Option<String>,
    secret_key: &Option<String>,
    endpoint: &Option<String>,
) -> Result<TestConnectionResult, String> {
    use opendal::services::S3;
    use opendal::Operator;

    let bucket = bucket
        .as_ref()
        .ok_or_else(|| "S3 bucket 不能为空".to_string())?;

    let region = region
        .as_ref()
        .ok_or_else(|| "S3 region 不能为空".to_string())?;

    let access_key = access_key
        .as_ref()
        .ok_or_else(|| "Access Key 不能为空".to_string())?;

    let secret_key = secret_key
        .as_ref()
        .ok_or_else(|| "Secret Key 不能为空".to_string())?;

    let mut builder = S3::default()
        .bucket(bucket)
        .region(region)
        .access_key_id(access_key)
        .secret_access_key(secret_key);

    if let Some(ep) = endpoint {
        if !ep.is_empty() {
            builder = builder.endpoint(ep);
        }
    }

    let operator = Operator::new(builder)
        .map_err(|e| format!("S3 配置错误: {}", e))?
        .finish();

    match operator.list("").await {
        Ok(_) => Ok(TestConnectionResult {
            success: true,
            message: "S3 连接成功".to_string(),
            details: Some(format!("Bucket: {}", bucket)),
        }),
        Err(e) => Ok(TestConnectionResult {
            success: false,
            message: "S3 连接失败".to_string(),
            details: Some(format!("检查凭证和 bucket 名称: {}", e)),
        }),
    }
}

async fn test_webdav_connection(
    webdav_endpoint: &Option<String>,
    root: &Option<String>,
    username: &Option<String>,
    password: &Option<String>,
) -> Result<TestConnectionResult, String> {
    use opendal::services::Webdav;
    use opendal::Operator;

    let endpoint = webdav_endpoint
        .as_ref()
        .ok_or_else(|| "WebDAV endpoint 不能为空".to_string())?;

    let username = username
        .as_ref()
        .ok_or_else(|| "WebDAV 用户名不能为空".to_string())?;

    let password = password
        .as_ref()
        .ok_or_else(|| "WebDAV 密码不能为空".to_string())?;

    // 如果有 root 路径，将其拼接到 endpoint 中（避免 OpenDAL 的 URL 编码问题）
    let final_endpoint = if let Some(r) = root {
        if !r.is_empty() {
            let trimmed_endpoint = endpoint.trim_end_matches('/');
            let trimmed_root = r.trim_start_matches('/').trim_end_matches('/');
            format!("{}/{}", trimmed_endpoint, trimmed_root)
        } else {
            endpoint.clone()
        }
    } else {
        endpoint.clone()
    };

    let builder = Webdav::default()
        .endpoint(&final_endpoint)
        .username(username)
        .password(password);

    let operator = Operator::new(builder)
        .map_err(|e| format!("WebDAV 配置错误: {}", e))?
        .finish();

    match operator.list("").await {
        Ok(_) => {
            Ok(TestConnectionResult {
                success: true,
                message: "WebDAV 连接成功".to_string(),
                details: Some(final_endpoint),
            })
        }
        Err(e) => Ok(TestConnectionResult {
            success: false,
            message: "WebDAV 连接失败".to_string(),
            details: Some(format!("检查凭证和服务器地址: {}", e)),
        }),
    }
}
