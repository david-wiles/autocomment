use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Credentials {
    /// Jira Username
    pub jira_user: String,

    /// Jira Password
    pub jira_pass: String,

    /// Jira Domain
    pub jira_domain: String,

    /// Github User
    pub github_user: String,

    /// Github Password
    pub github_pass: String,

    /// Github Domain
    pub github_domain: String,
}

impl Credentials {
    pub fn from_env() -> Result<Credentials, Error> {
        let f = std::fs::File::open(Self::config_file().as_path())?;
        serde_yaml::from_reader(f).map_err(Error::from)
    }

    pub fn save(&self) -> Result<(), Error> {
        let pathbuf = Self::config_file();
        let p = pathbuf.as_path();

        if !p.exists() {
            // Create directory if it doesn't exist
            if let Some(parent) = p.parent() {
                if !parent.exists() {
                    std::fs::create_dir(parent)?;
                }
            }
            // What if the parent directory is None?
        }

        let f = std::fs::File::create(p)?;
        serde_yaml::to_writer(f, self).map_err(Error::from)
    }

    /// Gets the default config file from the current user's home directory or
    /// from the current directory if there is no home
    fn config_file() -> PathBuf {
        home::home_dir()
            .map(|home_dir| home_dir.join(Path::new(".autocomment/config.yaml")))
            .unwrap_or(PathBuf::from(".autocomment/config.yaml"))
    }
}

pub enum Error {
    AutocommentError(String),
    SerdeJsonError(serde_json::Error),
    SerdeYamlError(serde_yaml::Error),
    ReqwestError(reqwest::Error),
    FsError(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Error::AutocommentError(err) => err.clone(),
            Error::SerdeJsonError(err) => "Unable to read response: ".to_owned() + &err.to_string(),
            Error::SerdeYamlError(err) => "Unable to read credentials: ".to_owned() + &err.to_string(),
            Error::ReqwestError(err) => "Unable to send request: ".to_owned() + &err.to_string(),
            Error::FsError(err) => "Filesystem issue: ".to_owned() + &err.to_string()
        };
        write!(f, "Error: {}", msg)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::AutocommentError(err)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::SerdeYamlError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJsonError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::FsError(err)
    }
}

#[derive(Serialize, Deserialize)]
struct GHPullRequest {
    base: GHPullRequestBase,
    html_url: String,
    title: String,
    body: Option<String>,
    created_at: String,
    user: GHPullRequestOwner,
}

#[derive(Serialize, Deserialize)]
struct GHPullRequestBase {
    repo: GHRepo
}

#[derive(Serialize, Deserialize)]
struct GHRepo {
    full_name: String,
}

#[derive(Serialize, Deserialize)]
struct GHPullRequestOwner {
    login: String,
}

fn get_pull_requests_for_repo(repo: String, filters: String, cred: &Credentials) -> Result<Vec<GHPullRequest>, Error> {
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

fn parse_jira_ticket_number(pr_body: String) -> Option<String> {
    let re = regex::Regex::new(r"\[(\w+\-\d+)\]\(https://taserintl\.atlassian\.net\S+\)").unwrap();
    for group in re.captures_iter(pr_body.as_str()) {
        // Return the first matching ticket
        return Some(group[1].to_string());
    }
    None
}

#[derive(Serialize, Deserialize)]
struct JiraCommentResponse {
    total: i32,
    comments: Vec<JiraComment>,
}

#[derive(Serialize, Deserialize)]
struct JiraComment {
    #[serde(rename = "renderedBody")]
    rendered_body: String,
}

fn post_jira_comment(ticket_id: String, pr: &GHPullRequest, cred: &Credentials) -> Result<String, Error> {
    let jira_url = format!("https://{}/rest/api/3/issue/{}/comment?expand=renderedBody", cred.jira_domain, ticket_id);

    // Get the first line of the PR body
    let pr_body_line_1 = pr.body.clone().ok_or(Error::from(format!("Pull Request {} has an invalid description", pr.html_url)))?.take_until('\n');

    // Build comment json
    let comment_text = format!("{{\"body\": {{ \"type\": \"doc\", \"version\": 1, \"content\": [ {{ \"type\": \"paragraph\", \"content\": [ {{ \"text\": \"Pull Request {}: {}\\n\\t{}\\n\\t{}\\n\\tCreated at: {}\", \"type\": \"text\" }} ] }} ] }} }}", pr.base.repo.full_name, pr.html_url, pr.title, pr_body_line_1, pr.created_at);

    let resp = Client::new()
        .post(jira_url)
        .basic_auth(cred.jira_user.clone(), Some(cred.jira_pass.clone()))
        .header("Content-Type", "application/json")
        .body(comment_text)
        .send()?;

    if resp.status().is_success() {
        Ok(format!("Added Jira Comment on ticket {} from {}", ticket_id, pr.html_url.clone()))
    } else {
        Err(Error::from("Unable to post Jira comment: ".to_owned() + &resp.status().to_string()))
    }
}

fn jira_ticket_has_existing_comment(ticket_id: String, pr: &GHPullRequest, cred: &Credentials) -> Result<bool, Error> {
    let jira_url = format!("https://{}/rest/api/3/issue/{}/comment?expand=renderedBody", cred.jira_domain, ticket_id);

    let resp = Client::new()
        .get(jira_url)
        .basic_auth(cred.jira_user.clone(), Some(cred.jira_pass.clone()))
        .send()?;

    if resp.status().is_success() {
        // Check if this ticket has no comments or if any comment contains the PR's URL
        let comments: JiraCommentResponse = serde_json::from_str(resp.text()?.as_str())?;
        Ok(comments.total == 0 || comments.comments.iter().any(|comment| comment.rendered_body.contains(&pr.html_url.clone())))
    } else {
        Err(Error::from(resp.text()?))
    }
}

pub fn sync_comments(repo: String, filters: String, creds: &Credentials) -> Result<Vec<String>, Error> {
    get_pull_requests_for_repo(repo, filters, creds)?.iter()
        .map(|pr| {
            if let Some(pr_body) = pr.body.clone() {
                if let Some(jira_id) = parse_jira_ticket_number(pr_body) {
                    if !jira_ticket_has_existing_comment(jira_id.clone(), pr, creds)? {
                        post_jira_comment(jira_id, pr, creds)
                    } else {
                        Ok(format!("Jira ticket {} already has comment for {}", jira_id, pr.html_url.clone()))
                    }
                } else {
                    Ok(format!("PR {} does not contain a Jira ticket!", pr.html_url.clone()))
                }
            } else {
                Ok(format!("PR {} does not have a description!", pr.html_url.clone()))
            }
        })
        .collect()
}

trait Take<T> {
    fn take_until(&self, limit: char) -> T;
    fn take(&self, count: usize) -> T;
}

impl Take<Self> for String {
    fn take_until(&self, limit: char) -> Self {
        if let Some(idx) = self.find(limit) {
            self[..idx-1].to_string()
        } else {
            self.clone()
        }
    }

    fn take(&self, count: usize) -> Self {
        if count < self.len() {
            self[..count-1].to_string()
        } else {
            self.clone()
        }
    }
}