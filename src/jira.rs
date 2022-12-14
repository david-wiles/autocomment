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

impl JiraCommentResponse {
    pub fn contains_text(&self, text: &str) -> bool {
        self.comments.iter().any(|comment| comment.rendered_body.contains(&text))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraComment {
    #[serde(rename = "renderedBody")]
    pub rendered_body: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraCommentRequest {
    pub body: JiraCommentElement
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraCommentElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,

    #[serde(rename = "type")]
    pub comment_type: String,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<JiraCommentElement>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<JiraCommentElement>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<JiraCommentAttrs>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraCommentAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    href: Option<String>
}

impl JiraCommentElement {
    pub fn doc(content: Vec<JiraCommentElement>) -> Self {
        JiraCommentElement {
            version: Some(1),
            comment_type: "doc".to_string(),
            content: content,
            text: None,
            marks: Vec::new(),
            attrs: None
        }
    }

    pub fn text(text: String) -> Self {
        JiraCommentElement {
            version: None,
            comment_type: "text".to_string(),
            content: Vec::new(),
            text: Some(text),
            marks: Vec::new(),
            attrs: None
        }
    }

    pub fn paragraph(content: Vec<JiraCommentElement>) -> Self {
        JiraCommentElement {
            version: None,
            comment_type: "paragraph".to_string(),
            content: content,
            text: None,
            marks: Vec::new(),
            attrs: None
        }
    }

    pub fn link(text: String, link: String) -> Self {
        JiraCommentElement {
            version: None,
            comment_type: "text".to_string(),
            content: Vec::new(),
            text: Some(text),
            marks: vec![JiraCommentElement {
                version: None,
                comment_type: "link".to_string(),
                content: Vec::new(),
                text: None,
                marks: Vec::new(),
                attrs: Some(JiraCommentAttrs { href: Some(link) })
            }],
            attrs: None
        }
    }
}

pub trait JiraClient {
    fn get_domain(&self) -> &str;
    fn post_jira_comment(&self, ticket_id: &str, text: &str) -> Result<(), Error>;
    fn get_jira_comments(&self, ticket_id: &str) -> Result<JiraCommentResponse, Error>;
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
    fn get_domain(&self) -> &str {
        self.creds.jira_domain.as_str()
    }

    fn post_jira_comment(&self, ticket_id: &str, text: &str) -> Result<(), Error> {
        let jira_url = format!("https://{}/rest/api/3/issue/{}/comment?expand=renderedBody", self.creds.jira_domain, ticket_id);
        let resp = self.client.post(jira_url)
            .basic_auth(self.creds.jira_user.clone(), Some(self.creds.jira_pass.clone()))
            .header("Content-Type", "application/json")
            .body(text.to_string())
            .send()?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::from("Unable to post Jira comment: ".to_owned() + &resp.status().to_string()))
        }
    }

    fn get_jira_comments(&self, ticket_id: &str) -> Result<JiraCommentResponse, Error> {
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

pub fn parse_jira_ticket_number(pr_body: &str, domain: &str) -> Option<String> {
    let re = regex::Regex::new(format!(r"\[(\w+\-\d+)\]\(https://{}\S+\)", domain.replace(".", r"\.")).as_str()).unwrap();
    for group in re.captures_iter(pr_body) {
        // Return the first matching ticket
        return Some(group[1].to_string());
    }
    None
}

pub struct MockJiraClient {
    pub domain: String,
    pub data: Box<JiraCommentResponse>
}

impl JiraClient for MockJiraClient {
    fn get_domain(&self) -> &str {
        self.domain.as_str()
    }

    fn post_jira_comment(&self, _ticket_id: &str, _text: &str) -> Result<(), Error> {
        Ok(())
    }

    fn get_jira_comments(&self, _ticket_id: &str) -> Result<JiraCommentResponse, Error> {
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
        assert!(resp.contains_text("https://url/org/repo"))
    }

    #[test]
    fn jira_comment_contains_text_no_comments() {
        let resp = JiraCommentResponse {
            total: 0,
            comments: Vec::new()
        };
        assert!(!resp.contains_text("https://url/org/repo"))
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
        assert!(!resp.contains_text("https://url/org/repo"))
    }

    #[test]
    fn parse_jira_ticket_number_with_match() {
        assert_eq!(parse_jira_ticket_number("dsaaerl; are aerg \nasfwqrwrv\nasdfawfr\t[CEC-123](https://jira.domain/asdf) asdfar w\nasdf", "jira.domain").unwrap(), "CEC-123".to_string())
    }

    #[test]
    fn parse_jira_ticket_number_no_match() {
        assert!(parse_jira_ticket_number("dsaaerl; are aerg \nasfwqrwrv\nasdfawfr\tasdfar w\nasdf", "jira.domain").is_none())
    }
}