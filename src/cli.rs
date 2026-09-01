use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "hod",
    version,
    about = "Hodstack makes coding agents more productive.",
    long_about = None,
    override_usage = "hod <skill>\n  hod <command> [options]",
    args_conflicts_with_subcommands = true,
    disable_help_subcommand = true,
    term_width = 0
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(value_name = "skill", help = "The name of the skill")]
    pub skill: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Write the files of this project, then start the skill init")]
    Init {
        #[arg(long, help = "Write over a project file that this program does not own")]
        force: bool,
    },
}
