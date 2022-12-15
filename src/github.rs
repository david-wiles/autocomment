use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

use crate::credentials::Credentials;
use crate::error::Error;
use crate::jira::{JiraCommentAttrs, JiraCommentElement, JiraCommentRequest};
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
    pub fn build_jira_comment(&self) -> Result<JiraCommentRequest, Error> {
        let pr_body = self.body.clone().ok_or(Error::from(format!("Pull Request {} has an invalid description", self.html_url)))?;

        let mut jira_comment = JiraCommentRequest::new();
        let jira_content = vec![
            JiraCommentElement::new("paragraph".to_string())
                .with_content(vec![
                    JiraCommentElement::new("text".to_string())
                        .with_text(format!("Pull Request in {}: ", self.base.repo.full_name))
                        .to_owned(),
                    JiraCommentElement::new("text".to_string())
                        .with_text(self.title.clone())
                        .with_marks(vec![
                            JiraCommentElement::new("link".to_string())
                                .with_attrs(JiraCommentAttrs{ href: Some(self.html_url.clone()) })
                                .to_owned()
                        ])
                        .to_owned()
                ])
                .to_owned(),
            JiraCommentElement::new("paragraph".to_string())
                .with_content(vec![
                    JiraCommentElement::new("text".to_string())
                        .with_text(pr_body.as_str().take_until('\n').trim().to_string())
                        .to_owned()
                ])
                .to_owned(),
            JiraCommentElement::new("paragraph".to_string())
                .with_content(vec![
                    JiraCommentElement::new("text".to_string())
                        .with_text(format!("Created at: {}", self.created_at))
                        .to_owned()
                ])
                .to_owned(),
        ];

        Ok(jira_comment.with_content(jira_content).to_owned())
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
    fn get_pull_requests_for_repo(&self, repo: &str, filters: &str) -> Result<Vec<GHPullRequest>, Error>;
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
    fn get_pull_requests_for_repo(&self, repo: &str, filters: &str) -> Result<Vec<GHPullRequest>, Error> {
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
    fn get_pull_requests_for_repo(&self, _repo: &str, _filters: &str) -> Result<Vec<GHPullRequest>, Error> {
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

        let format = "{\"body\":{\"version\":1,\"type\":\"doc\",\"content\":[{\"type\":\"paragraph\",\"content\":[{\"type\":\"text\",\"text\":\"Pull Request in test: \"},{\"type\":\"text\",\"text\":\"test title\",\"marks\":[{\"type\":\"link\",\"attrs\":{\"href\":\"https://url/org/repo\"}}]}]},{\"type\":\"paragraph\",\"content\":[{\"type\":\"text\",\"text\":\"test body\"}]},{\"type\":\"paragraph\",\"content\":[{\"type\":\"text\",\"text\":\"Created at: datetime\"}]}]}}".to_string();

        assert_eq!(format, serde_json::to_string(&pr.build_jira_comment().unwrap()).unwrap())
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
