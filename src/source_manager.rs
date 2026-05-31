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
        "wordpress-org" => fetch_wordpress_org(source),
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

fn fetch_wordpress_org(source: &Source) -> Result<PathBuf, String> {
    let slug = source.repo_url.as_ref()
        .or(source.local_path.as_ref())
        .ok_or_else(|| "wordpress-org type requires slug (stored in repo_url or local_path)".to_string())?;
    
    // Clean slug in case a full URL was provided
    let cleaned_slug = slug
        .trim_end_matches('/')
        .split('/')
        .last()
        .ok_or_else(|| "Invalid slug format".to_string())?;

    let dest_dir = cache_dir().join(format!("wp-org--{}", cleaned_slug));
    fs::create_dir_all(cache_dir()).map_err(|e| e.to_string())?;

    println!("Querying WordPress.org API for plugin '{}'...", cleaned_slug);
    let api_url = format!(
        "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]={}",
        cleaned_slug
    );

    let output = Command::new("curl")
        .args(&["-sL", &api_url])
        .output()
        .map_err(|e| format!("Failed to execute curl: {}", e))?;

    if !output.status.success() {
        return Err(format!("WordPress API query failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let api_res: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse WordPress API JSON: {}. Response: {}", e, json_str))?;

    let download_link = api_res["download_link"]
        .as_str()
        .ok_or_else(|| format!("Plugin '{}' not found or download_link is missing in API response.", cleaned_slug))?;

    let temp_zip = cache_dir().join(format!("{}.zip", cleaned_slug));
    println!("Downloading plugin from {}...", download_link);

    let download_status = Command::new("curl")
        .args(&["-sL", "-o", temp_zip.to_str().unwrap(), download_link])
        .status()
        .map_err(|e| e.to_string())?;

    if !download_status.success() {
        return Err(format!("Failed to download plugin zip from {}", download_link));
    }

    println!("Extracting plugin to cache...");
    let unzip_status = Command::new("unzip")
        .args(&["-q", "-o", temp_zip.to_str().unwrap(), "-d", cache_dir().to_str().unwrap()])
        .status()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_file(temp_zip);

    if !unzip_status.success() {
        return Err(format!("Failed to unzip plugin {}", cleaned_slug));
    }

    let extracted_dir = cache_dir().join(cleaned_slug);
    if extracted_dir.exists() {
        if dest_dir.exists() {
            let _ = fs::remove_dir_all(&dest_dir);
        }
        fs::rename(&extracted_dir, &dest_dir)
            .map_err(|e| format!("Failed to rename extracted directory: {}", e))?;
    } else {
        return Err(format!("Extracted directory {:?} not found", extracted_dir));
    }

    if let Some(ref subfolder) = source.subfolder {
        Ok(dest_dir.join(subfolder))
    } else {
        Ok(dest_dir)
    }
}
