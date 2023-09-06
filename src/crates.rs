use std::vec::IntoIter;

use log::debug;
use reqwest::header::{ACCEPT, USER_AGENT};
use semver::Version;
use serde::Deserialize;

use crate::Command;

pub struct CratesCommand {
    krate: String,
    versions: IntoIter<Version>,
    current_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionsPayload {
    versions: Vec<VersionPayload>,
}

#[derive(Debug, Deserialize)]
struct VersionPayload {
    num: Version,
}

impl CratesCommand {
    pub fn new(krate: String) -> Self {
        debug!("fetching versions for {}", krate);

        let response = reqwest::blocking::Client::new()
            .get(format!("https://crates.io/api/v1/crates/{krate}/versions"))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "jdno/rust-simpleinfra-340")
            .send()
            .unwrap();

        let versions = response
            .json::<VersionsPayload>()
            .unwrap()
            .versions
            .into_iter()
            .map(|payload| payload.num)
            .collect::<Vec<Version>>()
            .into_iter();

        Self {
            krate,
            versions,
            current_version: None,
        }
    }
}

impl Command for CratesCommand {
    fn next_step(&mut self) -> Option<String> {
        let current_version = self.versions.next().map(|version| version.to_string());
        self.current_version = current_version.clone();

        current_version
    }

    fn cloudfront_url(&self) -> Option<String> {
        self.current_version.as_ref().map(|step| {
            format!(
                "https://cloudfront-static.crates.io/crates/{}/{}-{}.crate",
                self.krate, self.krate, step
            )
        })
    }

    fn fastly_url(&self) -> Option<String> {
        self.current_version.as_ref().map(|step| {
            format!(
                "https://fastly-static.crates.io/crates/{}/{}-{}.crate",
                self.krate, self.krate, step
            )
        })
    }

    fn s3_url(&self) -> Option<String> {
        self.current_version.as_ref().map(|step| {
            format!(
                "https://crates-io.s3.us-west-1.amazonaws.com/crates/{}/{}-{}.crate",
                self.krate, self.krate, step
            )
        })
    }
}
