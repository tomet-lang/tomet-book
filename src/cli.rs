use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tmtbook")]
#[command(author, version, about = "A standalone book & wiki generator for Tomet", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build the book from source documents into static HTML
    Build {
        /// Source directory containing Tomet documents
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Output directory for static site
        #[arg(short = 'd', short_alias = 'o', long, alias = "out-dir")]
        dest: Option<PathBuf>,
    },

    /// Run the local development server with LiveReload
    #[command(alias = "dev")]
    Serve {
        /// Source directory containing Tomet documents
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Output directory for static site
        #[arg(short = 'd', short_alias = 'o', long, alias = "out-dir")]
        dest: Option<PathBuf>,

        /// Port to bind the server on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },

    /// Initialize a new tmtbook.toml configuration in the directory
    Init {
        /// Directory to initialize in
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Title of the book
        #[arg(long)]
        title: Option<String>,
    },
}
