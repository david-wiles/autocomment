use crate::credentials::Credentials;
use crate::error::Error;
use crate::github;
use crate::Take;

use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};


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

pub fn post_jira_comment(ticket_id: String, pr: &github::GHPullRequest, cred: &Credentials) -> Result<String, Error> {
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

pub fn jira_ticket_has_existing_comment(ticket_id: String, pr: &github::GHPullRequest, cred: &Credentials) -> Result<bool, Error> {
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

pub fn parse_jira_ticket_number(pr_body: String) -> Option<String> {
    let re = regex::Regex::new(r"\[(\w+\-\d+)\]\(https://taserintl\.atlassian\.net\S+\)").unwrap();
    for group in re.captures_iter(pr_body.as_str()) {
        // Return the first matching ticket
        return Some(group[1].to_string());
    }
    None
}
