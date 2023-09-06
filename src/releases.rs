use chrono::{Days, Duration, NaiveDate, Utc};

use crate::Command;

const ARTIFACT: &str = "llvm-tools-nightly-aarch64-unknown-linux-gnu.tar.gz";

#[derive(Debug, Default)]
pub struct ReleasesCommand {
    current_step: Option<NaiveDate>,
}

impl ReleasesCommand {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Command for ReleasesCommand {
    fn next_step(&mut self) -> Option<String> {
        let current_step = self.current_step.unwrap_or_else(|| {
            Utc::now()
                .checked_add_days(Days::new(1))
                .unwrap()
                .date_naive()
        });

        let next_step = current_step - Duration::days(1);
        self.current_step = Some(next_step);

        Some(format_step(&next_step))
    }

    fn cloudfront_url(&self) -> Option<String> {
        let step = self.current_step?;

        Some(format!(
            "https://cloudfront-static.rust-lang.org/dist/{step}/{ARTIFACT}"
        ))
    }

    fn fastly_url(&self) -> Option<String> {
        let step = self.current_step?;

        Some(format!(
            "https://fastly-static.rust-lang.org/dist/{step}/{ARTIFACT}"
        ))
    }

    fn s3_url(&self) -> Option<String> {
        let step = self.current_step?;

        Some(format!(
            "https://static-rust-lang-org.s3.us-west-1.amazonaws.com/dist/{step}/{ARTIFACT}"
        ))
    }
}

fn format_step(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
