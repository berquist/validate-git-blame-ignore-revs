use clap::Parser;
use regex::Regex;
use std::path::PathBuf;
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

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Opts {
    /// Path to the .git-blame-ignore-revs file
    pub file_path: PathBuf,

    /// Ensure each commit is in the history of the checked-out branch
    #[arg(long)]
    pub call_git: bool,

    /// Require each commit line to have one or more comment lines above it
    #[arg(long)]
    pub strict_comments: bool,

    /// Ensure the comment above each commit matches the first part of the
    /// commit message. Requires --strict-comments and --call-git
    #[arg(long)]
    pub strict_comments_git: bool,

    /// Ensure all commits authored by pre-commit-ci[bot] are present in the
    /// file. Requires --call-git
    #[arg(long)]
    pub pre_commit_ci: bool,
}

pub fn validate_git_blame_ignore_revs(opts: &Opts) -> Result<ValidationResult, String> {
    let file_path = &opts.file_path;
    let call_git = opts.call_git;
    let strict_comments = opts.strict_comments;
    let strict_comments_git = opts.strict_comments_git;
    let pre_commit_ci = opts.pre_commit_ci;

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
                .args(["show", "--quiet", "--pretty=format:%H %s", commit_hash])
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
            .args([
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
    use super::*;
    use std::path::PathBuf;

    fn clone_repository(url: &str, path: &Path, rev: Option<&str>) -> Result<(), String> {
        let path = path.to_str().unwrap();

        let output = ProcessCommand::new("git")
            .args(["clone", url, path])
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        if let Some(rev) = rev {
            let output = ProcessCommand::new("git")
                .args(["-C", path, "checkout", rev])
                .output()
                .map_err(|e| format!("Failed to execute git: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "Git checkout failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        Ok(())
    }

    fn setup_repo_cclib(temp_dir: &Path) -> PathBuf {
        let repo_url = "https://github.com/cclib/cclib.git";
        let repo_path = temp_dir.join("cclib");
        let commit_hash = "4557cf6d8e3eafdf76e80daa4b5de0bdc4d8ec2c";

        match clone_repository(repo_url, &repo_path, Some(commit_hash)) {
            Ok(_) => println!("Repository cloned successfully!"),
            Err(e) => eprintln!("Error: {}", e),
        }

        repo_path
    }

    #[test]
    fn test_validate_git_blame_ignore_revs_basic() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let repo_loc = setup_repo_cclib(temp_dir.path());

        let opts = Opts {
            file_path: repo_loc.join(".git-blame-ignore-revs"),
            call_git: false,
            strict_comments: false,
            strict_comments_git: false,
            pre_commit_ci: false,
        };
        let result = validate_git_blame_ignore_revs(&opts).expect("Validation failed");

        assert!(result.errors.is_empty());
        assert!(result.missing_commits.is_empty());
        assert!(result.strict_comment_errors.is_empty());
        assert!(result.comment_diffs.is_empty());
        assert!(result.missing_pre_commit_ci_commits.is_empty());

        let expected_valid_hashes: HashMap<usize, String> = [
            (2, "fb35435f66eeb8b4825f7022cc2ab315e5379483".to_string()),
            (5, "37740d43064bc13445b19ff2d3c5f1154f202896".to_string()),
            (7, "8f3fbf7d1fc060a3c8522343dd103604bd946e5d".to_string()),
            (9, "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5".to_string()),
            (11, "e986fd9bb37ca0113707f88f6fea7f2318671cdd".to_string()),
            (13, "04b3e4694e001e4b91457674a490b723e17af2b7".to_string()),
            (15, "3c499da3e44e4a8ae983407c0f07c71996d202d8".to_string()),
            (16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string()),
            (18, "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9".to_string()),
            (20, "0ae04b8fd098645aa3d4805abba311ccb86762dc".to_string()),
            (22, "57041a7cd327d4206d9c870b7d37acaa556596c6".to_string()),
            (24, "8243cb2c418f4f373c3a3e48ffa0c213dd77988e".to_string()),
            (26, "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(result.valid_hashes, expected_valid_hashes);
    }

    #[test]
    fn test_validate_git_blame_ignore_revs_strict_comments() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let repo_loc = setup_repo_cclib(temp_dir.path());

        let opts = Opts {
            file_path: repo_loc.join(".git-blame-ignore-revs"),
            call_git: false,
            strict_comments: true,
            strict_comments_git: false,
            pre_commit_ci: false,
        };
        let result = validate_git_blame_ignore_revs(&opts).expect("Validation failed");

        assert!(result.errors.is_empty());
        assert!(result.missing_commits.is_empty());
        assert!(!result.strict_comment_errors.is_empty());
        assert!(result.comment_diffs.is_empty());
        assert!(result.missing_pre_commit_ci_commits.is_empty());

        let expected_valid_hashes: HashMap<usize, String> = [
            (2, "fb35435f66eeb8b4825f7022cc2ab315e5379483".to_string()),
            (5, "37740d43064bc13445b19ff2d3c5f1154f202896".to_string()),
            (7, "8f3fbf7d1fc060a3c8522343dd103604bd946e5d".to_string()),
            (9, "7b2e8701dcb1a1a6c437919b185be78a35c3e2a5".to_string()),
            (11, "e986fd9bb37ca0113707f88f6fea7f2318671cdd".to_string()),
            (13, "04b3e4694e001e4b91457674a490b723e17af2b7".to_string()),
            (15, "3c499da3e44e4a8ae983407c0f07c71996d202d8".to_string()),
            (16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string()),
            (18, "7f407d1c7383a17cbe0995fc7bd65daf964f2eb9".to_string()),
            (20, "0ae04b8fd098645aa3d4805abba311ccb86762dc".to_string()),
            (22, "57041a7cd327d4206d9c870b7d37acaa556596c6".to_string()),
            (24, "8243cb2c418f4f373c3a3e48ffa0c213dd77988e".to_string()),
            (26, "47ee1d84e2325b15c9e5508d6d48ae1f49e507b3".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(result.valid_hashes, expected_valid_hashes);

        let expected_strict_comment_errors: HashMap<usize, String> =
            [(16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string())]
                .iter()
                .cloned()
                .collect();

        assert_eq!(result.strict_comment_errors, expected_strict_comment_errors);
    }
}
