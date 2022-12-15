pub mod error;
pub mod github;
pub mod jira;
pub mod credentials;

pub use crate::credentials::Credentials;
pub use crate::github::DefaultGithubClient;
pub use crate::jira::DefaultJiraClient;
pub use crate::error::Error;

use crate::github::GHPullRequest;

pub fn sync_comments(repo: &str, filters: &str, gh_client: &dyn github::GithubClient, jira_client: &dyn jira::JiraClient) -> Result<Vec<String>, Error> {
    gh_client.get_pull_requests_for_repo(repo, filters)?.iter()
        .map(|pr| process_pull_request(jira_client, pr))
        .collect()
}

fn process_pull_request(jira_client: &dyn jira::JiraClient, pr: &GHPullRequest) -> Result<String, Error> {
    let pr_body = pr.body.clone().ok_or(Error::AutocommentError(format!("PR {} does not have a description!", pr.html_url.clone())))?;

    // Parse the PR body to find a JIRA ticket
    if let Some(jira_id) = jira::parse_jira_ticket_number(pr_body.as_str(), jira_client.get_domain()) {

        // Create the URL linking to this specific ticket
        let ticket_url = format!("https://{}/browse/{}", jira_client.get_domain(), jira_id);

        // Do HTTP request to get the comments for this PR
        let comments = jira_client.get_jira_comments(jira_id.as_str())?;

        // Check whether the comments already contain this PR's URL
        if !comments.contains_text(pr.html_url.as_str()) {

            let comment_text = serde_json::to_string(&pr.build_jira_comment()?);

            // Do HTTP request to post the comment
            jira_client.post_jira_comment(jira_id.as_str(), comment_text?.as_str())
                .map(|_| format!("Added Jira Comment on ticket {} from {}.", ticket_url, pr.html_url.clone()))

        } else {
            Ok(format!("Jira ticket {} already has comment for {}.", ticket_url, pr.html_url.clone()))
        }
    } else {
        Ok(format!("PR {} does not contain a Jira ticket!", pr.html_url.clone()))
    }
}

trait TakeUntil<T> {
    fn take_until(&self, limit: char) -> T;
}

impl TakeUntil<Self> for &str {
    fn take_until(&self, limit: char) -> Self {
        if let Some(idx) = self.find(limit) {
            return &self[..idx];
        }
        self
    }
}

#[cfg(test)]
mod test {
    use crate::{GHPullRequest, sync_comments, TakeUntil};
    use crate::github::{GHPullRequestBase, GHPullRequestOwner, GHRepo, MockGithubClient};
    use crate::jira::{JiraComment, JiraCommentResponse, MockJiraClient};

