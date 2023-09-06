use std::time::Duration;

use chrono::Utc;
use clap::{Parser, Subcommand};
use log::{debug, info, warn};
use reqwest::Method;

use crate::releases::ReleasesCommand;

mod releases;

/// Gather data for debugging rust-lang/simpleinfra#340
#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Maximum number of attempts to download an uncached artifacts
    #[arg(short, long, default_value_t = 20)]
    attempts: usize,

    /// Number of samples to gather
    #[arg(short, long, default_value_t = 5)]
    samples: usize,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Releases,
}

trait Command {
    fn next_step(&mut self) -> Option<String>;

    fn cloudfront_url(&self) -> Option<String>;
    fn fastly_url(&self) -> Option<String>;
    fn s3_url(&self) -> Option<String>;
}

struct Stats {
    step: String,
    fastly: usize,
    cloudfront: usize,
    s3: usize,
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();
    let mut command = match cli.command {
        Commands::Releases => ReleasesCommand::new(),
    };

    let mut output = Vec::new();
    let mut attempts = 0;

    while output.len() < cli.samples && attempts < cli.attempts {
        if let Some(step) = command.next_step() {
            info!("downloading artifacts for {}", step);

            let stats = download_artifacts(
                &step,
                command.cloudfront_url().unwrap(),
                command.fastly_url().unwrap(),
                command.s3_url().unwrap(),
            );

            if let Some(stats) = stats {
                output.push(stats);
            }

            attempts += 1;
        } else {
            break;
        }
    }

    println!("| Date       | Fastly     | Cloudfront | S3         |");

    for stat in output {
        println!(
            "| {} | {:>10} | {:>10} | {:>10} |",
            stat.step, stat.fastly, stat.cloudfront, stat.s3
        );
    }
}

fn download_artifacts(
    step: &str,
    cloudfront_url: String,
    fastly_url: String,
    s3_url: String,
) -> Option<Stats> {
    let fastly = download(&fastly_url, Some("x-cache"), Some("HIT"))?;
    let cloudfront = download(&cloudfront_url, Some("x-cache"), Some("Hit"))?;
    let s3 = download(&s3_url, None, None)?;

    Some(Stats {
        step: step.to_string(),
        fastly,
        cloudfront,
        s3,
    })
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

    debug!("downloaded {} MB", bytes.len() / 1000 / 1000);
    debug!("start time: {}, end time: {}", start_time, end_time);

    let speed_in_kb = bytes.len() as f64 / 1000.0 / duration.num_seconds() as f64;

    Some(speed_in_kb as usize)
}
