use regex::Regex;
use std::{collections::HashMap, fs, path::Path, process::Command as ProcessCommand};

type HashEntries = HashMap<usize, String>;

#[derive(Debug)]
pub struct ValidationResult {
    pub valid_hashes: HashEntries,
    pub errors: HashEntries,
    pub missing_commits: HashEntries,
    pub strict_comment_errors: HashEntries,
    pub comment_diffs: HashMap<usize, (String, String)>, // Line number -> (comment, commit message)
    pub missing_pre_commit_ci_commits: HashMap<String, String>, // Commit hash -> Commit message
}

pub fn validate_git_blame_ignore_revs(
    file_path: &Path,
    call_git: bool,
    strict_comments: bool,
    strict_comments_git: bool,
    pre_commit_ci: bool,
) -> Result<ValidationResult, String> {
    if !file_path.exists() {
        return Err(format!(
            "The file '{}' does not exist.",
            file_path.to_str().unwrap()
        ));
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

#[cfg(test)]
mod tests {
    // use super::*;
}
