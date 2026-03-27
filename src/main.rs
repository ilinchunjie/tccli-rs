mod content_delivery;

use clap::{Args, Parser, Subcommand, ValueEnum};
use content_delivery::{
    PurgePathCacheRequest, PurgeUrlsCacheRequest, TencentCloudCredentials, purge_path_cache,
    purge_urls_cache,
};
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
    /// Submit a Tencent Cloud CDN directory purge task
    PurgePathCache(PurgePathCacheArgs),
    /// Submit a Tencent Cloud CDN URL purge task
    PurgeUrlsCache(PurgeUrlsCacheArgs),
}

#[derive(Debug, Args)]
struct PurgePathCacheArgs {
    #[arg(
        long,
        required = true,
        value_name = "SECRET_ID",
        value_parser = validate_secret_id,
        help = "Tencent Cloud SecretId"
    )]
    secret_id: String,

    #[arg(
        long,
        required = true,
        value_name = "SECRET_KEY",
        value_parser = validate_secret_key,
        help = "Tencent Cloud SecretKey"
    )]
    secret_key: String,

    #[arg(
        long,
        required = true,
        value_name = "REGION",
        value_parser = validate_region,
        help = "Tencent Cloud region, for example ap-shanghai"
    )]
    region: String,

    #[arg(
        long,
        required = true,
        num_args = 1..,
        value_name = "URL",
        value_parser = validate_url_with_scheme,
        help = "Directory list, each value must start with http:// or https://"
    )]
    paths: Vec<String>,

    #[arg(
        long,
        value_enum,
        help = "Purge type: flush refreshes updated resources, delete refreshes all resources"
    )]
    flush_type: FlushType,

    #[arg(long, help = "Whether to URL-encode Chinese characters before purging")]
    url_encode: Option<bool>,

    #[arg(
        long,
        help = "Purge area, for example mainland or overseas; omitted to follow the domain's default acceleration area"
    )]
    area: Option<String>,
}

#[derive(Debug, Args)]
struct PurgeUrlsCacheArgs {
    #[arg(
        long,
        required = true,
        value_name = "SECRET_ID",
        value_parser = validate_secret_id,
        help = "Tencent Cloud SecretId"
    )]
    secret_id: String,

    #[arg(
        long,
        required = true,
        value_name = "SECRET_KEY",
        value_parser = validate_secret_key,
        help = "Tencent Cloud SecretKey"
    )]
    secret_key: String,

    #[arg(
        long,
        required = true,
        num_args = 1..,
        value_name = "URL",
        value_parser = validate_url_with_scheme,
        help = "URL list, each value must start with http:// or https://"
    )]
    urls: Vec<String>,

    #[arg(long, help = "Whether to URL-encode Chinese characters before purging")]
    url_encode: Option<bool>,

    #[arg(
        long,
        help = "Purge area, for example mainland or overseas; omitted to follow the domain's default acceleration area"
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
        Commands::PurgeUrlsCache(args) => run_purge_urls_cache(args),
    }
}

fn validate_url_with_scheme(value: &str) -> Result<String, String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        Ok(value.to_owned())
    } else {
        Err("value must start with http:// or https://".to_owned())
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

fn run_purge_urls_cache(args: PurgeUrlsCacheArgs) -> Result<(), String> {
    let credentials = TencentCloudCredentials {
        secret_id: args.secret_id,
        secret_key: args.secret_key,
    };
    let request = PurgeUrlsCacheRequest {
        urls: args.urls,
        url_encode: args.url_encode,
        area: args.area,
    };
    let response = purge_urls_cache(&credentials, &request).map_err(|err| err.to_string())?;

    println!(
        "purge-urls-cache submitted successfully. task_id={}, request_id={}",
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
