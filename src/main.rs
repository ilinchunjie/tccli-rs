mod content_delivery;

use clap::{Args, Parser, Subcommand, ValueEnum};
use content_delivery::{PurgePathCacheRequest, TencentCloudCredentials, purge_path_cache};
use std::process;

#[derive(Debug, Parser)]
#[command(name = "tccli-rs", version, about, long_about = None)]
#[command(subcommand_required = true, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 提交腾讯云 CDN 目录刷新任务
    PurgePathCache(PurgePathCacheArgs),
}

#[derive(Debug, Args)]
struct PurgePathCacheArgs {
    #[arg(
        long,
        required = true,
        value_name = "SECRET_ID",
        value_parser = validate_secret_id,
        help = "腾讯云 SecretId"
    )]
    secret_id: String,

    #[arg(
        long,
        required = true,
        value_name = "SECRET_KEY",
        value_parser = validate_secret_key,
        help = "腾讯云 SecretKey"
    )]
    secret_key: String,

    #[arg(
        long,
        required = true,
        value_name = "REGION",
        value_parser = validate_region,
        help = "腾讯云地域，例如 ap-shanghai"
    )]
    region: String,

    #[arg(
        long,
        required = true,
        num_args = 1..,
        value_name = "URL",
        value_parser = validate_purge_path,
        help = "目录列表，需要包含协议头部 http:// 或 https://"
    )]
    paths: Vec<String>,

    #[arg(
        long,
        value_enum,
        help = "刷新类型：flush 刷新产生更新的资源；delete 刷新全部资源"
    )]
    flush_type: FlushType,

    #[arg(long, help = "是否对中文字符进行编码后刷新")]
    url_encode: Option<bool>,

    #[arg(
        long,
        help = "刷新区域字符串，示例值 mainland 或 overseas；省略时按域名默认加速区域处理"
    )]
    area: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum FlushType {
    Flush,
    Delete,
}

impl FlushType {
    fn as_api_value(self) -> &'static str {
        match self {
            Self::Flush => "flush",
            Self::Delete => "delete",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::PurgePathCache(args) => run_purge_path_cache(args),
    }
}

fn validate_purge_path(value: &str) -> Result<String, String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        Ok(value.to_owned())
    } else {
        Err("path must start with http:// or https://".to_owned())
    }
}

fn validate_secret_id(value: &str) -> Result<String, String> {
    validate_non_empty("SecretId", value)
}

fn validate_secret_key(value: &str) -> Result<String, String> {
    validate_non_empty("SecretKey", value)
}

fn validate_region(value: &str) -> Result<String, String> {
    validate_non_empty("Region", value)
}

fn run_purge_path_cache(args: PurgePathCacheArgs) -> Result<(), String> {
    let credentials = TencentCloudCredentials {
        secret_id: args.secret_id,
        secret_key: args.secret_key,
        region: args.region,
    };
    let request = PurgePathCacheRequest {
        paths: args.paths,
        flush_type: args.flush_type.as_api_value().to_owned(),
        url_encode: args.url_encode,
        area: args.area,
    };
    let response = purge_path_cache(&credentials, &request).map_err(|err| err.to_string())?;

    println!(
        "purge-path-cache submitted successfully. task_id={}, request_id={}",
        response.task_id, response.request_id
    );

    Ok(())
}

fn validate_non_empty(field_name: &str, value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("{field_name} must not be empty"))
    } else {
        Ok(value.to_owned())
    }
}