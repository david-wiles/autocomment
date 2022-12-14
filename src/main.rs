use clap::{Parser, Subcommand};

use autocomment_rs::sync_comments;
use autocomment_rs::credentials::Credentials;

#[derive(Parser)]
#[command(name = "AutoComment")]
#[command(about = "Adds comments to Jira tickets based on Github PR's")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Syncs Jira comments with Github PR's
    Sync {
        /// Full name of the repository to scan
        #[arg(short, long)]
        repo: String,

        /// Only sync open PR's
        #[arg(short, long)]
        only_open: bool,
    },

    /// Updates Github or Jira credentials
    Credentials {
        /// Jira Username
        #[arg(long)]
        jira_user: Option<String>,

        /// Jira Password
        #[arg(long)]
        jira_pass: Option<String>,

        /// Jira Domain
        #[arg(long)]
        jira_domain: Option<String>,

        /// Github User
        #[arg(long)]
        github_user: Option<String>,

        /// Github Password
        #[arg(long)]
        github_pass: Option<String>,

        /// Github Domain
        #[arg(long)]
        github_domain: Option<String>,
    },
}

fn main() {
    let cli: Cli = Cli::parse();

    if let Some(cmd) = &cli.command {
        match cmd {
            Commands::Sync { repo, only_open } => {
                if let Ok(creds) = Credentials::from_env() {
                    let filters = if *only_open {
                        "?stat=open"
                    } else {
                        ""
                    };

                    match sync_comments(repo.clone(), filters.to_string(), &creds) {
                        Ok(comments) => comments.iter().for_each(|comment| println!("{}", comment)),
                        Err(err) => println!("{}", err)
                    }
                }
            }
            Commands::Credentials {
                jira_user,
                jira_pass,
                jira_domain,
                github_user,
                github_pass,
                github_domain,
            } => {
                // TODO password protect the credentials
                let mut creds = Credentials::from_env().unwrap_or(Credentials::default());

                if let Some(cred) = jira_user { creds.jira_user = cred.clone(); }
                if let Some(cred) = jira_pass { creds.jira_pass = cred.clone(); }
                if let Some(cred) = jira_domain { creds.jira_domain = cred.clone(); }
                if let Some(cred) = github_user { creds.github_user = cred.clone(); }
                if let Some(cred) = github_pass { creds.github_pass = cred.clone(); }
                if let Some(cred) = github_domain { creds.github_domain = cred.clone(); }

                if let Some(err) = creds.save().err() {
                    println!("{}", err)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

}