    #[test]
    fn adds_comments_on_prs() {
        let jira_client = MockJiraClient {
            domain: "jira.domain".to_string(),
            data: Box::new(JiraCommentResponse {
                total: 2,
                comments: vec![
                    JiraComment {
                        rendered_body: "asdfageta".to_string()
                    },
                    JiraComment {
                        rendered_body: "aeradadf asafsd asd ".to_string()
                    },
                ],
            })
        };

        let gh_client = MockGithubClient {
            data: Box::new(vec![
                GHPullRequest {
                    base: GHPullRequestBase { repo: GHRepo { full_name: "org/repo".to_string() } },
                    html_url: "https://url/org/repo/1".to_string(),
                    title: "test title".to_string(),
                    body: Some("test body [A-1](https://jira.domain/asdf)".to_string()),
                    created_at: "datetime".to_string(),
                    user: GHPullRequestOwner { login: "me".to_string() },
                },
                GHPullRequest {
                    base: GHPullRequestBase { repo: GHRepo { full_name: "org/repo".to_string() } },
                    html_url: "https://url/org/repo/2".to_string(),
                    title: "test title".to_string(),
                    body: Some("test body".to_string()),
                    created_at: "datetime".to_string(),
                    user: GHPullRequestOwner { login: "me".to_string() },
                },
                GHPullRequest {
                    base: GHPullRequestBase { repo: GHRepo { full_name: "org/repo".to_string() } },
                    html_url: "https://url/org/repo/3".to_string(),
                    title: "test title".to_string(),
                    body: Some("test body".to_string()),
                    created_at: "datetime".to_string(),
                    user: GHPullRequestOwner { login: "me".to_string() },
                },
            ])
        };

        let results = sync_comments("org/repo", "", &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Added Jira Comment on ticket https://jira.domain/browse/A-1 from https://url/org/repo/1.".to_string(), "PR https://url/org/repo/2 does not contain a Jira ticket!".to_string(), "PR https://url/org/repo/3 does not contain a Jira ticket!".to_string()]);
    }

    #[test]
    fn dedups_existing_comments() {
        let jira_client = MockJiraClient {
            domain: "jira.domain".to_string(),
            data: Box::new(JiraCommentResponse {
                total: 2,
                comments: vec![
                    JiraComment {
                        rendered_body: "asdfageta https://url/org/repo/1 asdadf".to_string()
                    },
                    JiraComment {
                        rendered_body: "aeradadf asafsd asd ".to_string()
                    },
                ],
            })
        };

        let gh_client = MockGithubClient {
            data: Box::new(vec![
                GHPullRequest {
                    base: GHPullRequestBase { repo: GHRepo { full_name: "org/repo".to_string() } },
                    html_url: "https://url/org/repo/1".to_string(),
                    title: "test title".to_string(),
                    body: Some("test body [A-1](https://jira.domain/asdf)".to_string()),
                    created_at: "datetime".to_string(),
                    user: GHPullRequestOwner { login: "me".to_string() },
                },
            ])
        };

        let results = sync_comments("org/repo", "", &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Jira ticket https://jira.domain/browse/A-1 already has comment for https://url/org/repo/1.".to_string()]);
    }

    #[test]
    fn test_no_prs() {
        let jira_client = MockJiraClient {
            domain: "jira.domain".to_string(),
            data: Box::new(JiraCommentResponse {
                total: 2,
                comments: vec![
                    JiraComment {
                        rendered_body: "asdfageta https://url/org/repo/1 asdadf".to_string()
                    },
                    JiraComment {
                        rendered_body: "aeradadf asafsd asd ".to_string()
                    },
                ],
            })
        };

        let gh_client = MockGithubClient {
            data: Box::new(Vec::new())
        };

        let results = sync_comments("org/repo", "", &gh_client, &jira_client).unwrap();

        assert_eq!(results, Vec::<String>::new());
    }

    #[test]
    fn test_no_comments() {
        let jira_client = MockJiraClient {
            domain: "jira.domain".to_string(),
            data: Box::new(JiraCommentResponse {
                total: 0,
                comments: Vec::new(),
            })
        };

        let gh_client = MockGithubClient {
            data: Box::new(vec![
                GHPullRequest {
                    base: GHPullRequestBase { repo: GHRepo { full_name: "org/repo".to_string() } },
                    html_url: "https://url/org/repo/1".to_string(),
                    title: "test title".to_string(),
                    body: Some("test body [A-1](https://jira.domain/asdf)".to_string()),
                    created_at: "datetime".to_string(),
                    user: GHPullRequestOwner { login: "me".to_string() },
                },
            ])
        };

        let results = sync_comments("org/repo", "", &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Added Jira Comment on ticket https://jira.domain/browse/A-1 from https://url/org/repo/1.".to_string()]);
    }

    #[test]
    fn take_until_empty_string() {
        assert_eq!("".take_until('1'), "");
    }

    #[test]
    fn take_until_non_empty_string() {
        assert_eq!("asdf\nasdf".take_until('\n'), "asdf");
    }

    #[test]
    fn take_until_no_match() {
        assert_eq!("asdf\tasdf".take_until('\n'), "asdf\tasdf");
    }
}