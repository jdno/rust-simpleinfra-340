use std::time::Duration;

use chrono::Utc;
use clap::Parser;
use log::{debug, warn};
use reqwest::Method;

const ARTIFACT: &str = "llvm-tools-nightly-aarch64-unknown-linux-gnu.tar.gz";

/// Gather data for debugging rust-lang/simpleinfra#340
#[derive(Debug, Parser)]
struct Args {
    /// Maximum number of attempts to download an uncached artifacts
    #[arg(short, long, default_value_t = 20)]
    attempts: usize,

    /// Number of samples to gather
    #[arg(short, long, default_value_t = 5)]
    samples: usize,
}

struct Stats {
    day: String,
    fastly: usize,
    cloudfront: usize,
    s3: usize,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    let mut output = Vec::new();

    let mut attempts = 0;
    let date = Utc::now().date_naive();

    while output.len() < args.samples && attempts < args.attempts {
        let day = (date - chrono::Duration::days(attempts as i64))
            .format("%Y-%m-%d")
            .to_string();

        debug!("downloading artifacts for {}", day);

        let stats = download_artifacts(&day);

        if let Some(stats) = stats {
            output.push(stats);
        }

        attempts += 1;
    }

    println!("| Date       | Fastly     | Cloudfront | S3         |");

    for stat in output {
        println!(
            "| {} | {:>10} | {:>10} | {:>10} |",
            stat.day, stat.fastly, stat.cloudfront, stat.s3
        );
    }
}

fn download_artifacts(day: &str) -> Option<Stats> {
    let fastly = download_from_fastly(day)?;
    let cloudfront = download_from_cloudfront(day)?;
    let s3 = download_from_s3(day)?;

    Some(Stats {
        day: day.to_string(),
        fastly,
        cloudfront,
        s3,
    })
}

fn download_from_fastly(day: &str) -> Option<usize> {
    let url = format!("https://fastly-static.rust-lang.org/dist/{day}/{ARTIFACT}");
    download(&url, Some("x-cache"), Some("HIT"))
}

fn download_from_cloudfront(day: &str) -> Option<usize> {
    let url = format!("https://cloudfront-static.rust-lang.org/dist/{day}/{ARTIFACT}");
    download(&url, Some("x-cache"), Some("Hit"))
}

fn download_from_s3(day: &str) -> Option<usize> {
    let url =
        format!("https://static-rust-lang-org.s3.us-west-1.amazonaws.com/dist/{day}/{ARTIFACT}");
    download(&url, None, None)
}

fn download(
    url: &str,
    cache_hit_header: Option<&str>,
    cache_hit_value: Option<&str>,
) -> Option<usize> {
    debug!("downloading {}", url);

    let start_time = Utc::now();

    let response = reqwest::blocking::Client::builder()
        .timeout(Some(Duration::from_secs(60 * 10)))
        .build()
        .unwrap()
        .request(Method::GET, url)
        .send()
        .ok()?;

    if !response.status().is_success() {
        warn!(
            "failed to download artifact: {:?}",
            response.text().unwrap()
        );
        return None;
    }

    if let (Some(header), Some(value)) = (cache_hit_header, cache_hit_value) {
        if response
            .headers()
            .get(header)
            .expect("failed to confirm cache hit")
            .to_str()
            .unwrap()
            .starts_with(value)
        {
            debug!("cache hit - skipping");
            return None;
        }
    }

    let bytes = response.bytes().unwrap();

    let end_time = Utc::now();
    let duration = end_time - start_time;

    debug!(
        "downloaded {} MB from CloudFront",
        bytes.len() / 1000 / 1000
    );
    debug!("start time: {}, end time: {}", start_time, end_time);

    let speed_in_kb = bytes.len() as f64 / 1000.0 / duration.num_seconds() as f64;

    Some(speed_in_kb as usize)
}
