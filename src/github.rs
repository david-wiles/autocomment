use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

use crate::credentials::Credentials;
use crate::error::Error;
use crate::TakeUntil;

/// Representation of a Github Pull Request, only including
/// the fields needed to create a comment on a matching Jira
/// ticket.
#[derive(Serialize, Deserialize, Clone)]
pub struct GHPullRequest {
    pub base: GHPullRequestBase,
    pub html_url: String,
    pub title: String,
    pub body: Option<String>,
    pub created_at: String,
    pub user: GHPullRequestOwner,
}

impl GHPullRequest {
    pub fn build_jira_comment(&self) -> Result<String, Error> {
        // Get the first line of the PR body
        let pr_body_line_1 = self.body.clone().ok_or(Error::from(format!("Pull Request {} has an invalid description", self.html_url)))?.take_until('\n');
        // Build comment json
        Ok(format!("{{\"body\": {{ \"type\": \"doc\", \"version\": 1, \"content\": [ {{ \"type\": \"paragraph\", \"content\": [ {{ \"text\": \"Pull Request {}: {}\\n\\t{}\\n\\t{}\\n\\tCreated at: {}\", \"type\": \"text\" }} ] }} ] }} }}", self.base.repo.full_name, self.html_url, self.title, pr_body_line_1.trim(), self.created_at))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GHPullRequestBase {
    pub repo: GHRepo
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GHRepo {
    pub full_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GHPullRequestOwner {
    pub login: String,
}

pub trait GithubClient {
    /// Get a list of all pull requests for a repo, using the filters provided.
    /// Only pull requests created by the user found in the Credentials will be
    /// returned.
    fn get_pull_requests_for_repo(&self, repo: String, filters: String) -> Result<Vec<GHPullRequest>, Error>;
}

pub struct DefaultGithubClient<'a> {
    client: Client,
    creds: &'a Credentials
}

impl<'a> DefaultGithubClient<'a> {
    pub fn new(creds: &'a Credentials) -> DefaultGithubClient<'a> {
        let client: Client = Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap();

        DefaultGithubClient { client, creds }
    }
}

impl<'a> GithubClient for DefaultGithubClient<'a> {
    fn get_pull_requests_for_repo(&self, repo: String, filters: String) -> Result<Vec<GHPullRequest>, Error> {
        let gh_url = format!("https://{}/repos/{}/pulls{}", self.creds.github_domain, repo, filters);

        let resp = self.client.get(gh_url)
            .basic_auth(self.creds.github_user.clone(), Some(self.creds.github_pass.clone()))
            .send()?;

        if resp.status().is_success() {
            let prs: Vec<GHPullRequest> = serde_json::from_str(resp.text()?.as_str())?;
            Ok(prs.into_iter().filter(|pr| pr.user.login == self.creds.github_user).collect())
        } else {
            Err(Error::from(resp.text()?))
        }
    }
}

pub struct MockGithubClient {
    pub data: Box<Vec<GHPullRequest>>
}

impl GithubClient for MockGithubClient {
    fn get_pull_requests_for_repo(&self, repo: String, filters: String) -> Result<Vec<GHPullRequest>, Error> {
        Ok(*self.data.clone())
    }
}

#[cfg(test)]
mod test {
    use crate::GHPullRequest;
    use crate::github::{GHPullRequestBase, GHPullRequestOwner};
    use crate::github::GHRepo;

    #[test]
    fn build_jira_comment_success() {
        let pr = GHPullRequest{
            base: GHPullRequestBase {
                repo: GHRepo { full_name: "test".to_string() }
            },
            html_url: "https://url/org/repo".to_string(),
            title: "test title".to_string(),
            body: Some("test body\nwith two lines".to_string()),
            created_at: "datetime".to_string(),
            user: GHPullRequestOwner { login: "me".to_string() }
        };

        let format = "{\"body\": { \"type\": \"doc\", \"version\": 1, \"content\": [ { \"type\": \"paragraph\", \"content\": [ { \"text\": \"Pull Request test: https://url/org/repo\\n\\ttest title\\n\\ttest body\\n\\tCreated at: datetime\", \"type\": \"text\" } ] } ] } }".to_string();

        assert_eq!(format, pr.build_jira_comment().unwrap())
    }

    #[test]
    fn build_jira_comment_failure() {
        let pr = GHPullRequest{
            base: GHPullRequestBase {
                repo: GHRepo { full_name: "test".to_string() }
            },
            html_url: "https://url/org/repo".to_string(),
            title: "test title".to_string(),
            body: None,
            created_at: "datetime".to_string(),
            user: GHPullRequestOwner { login: "me".to_string() }
        };

        assert!(pr.build_jira_comment().is_err())
    }
}
