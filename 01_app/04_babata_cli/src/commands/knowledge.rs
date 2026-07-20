use std::path::PathBuf;

use babata_application::ApplicationError;

#[derive(Debug, clap::Subcommand)]
pub enum KnowledgeCommand {
    Review {
        #[arg(long)]
        item: String,
        #[arg(long)]
        revision: String,
    },
    Create {
        #[arg(long)]
        source_revision: String,
        #[arg(long, default_value = "user")]
        author: String,
        #[command(flatten)]
        content: KnowledgeContentInput,
    },
    Revise {
        #[arg(long)]
        knowledge: String,
        #[arg(long)]
        note: Option<String>,
        #[command(flatten)]
        content: KnowledgeContentInput,
    },
    Show {
        #[arg(long)]
        knowledge: String,
    },
}

#[derive(Debug, clap::Args)]
pub struct KnowledgeContentInput {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub path: Option<PathBuf>,
}

pub fn read_content(input: KnowledgeContentInput) -> Result<(String, String), ApplicationError> {
    let body = match (input.text, input.path) {
        (Some(text), None) => text,
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|error| {
            ApplicationError::Asset(format!("unable to read knowledge text: {:?}", error.kind()))
        })?,
        _ => {
            return Err(ApplicationError::Conflict(
                "provide exactly one of --text or --path".to_owned(),
            ));
        }
    };
    Ok((input.title, body))
}
