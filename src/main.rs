use clap::{Parser, Subcommand};
use autocomment::{sync_comments, Error, Credentials, DefaultGithubClient, DefaultJiraClient};

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

        /// Filters to pass to Github when querying repos. Try state=open for open PR's
        #[arg(short, long)]
        filter: Option<String>,
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
            Commands::Sync { repo, filter } => {
                if let Ok(creds) = Credentials::from_env() {
                    let mut filters = String::new();

                    if let Some(querystring) = filter {
                        filters = "?".to_owned() + querystring;
                    }

                    let gh_client = DefaultGithubClient::new(&creds);
                    let jira_client = DefaultJiraClient::new(&creds);

                    if let Some(err) = sync_comments(repo, &filters, &gh_client, &jira_client).err() {
                        match err {
                            Error::AutocommentError(err) => println!("Unable to save credentials: {}", err),
                            Error::SerdeYamlError(err) => println!("Error occurred while saving config file: {}", err.to_string()),
                            Error::FsError(err) => println!("Error occurred while reading files: {}", err.to_string()),
                            Error::ReqwestError(err) => println!("Network error occurred: {}", err.to_string()),
                            Error::SerdeJsonError(err) => println!("Unable to read response: {}", err.to_string()),
                        }
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
                    match err {
                        Error::AutocommentError(err) => println!("Unable to save credentials: {}", err),
                        Error::SerdeYamlError(err) => println!("Error occurred while saving config file: {}", err.to_string()),
                        Error::FsError(err) => println!("Error occurred while reading files: {}", err.to_string()),
                        _ => println!("Unknown error occurred!")
                    }
                }
            }
        }
    }
}
