mod error;
pub mod github;
pub mod jira;
pub mod credentials;

use error::Error;
use crate::github::GHPullRequest;

pub fn sync_comments(repo: String, filters: String, gh_client: &dyn github::GithubClient, jira_client: &dyn jira::JiraClient) -> Result<Vec<String>, Error> {
    gh_client.get_pull_requests_for_repo(repo, filters)?.iter()
        .map(|pr| process_pull_request(jira_client, pr))
        .collect()
}

fn process_pull_request(jira_client: &dyn jira::JiraClient, pr: &GHPullRequest) -> Result<String, Error> {
    let pr_body = pr.body.clone()
        .ok_or(Error::AutocommentError(format!("PR {} does not have a description!", pr.html_url.clone())))?;

    if let Some(jira_id) = jira::parse_jira_ticket_number(pr_body, jira_client.get_domain()) {
        let comments = jira_client.get_jira_comments(jira_id.clone())?;
        if !comments.contains_text(pr.html_url.clone()) {
            jira_client.post_jira_comment(jira_id.clone(), pr.build_jira_comment()?)
                .map(|_| format!("Added Jira Comment on ticket {} from {}.", jira_id, pr.html_url.clone()))
        } else {
            Ok(format!("Jira ticket {} already has comment for {}.", jira_id, pr.html_url.clone()))
        }
    } else {
        Ok(format!("PR {} does not contain a Jira ticket!", pr.html_url.clone()))
    }
}

trait TakeUntil<T> {
    fn take_until(&self, limit: char) -> T;
}

impl TakeUntil<Self> for String {
    fn take_until(&self, limit: char) -> Self {
        if let Some(idx) = self.find(limit) {
            return self[..idx].to_string();
        }
        self.clone()
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

        let results = sync_comments("org/repo".to_string(), "".to_string(), &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Added Jira Comment on ticket A-1 from https://url/org/repo/1.".to_string(), "PR https://url/org/repo/2 does not contain a Jira ticket!".to_string(), "PR https://url/org/repo/3 does not contain a Jira ticket!".to_string()]);
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

        let results = sync_comments("org/repo".to_string(), "".to_string(), &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Jira ticket A-1 already has comment for https://url/org/repo/1.".to_string()]);
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

        let results = sync_comments("org/repo".to_string(), "".to_string(), &gh_client, &jira_client).unwrap();

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

        let results = sync_comments("org/repo".to_string(), "".to_string(), &gh_client, &jira_client).unwrap();

        assert_eq!(results, vec!["Added Jira Comment on ticket A-1 from https://url/org/repo/1.".to_string()]);
    }

    #[test]
    fn take_until_empty_string() {
        assert_eq!("".to_string().take_until('1'), "".to_string());
    }

    #[test]
    fn take_until_non_empty_string() {
        assert_eq!("asdf\nasdf".to_string().take_until('\n'), "asdf".to_string());
    }

    #[test]
    fn take_until_no_match() {
        assert_eq!("asdf\tasdf".to_string().take_until('\n'), "asdf\tasdf".to_string());
    }
}