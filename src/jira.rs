use crate::credentials::Credentials;
use crate::error::Error;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};


/// Representation of a Jira comment response, with only
/// the fields necessary to parse a comment's body.
#[derive(Serialize, Deserialize, Clone)]
pub struct JiraCommentResponse {
    pub total: i32,
    pub comments: Vec<JiraComment>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraComment {
    #[serde(rename = "renderedBody")]
    pub rendered_body: String,
}

impl JiraCommentResponse {
    pub fn contains_text(&self, text: String) -> bool {
        self.comments.iter().any(|comment| comment.rendered_body.contains(&text))
    }
}

pub trait JiraClient {
    fn post_jira_comment(&self, ticket_id: String, text: String) -> Result<(), Error>;
    fn get_jira_comments(&self, ticket_id: String) -> Result<JiraCommentResponse, Error>;
}

pub struct DefaultJiraClient<'a> {
    client: Client,
    creds: &'a Credentials,
}

impl<'a> DefaultJiraClient<'a> {
    pub fn new(creds: &'a Credentials) -> DefaultJiraClient<'a> {
        let client = Client::new();
        DefaultJiraClient { client, creds }
    }
}

impl<'a> JiraClient for DefaultJiraClient<'a> {
    fn post_jira_comment(&self, ticket_id: String, text: String) -> Result<(), Error> {
        let jira_url = format!("https://{}/rest/api/3/issue/{}/comment?expand=renderedBody", self.creds.jira_domain, ticket_id);
        let resp = self.client.post(jira_url)
            .basic_auth(self.creds.jira_user.clone(), Some(self.creds.jira_pass.clone()))
            .header("Content-Type", "application/json")
            .body(text)
            .send()?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::from("Unable to post Jira comment: ".to_owned() + &resp.status().to_string()))
        }
    }

    fn get_jira_comments(&self, ticket_id: String) -> Result<JiraCommentResponse, Error> {
        let jira_url = format!("https://{}/rest/api/3/issue/{}/comment?expand=renderedBody", self.creds.jira_domain, ticket_id);

        let resp = self.client.get(jira_url)
            .basic_auth(self.creds.jira_user.clone(), Some(self.creds.jira_pass.clone()))
            .send()?;

        if resp.status().is_success() {
            Ok(serde_json::from_str(resp.text()?.as_str())?)
        } else {
            Err(Error::from(resp.text()?))
        }
    }
}

pub fn parse_jira_ticket_number(pr_body: String) -> Option<String> {
    let re = regex::Regex::new(r"\[(\w+\-\d+)\]\(https://taserintl\.atlassian\.net\S+\)").unwrap();
    for group in re.captures_iter(pr_body.as_str()) {
        // Return the first matching ticket
        return Some(group[1].to_string());
    }
    None
}

pub struct MockJiraClient {
    pub data: Box<JiraCommentResponse>
}

impl JiraClient for MockJiraClient {
    fn post_jira_comment(&self, ticket_id: String, text: String) -> Result<(), Error> {
        Ok(())
    }

    fn get_jira_comments(&self, ticket_id: String) -> Result<JiraCommentResponse, Error> {
        Ok(*self.data.clone())
    }
}

#[cfg(test)]
mod test {
    use crate::jira::{JiraComment, JiraCommentResponse, parse_jira_ticket_number};

    #[test]
    fn jira_comment_contains_text_true() {
        let resp = JiraCommentResponse {
            total: 2,
            comments: vec![
                JiraComment {
                    rendered_body: "asdfas asdf asdf ads".to_string()
                },
                JiraComment {
                    rendered_body: "asdf asdf afsd adfs https://url/org/repo asdfasdf".to_string()
                },
            ]
        };
        assert!(resp.contains_text("https://url/org/repo".to_string()))
    }

    #[test]
    fn jira_comment_contains_text_no_comments() {
        let resp = JiraCommentResponse {
            total: 0,
            comments: Vec::new()
        };
        assert!(!resp.contains_text("https://url/org/repo".to_string()))
    }

    #[test]
    fn jira_comment_contains_text_false() {
        let resp = JiraCommentResponse {
            total: 2,
            comments: vec![
                JiraComment {
                    rendered_body: "asdf asdf afsd adfs https://url/org/otherrepo asdfasdf".to_string()
                },
                JiraComment {
                    rendered_body: "asdfas asdf asdf ads".to_string()
                }
            ]
        };
        assert!(!resp.contains_text("https://url/org/repo".to_string()))
    }

    #[test]
    fn parse_jira_ticket_number_with_match() {
        assert_eq!(parse_jira_ticket_number("dsaaerl; are aerg \nasfwqrwrv\nasdfawfr\t[CEC-123](https://taserintl.atlassian.net/asdf) asdfar w\nasdf".to_string()).unwrap(), "CEC-123".to_string())
    }

    #[test]
    fn parse_jira_ticket_number_no_match() {
        assert!(parse_jira_ticket_number("dsaaerl; are aerg \nasfwqrwrv\nasdfawfr\tasdfar w\nasdf".to_string()).is_none())
    }
}