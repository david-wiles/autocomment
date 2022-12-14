use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

use crate::credentials::Credentials;
use crate::error::Error;

#[derive(Serialize, Deserialize)]
pub struct GHPullRequest {
    pub base: GHPullRequestBase,
    pub html_url: String,
    pub title: String,
    pub body: Option<String>,
    pub created_at: String,
    pub user: GHPullRequestOwner,
}

#[derive(Serialize, Deserialize)]
pub struct GHPullRequestBase {
    pub repo: GHRepo
}

#[derive(Serialize, Deserialize)]
pub struct GHRepo {
    pub full_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GHPullRequestOwner {
    pub login: String,
}

pub fn get_pull_requests_for_repo(repo: String, filters: String, cred: &Credentials) -> Result<Vec<GHPullRequest>, Error> {
    let gh_url = format!("https://{}/repos/{}/pulls{}", cred.github_domain, repo, filters);
    let gh_client: Client = Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_hostnames(true)
        .build()?;

    let resp = gh_client.get(gh_url)
        .basic_auth(cred.github_user.clone(), Some(cred.github_pass.clone()))
        .send()?;

    if resp.status().is_success() {
        let prs: Vec<GHPullRequest> = serde_json::from_str(resp.text()?.as_str())?;
        Ok(prs.into_iter().filter(|pr| pr.user.login == cred.github_user).collect())
    } else {
        Err(Error::from(resp.text()?))
    }
}
