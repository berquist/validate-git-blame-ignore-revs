use clap::Parser;
use std::path::PathBuf;

use validate_git_blame_ignore_revs::validate_git_blame_ignore_revs;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the .git-blame-ignore-revs file
    file_path: PathBuf,

    /// Ensure each commit is in the history of the checked-out branch
    #[arg(long)]
    call_git: bool,

    /// Require each commit line to have one or more comment lines above it
    #[arg(long)]
    strict_comments: bool,

    /// Ensure the comment above each commit matches the first part of the
    /// commit message. Requires --strict-comments and --call-git
    #[arg(long)]
    strict_comments_git: bool,

    /// Ensure all commits authored by pre-commit-ci[bot] are present in the
    /// file. Requires --call-git
    #[arg(long)]
    pre_commit_ci: bool,
}

fn main() {
    let cli = Cli::parse();

    let file_path = cli.file_path;
    let call_git = cli.call_git;
    let strict_comments = cli.strict_comments;
    let strict_comments_git = cli.strict_comments_git;
    let pre_commit_ci = cli.pre_commit_ci;

    if strict_comments_git && !(strict_comments && call_git) {
        eprintln!("Error: --strict-comments-git requires --strict-comments and --call-git.");
        return;
    }

    if pre_commit_ci && !call_git {
        eprintln!("Error: --pre-commit-ci requires --call-git.");
        return;
    }

    match validate_git_blame_ignore_revs(
        &file_path,
        call_git,
        strict_comments,
        strict_comments_git,
        pre_commit_ci,
    ) {
        Ok(result) => {
            println!("Validation Results:");
            println!("Valid hashes ({}):", result.valid_hashes.len());
            for (line_number, hash) in &result.valid_hashes {
                println!("  Line {}: {}", line_number, hash);
            }

            if !result.errors.is_empty() {
                println!("\nErrors ({}):", result.errors.len());
                for (line_number, line) in &result.errors {
                    println!("  Line {}: {}", line_number, line);
                }
            } else {
                println!("\nNo errors found!");
            }

            if call_git {
                if !result.missing_commits.is_empty() {
                    println!("\nMissing commits ({}):", result.missing_commits.len());
                    for (line_number, commit) in &result.missing_commits {
                        println!("  Line {}: {}", line_number, commit);
                    }
                } else {
                    println!("\nAll commits are present in the Git history!");
                }
            }

            if strict_comments {
                if !result.strict_comment_errors.is_empty() {
                    println!(
                        "\nStrict comment errors ({}):",
                        result.strict_comment_errors.len()
                    );
                    for (line_number, line) in &result.strict_comment_errors {
                        println!("  Line {}: {}", line_number, line);
                    }
                } else {
                    println!("\nAll commit lines have comments above them!");
                }
            }

            if strict_comments_git {
                if !result.comment_diffs.is_empty() {
                    println!("\nComment diffs ({}):", result.comment_diffs.len());
                    for (line_number, (comment, commit_message)) in &result.comment_diffs {
                        println!("  Line {}:", line_number);
                        println!("    Comment: {}", comment);
                        println!("    Commit message: {}", commit_message);
                    }
                } else {
                    println!("\nAll comments match the corresponding commit messages!");
                }
            }

            if pre_commit_ci {
                if !result.missing_pre_commit_ci_commits.is_empty() {
                    println!(
                        "\nMissing pre-commit-ci commits ({}):",
                        result.missing_pre_commit_ci_commits.len()
                    );
                    for (commit_hash, commit_message) in &result.missing_pre_commit_ci_commits {
                        println!("  Commit {}: {}", commit_hash, commit_message);
                    }
                } else {
                    println!("\nAll pre-commit-ci commits are present in the file!");
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
