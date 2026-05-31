use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::constants::cache_dir;
use crate::db::Source;

pub fn fetch_source(source: &Source) -> Result<PathBuf, String> {
    match source.r#type.as_str() {
        "github-public" => fetch_github_public(source),
        "github-private" => fetch_github_private(source),
        "local-folder" => fetch_local_folder(source),
        other => Err(format!("Unknown source type: {}", other)),
    }
}

fn fetch_github_public(source: &Source) -> Result<PathBuf, String> {
    let repo_url = source.repo_url.as_ref().ok_or_else(|| "github-public type requires repo_url".to_string())?;
    let branch = source.branch.as_deref().unwrap_or("main");

    // Compute repo folder name
    // e.g. https://github.com/WordPress/gutenberg.git -> WordPress--gutenberg
    let repo_name = clean_repo_name(repo_url);
    let clone_dir = cache_dir().join(&repo_name);

    fs::create_dir_all(cache_dir()).map_err(|e| e.to_string())?;

    if clone_dir.join(".git").exists() {
        // Repository exists, fetch and hard reset
        println!("Updating existing public repository at {:?}", clone_dir);
        let _ = Command::new("git")
            .args(&["fetch", "origin", branch, "--depth=1"])
            .current_dir(&clone_dir)
            .status()
            .map_err(|e| e.to_string())?;

        let reset_status = Command::new("git")
            .args(&["reset", "--hard", &format!("origin/{}", branch)])
            .current_dir(&clone_dir)
            .status()
            .map_err(|e| e.to_string())?;

        if !reset_status.success() {
            return Err(format!("Git reset failed for {}", source.name));
        }
    } else {
        // Shallow clone
        println!("Cloning public repository to {:?}", clone_dir);
        let status = Command::new("git")
            .args(&[
                "clone",
                repo_url,
                clone_dir.to_str().unwrap(),
                "--depth=1",
                "--branch",
                branch,
                "--single-branch",
            ])
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!("Git clone failed for {}", source.name));
        }
    }

    if let Some(ref subfolder) = source.subfolder {
        Ok(clone_dir.join(subfolder))
    } else {
        Ok(clone_dir)
    }
}

fn fetch_github_private(source: &Source) -> Result<PathBuf, String> {
    let repo_url = source.repo_url.as_ref().ok_or_else(|| "github-private type requires repo_url".to_string())?;
    let branch = source.branch.as_deref().unwrap_or("main");
    let token_env_var = source.token_env_var.as_ref().ok_or_else(|| "github-private type requires token_env_var".to_string())?;

    let token = std::env::var(token_env_var)
        .map_err(|_| format!("Environment variable '{}' is not set for private repo", token_env_var))?;

    let repo_name = clean_repo_name(repo_url);
    let clone_dir = cache_dir().join(&repo_name);

    fs::create_dir_all(cache_dir()).map_err(|e| e.to_string())?;

    // Inject token: https://github.com/... -> https://<token>@github.com/...
    let authed_url = repo_url.replace("https://", &format!("https://{}@", token));

    if clone_dir.join(".git").exists() {
        println!("Updating existing private repository at {:?}", clone_dir);
        // Setremote temporarily
        let _ = Command::new("git")
            .args(&["remote", "set-url", "origin", &authed_url])
            .current_dir(&clone_dir)
            .status();

        let fetch_status = Command::new("git")
            .args(&["fetch", "origin", branch, "--depth=1"])
            .current_dir(&clone_dir)
            .status();

        let reset_status = Command::new("git")
            .args(&["reset", "--hard", &format!("origin/{}", branch)])
            .current_dir(&clone_dir)
            .status();

        // Restore clean remote URL so token isn't stored in .git/config
        let _ = Command::new("git")
            .args(&["remote", "set-url", "origin", repo_url])
            .current_dir(&clone_dir)
            .status();

        fetch_status.map_err(|e| e.to_string())?;
        let status = reset_status.map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("Git reset failed for private repo {}", source.name));
        }
    } else {
        println!("Cloning private repository to {:?}", clone_dir);
        let status = Command::new("git")
            .args(&[
                "clone",
                &authed_url,
                clone_dir.to_str().unwrap(),
                "--depth=1",
                "--branch",
                branch,
                "--single-branch",
            ])
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!("Git clone failed for private repo {}", source.name));
        }

        // Clean up remote URL
        let _ = Command::new("git")
            .args(&["remote", "set-url", "origin", repo_url])
            .current_dir(&clone_dir)
            .status();
    }

    if let Some(ref subfolder) = source.subfolder {
        Ok(clone_dir.join(subfolder))
    } else {
        Ok(clone_dir)
    }
}

fn fetch_local_folder(source: &Source) -> Result<PathBuf, String> {
    let local_path = source.local_path.as_ref().ok_or_else(|| "local-folder type requires local_path".to_string())?;
    let path = Path::new(local_path);
    if !path.exists() {
        return Err(format!("Local path does not exist: {}", local_path));
    }
    Ok(path.to_path_buf())
}

fn clean_repo_name(url: &str) -> String {
    // e.g. https://github.com/WordPress/gutenberg.git -> WordPress--gutenberg
    let cleaned = url
        .trim_end_matches(".git")
        .replace("https://", "")
        .replace("http://", "");
    
    // strip host name
    let parts: Vec<&str> = cleaned.split('/').collect();
    if parts.len() >= 3 {
        parts[1..].join("--")
    } else {
        cleaned.replace('/', "--")
    }
}
