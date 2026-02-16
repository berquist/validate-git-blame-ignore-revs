use clap::Parser;
use regex::Regex;
use std::{collections::HashMap, env, fs, path::PathBuf, process::Command as ProcessCommand};

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
        env::set_current_dir(file_path.parent().unwrap()).unwrap();

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
    use std::path::Path;

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

        // The second "isort" commit doesn't have a comment.
        let expected_strict_comment_errors: HashMap<usize, String> =
            [(16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string())]
                .iter()
                .cloned()
                .collect();

        assert_eq!(result.strict_comment_errors, expected_strict_comment_errors);
    }

    //  We don't bother with --call-git alone because all of the commit refs
    //  are known to be present in the repository.

    #[test]
    fn test_validate_git_blame_ignore_revs_strict_comments_git() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let repo_loc = setup_repo_cclib(temp_dir.path());

        let opts = Opts {
            file_path: repo_loc.join(".git-blame-ignore-revs"),
            call_git: true,
            strict_comments: true,
            strict_comments_git: true,
            pre_commit_ci: false,
        };
        let result = validate_git_blame_ignore_revs(&opts).expect("Validation failed");

        assert!(result.errors.is_empty());
        assert!(result.missing_commits.is_empty());
        assert!(!result.strict_comment_errors.is_empty());
        assert!(!result.comment_diffs.is_empty());
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

        // The second "isort" commit doesn't have a comment.
        let expected_strict_comment_errors: HashMap<usize, String> =
            [(16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string())]
                .iter()
                .cloned()
                .collect();

        assert_eq!(result.strict_comment_errors, expected_strict_comment_errors);

        let expected_comment_diff_errors: HashMap<usize, (String, String)> = [
            (
                5,
                (
                    "remove trailing whitespace".to_string(),
                    "pre-commit: fix trailing-whitespace".to_string(),
                ),
            ),
            (
                7,
                (
                    "fix end of file".to_string(),
                    "pre-commit: end-of-file-fixer".to_string(),
                ),
            ),
            (
                9,
                (
                    "fix line endings to be LF".to_string(),
                    "make all line endings LF".to_string(),
                ),
            ),
            (
                11,
                (
                    "ruff for all but cclib/parser".to_string(),
                    "apply ruff to all but cclib/parser".to_string(),
                ),
            ),
            (
                13,
                (
                    "ruff for cclib/parser".to_string(),
                    "apply ruff to cclib/parser".to_string(),
                ),
            ),
            (15, ("isort".to_string(), "apply isort".to_string())),
            // TODO fix this side effect of --strict-comments
            (
                16,
                (
                    "3c499da3e44e4a8ae983407c0f07c71996d202d8".to_string(),
                    "isort: fix for circular import".to_string(),
                ),
            ),
            (
                18,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                20,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                22,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                24,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                26,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(result.comment_diffs, expected_comment_diff_errors);
    }

    #[test]
    fn test_validate_git_blame_ignore_revs_all() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let repo_loc = setup_repo_cclib(temp_dir.path());

        let opts = Opts {
            file_path: repo_loc.join(".git-blame-ignore-revs"),
            call_git: true,
            strict_comments: true,
            strict_comments_git: true,
            pre_commit_ci: true,
        };
        let result = validate_git_blame_ignore_revs(&opts).expect("Validation failed");

        assert!(result.errors.is_empty());
        assert!(result.missing_commits.is_empty());
        assert!(!result.strict_comment_errors.is_empty());
        assert!(!result.comment_diffs.is_empty());
        assert!(!result.missing_pre_commit_ci_commits.is_empty());

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

        // The second "isort" commit doesn't have a comment.
        let expected_strict_comment_errors: HashMap<usize, String> =
            [(16, "06e54bd1eb663dd948973037f336d5190f4734ce".to_string())]
                .iter()
                .cloned()
                .collect();

        assert_eq!(result.strict_comment_errors, expected_strict_comment_errors);

        let expected_comment_diff_errors: HashMap<usize, (String, String)> = [
            (
                5,
                (
                    "remove trailing whitespace".to_string(),
                    "pre-commit: fix trailing-whitespace".to_string(),
                ),
            ),
            (
                7,
                (
                    "fix end of file".to_string(),
                    "pre-commit: end-of-file-fixer".to_string(),
                ),
            ),
            (
                9,
                (
                    "fix line endings to be LF".to_string(),
                    "make all line endings LF".to_string(),
                ),
            ),
            (
                11,
                (
                    "ruff for all but cclib/parser".to_string(),
                    "apply ruff to all but cclib/parser".to_string(),
                ),
            ),
            (
                13,
                (
                    "ruff for cclib/parser".to_string(),
                    "apply ruff to cclib/parser".to_string(),
                ),
            ),
            (15, ("isort".to_string(), "apply isort".to_string())),
            // TODO fix this side effect of --strict-comments
            (
                16,
                (
                    "3c499da3e44e4a8ae983407c0f07c71996d202d8".to_string(),
                    "isort: fix for circular import".to_string(),
                ),
            ),
            (
                18,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                20,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                22,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                24,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
            (
                26,
                (
                    "pre-commit autofix".to_string(),
                    "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
                ),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(result.comment_diffs, expected_comment_diff_errors);

        let expected_missing_pre_commit_ci_commits = [
            (
                "038bbefc86b090ed18572e35665b939e45215ea7".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "089c6ee49a9c6035046979dfe665ed105b346dbc".to_string(),
                "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
            ),
            (
                "0b06cb3fd5563978fa361edc42f1a49a81ac457c".to_string(),
                "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
            ),
            (
                "0f290e88405383f4a2f9dce7b3e67e958ea0212a".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "0f4d7afbe9ca65f9c4d102f92eecacc78e76db6c".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "106dab76ebb1ed6e4737319ad67e0abe5f233146".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "14878b2f6508a0fabb4b09fd0d09cb1db3f6b656".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "15624b49b0a7ab8fab50843d1ff2e969962e6aad".to_string(),
                "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
            ),
            (
                "186817fcd59bc253a432f73a2ed36ea7679b7991".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "21c3289ff45fb65746d5795edfc1515fc22822b1".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "2590884a2d8a1ded2492b9bb145b19cc3434d7b7".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "25ef9af951ef7195c3b26fdae7949cb76ec0cea1".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "2e984315d3ea4bc9f5532e8db078a0affda428b3".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "2f2262f35d7d8d18852cce48ccac0629fd630d67".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "33f470c73f582bb6a0d127a115bb3ae404eb9731".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "3b4b4a6dbe7dd7ffc418bdf3e4300209ad2525e0".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "54bdc19452c9107cddad6090f5f9b20fb88cae44".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "5a926059ffd91c555e76e4ab113324b203b2214c".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "5e3393e87c8cfa80a923c6318f804e89036a69e0".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "635d13f7ca0c74623356661c851768b6f3afa151".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "67b0f630cb0894a3f77f1d26cb06d568a8cd1e2e".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "688d3e428552100ce2bf584c8beb9c7a89d9b6d6".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "79e919100300bdeddcfe27caaab55a43b6ec0719".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "7cd56513313c7534523c219d6a7ef44bc010e241".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "7faac65ddec1a342011c026c52bdd462569c432b".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8261954e2f0cb6e7e896e4139b18d588fe911508".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "849c8292910fbb74449fa42e0f95520081c71388".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8748529bc89e35b83e1dd34431fe91b3d4ba57e5".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8b6396ad1f06ee3f777f3fdd041075bc680f2a31".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8bc987191430e5a880bc69af8decc95155ade3e2".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8ccbeb072413ca9dda3a0da88d54f4ed55ab59b7".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "8e0410ac9e8b85ed17ed687227b8ca4e474234f5".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "900a6eef07ad9fc4a03ebfb5de4327cd0842fb66".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "93c0492f0d05f6fcc40fa5874c0f4ed38099bbff".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "981fe13cfec0d7a0d22eae4c4ee276dd9face9ee".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "9fd1fdc0e3cb3d85336b81fa48ddd708170bb31b".to_string(),
                "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
            ),
            (
                "a54c1eb8ce367677c5f35ab1b4f70de2b0660282".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "a856068ba0b9ae46d1d6f52681b49b0297de2ca5".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "adde67a3e7b5f12d1051cb41535e15e63a3f9c9b".to_string(),
                "Update license year from 2024 to 2025".to_string(),
            ),
            (
                "b1ed6e2cd427dccc5c875dfc4b0d90d0e6e850f3".to_string(),
                "[pre-commit.ci] auto fixes from pre-commit.com hooks".to_string(),
            ),
            (
                "b892a126f0d1d1ca37c62495a89de75feb49ccf7".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "bc00b002eafcaace2024f33e64291ea37181c278".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "c1d280e0fdee1473b6ee50d0c9e0e4eef609acff".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "c2a633c36d67cdf83486b9bc41151490d5a60291".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "c2f215794365df9c3e447bfbb0c008d6f755e139".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "d08971e85461bcf32f6562d6b86eac48e652fa13".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "d24f91ac92be4bd73a5271dda1a7a2c7059ec5e2".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "d51a21a6a63dc368aeea8004851f9c63607b4a8c".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "d8bae46f30390f43c868d0d28119998a87a7d440".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "dc90f9920f43dfb8f300a35e8d3cc670c21606ac".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "e1841cd47230a6c1f48c7d44e0e1ef6d7c1c12b4".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "e66b13a2157fc3f071e9c03ec7b897f118753062".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "ea847a6d5c9508408f0eb7a741fe5f1414e015a7".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "eb166a9c655810e550ce98216bc83becb0de2404".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "ebb0e7b996d2d1c8c4b037634e6311fedc9f1930".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "ec118b5f4412a340a19ed2b183fd9a1bfb8e2430".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "f253f860459e578e56aa10adf7edacdc2c98e778".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "f508805db96c1ce60c628b62d38fbb0f4979da8f".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "f7c8c8a33dd1f9ada079acdf4cf369d86632bf07".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
            (
                "f84e9d4ae7f757877a8d30e1feffd8394d11b213".to_string(),
                "[pre-commit.ci] pre-commit autoupdate".to_string(),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(
            result.missing_pre_commit_ci_commits,
            expected_missing_pre_commit_ci_commits
        );
    }
}
