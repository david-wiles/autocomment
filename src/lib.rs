mod error;
mod github;
mod jira;
pub mod credentials;

use error::Error;

pub fn sync_comments(repo: String, filters: String, creds: &credentials::Credentials) -> Result<Vec<String>, Error> {
    github::get_pull_requests_for_repo(repo, filters, creds)?.iter()
        .map(|pr| {
            if let Some(pr_body) = pr.body.clone() {
                if let Some(jira_id) = jira::parse_jira_ticket_number(pr_body) {
                    if !jira::jira_ticket_has_existing_comment(jira_id.clone(), pr, creds)? {
                        jira::post_jira_comment(jira_id, pr, creds)
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