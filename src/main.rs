mod content_delivery;

use clap::{Args, Parser, Subcommand, ValueEnum};
use content_delivery::{
    HttpHeader, PurgePathCacheRequest, PurgeUrlsCacheRequest, PushUrlsCacheRequest,
    TencentCloudCredentials, purge_path_cache, purge_urls_cache, push_urls_cache,
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
    /// Submit a Tencent Cloud CDN URL prefetch task
    PushUrlsCache(PushUrlsCacheArgs),
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

#[derive(Debug, Args)]
struct PushUrlsCacheArgs {
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

    #[arg(long, help = "User-Agent sent by Tencent Cloud when prefetching")]
    user_agent: Option<String>,

    #[arg(
        long,
        value_name = "AREA",
        value_parser = validate_push_area,
        help = "Prefetch area: mainland, overseas, or global"
    )]
    area: Option<String>,

    #[arg(
        long,
        value_name = "LAYER",
        value_parser = validate_push_layer,
        help = "Prefetch layer, only middle is currently supported by the API"
    )]
    layer: Option<String>,

    #[arg(
        long,
        help = "Whether to parse M3U8 files and prefetch the listed segments"
    )]
    parse_m3u8: Option<bool>,

    #[arg(long, help = "Whether to disable Range requests during prefetch")]
    disable_range: Option<bool>,

    #[arg(
        long = "header",
        value_name = "NAME:VALUE",
        value_parser = parse_http_header,
        help = "Custom request header in NAME:VALUE format; repeat this option to add multiple headers"
    )]
    headers: Vec<HttpHeader>,

    #[arg(
        long,
        help = "Whether to URL-encode Chinese characters before prefetching"
    )]
    url_encode: Option<bool>,
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
        Commands::PushUrlsCache(args) => run_push_urls_cache(args),
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

fn validate_push_area(value: &str) -> Result<String, String> {
    match value {
        "mainland" | "overseas" | "global" => Ok(value.to_owned()),
        _ => Err("area must be one of mainland, overseas, or global".to_owned()),
    }
}

fn validate_push_layer(value: &str) -> Result<String, String> {
    if value == "middle" {
        Ok(value.to_owned())
    } else {
        Err("layer must be middle".to_owned())
    }
}

fn parse_http_header(value: &str) -> Result<HttpHeader, String> {
    let (name, header_value) = value
        .split_once(':')
        .ok_or_else(|| "header must be in NAME:VALUE format".to_owned())?;
    let name = name.trim();
    let header_value = header_value.trim();

    if name.is_empty() {
        return Err("header name must not be empty".to_owned());
    }

    if header_value.is_empty() {
        return Err("header value must not be empty".to_owned());
    }

    if name.len() > 128 {
        return Err("header name must not exceed 128 bytes".to_owned());
    }

    if header_value.len() > 1024 {
        return Err("header value must not exceed 1024 bytes".to_owned());
    }

    Ok(HttpHeader {
        name: name.to_owned(),
        value: header_value.to_owned(),
    })
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

fn run_push_urls_cache(args: PushUrlsCacheArgs) -> Result<(), String> {
    if args.headers.len() > 20 {
        return Err("at most 20 custom headers can be specified".to_owned());
    }

    let credentials = TencentCloudCredentials {
        secret_id: args.secret_id,
        secret_key: args.secret_key,
    };
    let request = PushUrlsCacheRequest {
        urls: args.urls,
        user_agent: args.user_agent,
        area: args.area,
        layer: args.layer,
        parse_m3u8: args.parse_m3u8,
        disable_range: args.disable_range,
        headers: (!args.headers.is_empty()).then_some(args.headers),
        url_encode: args.url_encode,
    };
    let response = push_urls_cache(&credentials, &request).map_err(|err| err.to_string())?;

    println!(
        "push-urls-cache submitted successfully. task_id={}, request_id={}",
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
