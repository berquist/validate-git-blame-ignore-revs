use clap::{Arg, Command};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::str;

type HashEntries = HashMap<usize, String>;

#[derive(Debug)]
struct ValidationResult {
    valid_hashes: HashEntries,
    errors: HashEntries,
    missing_commits: HashEntries,
    strict_comment_errors: HashEntries,
    comment_diffs: HashMap<usize, (String, String)>, // Line number -> (comment, commit message)
    missing_pre_commit_ci_commits: HashMap<String, String>, // Commit hash -> Commit message
}

fn validate_git_blame_ignore_revs(
    file_path: &str,
    call_git: bool,
    strict_comments: bool,
    strict_comments_git: bool,
    pre_commit_ci: bool,
) -> Result<ValidationResult, String> {
    if !Path::new(file_path).exists() {
        return Err(format!("The file '{}' does not exist.", file_path));
    }

    let mut valid_hashes: HashEntries = HashMap::new();
    let mut errors: HashEntries = HashMap::new();
    let mut missing_commits: HashEntries = HashMap::new();
    let mut strict_comment_errors: HashEntries = HashMap::new();
    let mut comment_diffs: HashMap<usize, (String, String)> = HashMap::new();
    let mut missing_pre_commit_ci_commits: HashMap<String, String> = HashMap::new();

    let commit_hash_regex = Regex::new(r"^[0-9a-f]{40}$").unwrap();

    let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();

    let mut has_comment_above = false;
    let mut last_comment = None;

    for (line_number, line) in lines.iter().enumerate() {
        let line_number = line_number + 1; // Convert to 1-based indexing
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with('#') {
            has_comment_above = true;
            last_comment = Some(line.trim_start_matches('#').trim().to_string());
            continue;
        }

        if commit_hash_regex.is_match(line) {
            valid_hashes.insert(line_number, line.to_string());

            if strict_comments && !has_comment_above {
                strict_comment_errors.insert(line_number, line.to_string());
            }

            has_comment_above = false;
            last_comment = None;
        } else {
            errors.insert(line_number, line.to_string());
        }
    }

    if call_git || strict_comments_git {
        for (line_number, commit_hash) in &valid_hashes {
            let output = ProcessCommand::new("git")
                .args(&["show", "--quiet", "--pretty=format:%H %s", commit_hash])
                .output();

            match output {
                Ok(result) => {
                    let git_output = str::from_utf8(&result.stdout).unwrap_or("").trim();
                    if git_output.is_empty() {
                        missing_commits.insert(*line_number, commit_hash.clone());
                    } else {
                        let parts: Vec<&str> = git_output.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            let commit_message = parts[1];
                            if strict_comments_git {
                                let comment = if *line_number > 1 {
                                    lines[*line_number - 2].trim_start_matches('#').trim()
                                } else {
                                    ""
                                };
                                if !commit_message.starts_with(comment) {
                                    comment_diffs.insert(
                                        *line_number,
                                        (comment.to_string(), commit_message.to_string()),
                                    );
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    missing_commits.insert(*line_number, commit_hash.clone());
                }
            }
        }
    }

    if pre_commit_ci {
        let output = ProcessCommand::new("git")
            .args(&[
                "log",
                "--pretty=format:%H %s",
                "--author=pre-commit-ci[bot]",
            ])
            .output();

        match output {
            Ok(result) => {
                let pre_commit_ci_commits = str::from_utf8(&result.stdout).unwrap_or("").trim();
                for commit_entry in pre_commit_ci_commits.lines() {
                    let parts: Vec<&str> = commit_entry.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let commit_hash = parts[0];
                        let commit_message = parts[1];
                        if !valid_hashes.values().any(|hash| hash == commit_hash) {
                            missing_pre_commit_ci_commits
                                .insert(commit_hash.to_string(), commit_message.to_string());
                        }
                    }
                }
            }
            Err(_) => {
                return Err("Failed to fetch commits authored by pre-commit-ci[bot].".to_string());
            }
        }
    }

    Ok(ValidationResult {
        valid_hashes,
        errors,
        missing_commits,
        strict_comment_errors,
        comment_diffs,
        missing_pre_commit_ci_commits,
    })
}

fn main() {
    let matches = Command::new("validate_git_blame_ignore_revs")
        .version("1.0")
        .author("Eric John Berquist")
        .about("Validate a .git-blame-ignore-revs file")
        .arg(
            Arg::new("file_path")
                .required(true)
                .takes_value(true)
                .help("Path to the .git-blame-ignore-revs file"),
        )
        .arg(
            Arg::new("call_git")
                .long("call-git")
                .takes_value(false)
                .help("Ensure each commit is in the history of the checked-out branch"),
        )
        .arg(
            Arg::new("strict_comments")
                .long("strict-comments")
                .takes_value(false)
                .help("Require each commit line to have one or more comment lines above it"),
        )
        .arg(
            Arg::new("strict_comments_git")
                .long("strict-comments-git")
                .takes_value(false)
                .help("Ensure the comment above each commit matches the first part of the commit message. Requires --strict-comments and --call-git"),
        )
        .arg(
            Arg::new("pre_commit_ci")
                .long("pre-commit-ci")
                .takes_value(false)
                .help("Ensure all commits authored by pre-commit-ci[bot] are present in the file. Requires --call-git"),
        )
        .get_matches();

    let file_path = matches.value_of("file_path").unwrap();
    let call_git = matches.is_present("call_git");
    let strict_comments = matches.is_present("strict_comments");
    let strict_comments_git = matches.is_present("strict_comments_git");
    let pre_commit_ci = matches.is_present("pre_commit_ci");

    if strict_comments_git && !(strict_comments && call_git) {
        eprintln!("Error: --strict-comments-git requires --strict-comments and --call-git.");
        return;
    }

    if pre_commit_ci && !call_git {
        eprintln!("Error: --pre-commit-ci requires --call-git.");
        return;
    }

    match validate_git_blame_ignore_revs(
        file_path,
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
