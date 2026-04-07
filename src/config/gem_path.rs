use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cache::cache_root_dir;

/// Cache for gem path resolution. Keyed on (working_dir, gem_name), stores
/// the resolved path. Invalidated when Gemfile.lock mtime changes.
struct GemPathCache {
    entries: HashMap<(PathBuf, String), PathBuf>,
    lockfile_mtime: Option<SystemTime>,
    working_dir: PathBuf,
}

static GEM_PATH_CACHE: Mutex<Option<GemPathCache>> = Mutex::new(None);

const GEM_PATH_DISK_CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct GemPathDiskCache {
    version: u32,
    lockfile_mtime_secs: u64,
    lockfile_mtime_nanos: u32,
    lockfile_size: u64,
    entries: HashMap<String, String>,
}

fn lockfile_meta(working_dir: &Path) -> (Option<SystemTime>, Option<(u64, u32, u64)>) {
    let path = working_dir.join("Gemfile.lock");
    let Ok(meta) = path.metadata() else {
        return (None, None);
    };
    let modified = meta.modified().ok();
    let (secs, nanos) = systemtime_to_parts(modified);
    (modified, Some((secs, nanos, meta.len())))
}

fn systemtime_to_parts(time: Option<SystemTime>) -> (u64, u32) {
    match time {
        Some(t) => match t.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => (d.as_secs(), d.subsec_nanos()),
            Err(_) => (0, 0),
        },
        None => (0, 0),
    }
}

fn gem_path_disk_cache_path(working_dir: &Path) -> PathBuf {
    let canonical = working_dir
        .canonicalize()
        .unwrap_or_else(|_| working_dir.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    cache_root_dir()
        .join("config")
        .join(format!("gem-paths-{}.json", &hash[..16]))
}

fn load_disk_cache_entries(working_dir: &Path, stamp: (u64, u32, u64)) -> HashMap<String, PathBuf> {
    let cache_path = gem_path_disk_cache_path(working_dir);
    let Ok(bytes) = std::fs::read(cache_path) else {
        return HashMap::new();
    };
    let Ok(cache) = serde_json::from_slice::<GemPathDiskCache>(&bytes) else {
        return HashMap::new();
    };
    if cache.version != GEM_PATH_DISK_CACHE_VERSION
        || cache.lockfile_mtime_secs != stamp.0
        || cache.lockfile_mtime_nanos != stamp.1
        || cache.lockfile_size != stamp.2
    {
        return HashMap::new();
    }

    cache
        .entries
        .into_iter()
        .map(|(gem, path)| (gem, PathBuf::from(path)))
        .filter(|(_, path)| path.exists())
        .collect()
}

fn write_disk_cache_entries(
    working_dir: &Path,
    stamp: (u64, u32, u64),
    entries: &HashMap<String, PathBuf>,
) {
    let cache = GemPathDiskCache {
        version: GEM_PATH_DISK_CACHE_VERSION,
        lockfile_mtime_secs: stamp.0,
        lockfile_mtime_nanos: stamp.1,
        lockfile_size: stamp.2,
        entries: entries
            .iter()
            .map(|(gem, path)| (gem.clone(), path.to_string_lossy().to_string()))
            .collect(),
    };

    let Ok(bytes) = serde_json::to_vec(&cache) else {
        return;
    };

    let cache_path = gem_path_disk_cache_path(working_dir);
    if let Some(parent) = cache_path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let _ = std::fs::write(cache_path, bytes);
}

/// Resolve a gem's install path via `bundle info --path <gem_name>`.
///
/// `working_dir` is the directory where `bundle` should run (typically the
/// project root where `Gemfile.lock` lives). Results are cached per
/// (working_dir, gem_name) and invalidated when Gemfile.lock mtime changes.
pub fn resolve_gem_path(gem_name: &str, working_dir: &Path) -> Result<PathBuf> {
    let (lockfile_mtime, lockfile_stamp) = lockfile_meta(working_dir);
    let cache_key = (working_dir.to_path_buf(), gem_name.to_string());

    // Check in-process cache.
    {
        let cache = GEM_PATH_CACHE.lock().unwrap();
        if let Some(ref c) = *cache {
            if c.working_dir == working_dir && c.lockfile_mtime == lockfile_mtime {
                if let Some(path) = c.entries.get(&cache_key) {
                    return Ok(path.clone());
                }
            }
        }
    }

    // Check persistent cache keyed by Gemfile.lock stamp.
    if let Some(stamp) = lockfile_stamp {
        let disk_entries = load_disk_cache_entries(working_dir, stamp);
        if let Some(path) = disk_entries.get(gem_name) {
            insert_cache_entry(working_dir, lockfile_mtime, gem_name, path);
            return Ok(path.clone());
        }
    }

    // Run bundle info --path from the working directory.
    // Use `mise exec --` if the target project has a .ruby-version or .tool-versions
    // that may differ from the current shell's Ruby. This ensures the correct
    // Ruby/Bundler environment resolves the gems.
    let bundle_start = std::time::Instant::now();
    let needs_mise = needs_mise_exec(working_dir);
    let output = if needs_mise {
        Command::new("mise")
            .args(["exec", "--", "bundle", "info", "--path", gem_name])
            .current_dir(working_dir)
            .output()
            .with_context(|| {
                format!(
                    "Cannot resolve gem '{}': `mise exec -- bundle` failed. \
                     Ensure mise is installed and `bundle install` has been run.",
                    gem_name
                )
            })?
    } else {
        Command::new("bundle")
            .args(["info", "--path", gem_name])
            .current_dir(working_dir)
            .output()
            .with_context(|| {
                format!(
                    "Cannot resolve gem '{}': `bundle` not found on PATH. \
                     Install Bundler or remove inherit_gem/require from your .rubocop.yml.",
                    gem_name
                )
            })?
    };
    let bundle_elapsed = bundle_start.elapsed();
    eprintln!(
        "debug: bundle info --path {}: {:.0?}",
        gem_name, bundle_elapsed
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Gem '{}' not found in bundle (working_dir: {}). \
             Run `bundle install` or remove it from inherit_gem. \
             bundle info stderr: {}",
            gem_name,
            working_dir.display(),
            stderr.trim()
        );
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(&path_str);

    if !path.exists() {
        anyhow::bail!(
            "Gem '{}' resolved to '{}' but that directory does not exist.",
            gem_name,
            path_str
        );
    }

    insert_cache_entry(working_dir, lockfile_mtime, gem_name, &path);

    if let Some(stamp) = lockfile_stamp {
        let mut disk_entries = load_disk_cache_entries(working_dir, stamp);
        disk_entries.insert(gem_name.to_string(), path.clone());
        write_disk_cache_entries(working_dir, stamp, &disk_entries);
    }

    Ok(path)
}

/// Resolve multiple gem paths in one Bundler process.
///
/// Returns only successfully resolved gems and updates the in-process cache.
pub fn resolve_gem_paths_batch(
    gem_names: &[String],
    working_dir: &Path,
) -> Result<HashMap<String, PathBuf>> {
    if gem_names.is_empty() {
        return Ok(HashMap::new());
    }

    let (lockfile_mtime, lockfile_stamp) = lockfile_meta(working_dir);

    let mut resolved: HashMap<String, PathBuf> = HashMap::new();
    let mut missing: Vec<String> = Vec::new();

    {
        let cache = GEM_PATH_CACHE.lock().unwrap();
        if let Some(ref c) = *cache {
            if c.working_dir == working_dir && c.lockfile_mtime == lockfile_mtime {
                for gem in gem_names {
                    let key = (working_dir.to_path_buf(), gem.clone());
                    if let Some(path) = c.entries.get(&key) {
                        resolved.insert(gem.clone(), path.clone());
                    } else {
                        missing.push(gem.clone());
                    }
                }
            } else {
                missing.extend(gem_names.iter().cloned());
            }
        } else {
            missing.extend(gem_names.iter().cloned());
        }
    }

    let mut disk_entries = lockfile_stamp.map(|stamp| load_disk_cache_entries(working_dir, stamp));
    if !missing.is_empty() {
        if let Some(ref entries) = disk_entries {
            let mut still_missing = Vec::new();
            for gem in missing {
                if let Some(path) = entries.get(&gem) {
                    resolved.insert(gem.clone(), path.clone());
                    insert_cache_entry(working_dir, lockfile_mtime, &gem, path);
                } else {
                    still_missing.push(gem);
                }
            }
            missing = still_missing;
        }
    }

    if missing.is_empty() {
        return Ok(resolved);
    }

    let script = r##"
require 'bundler'
specs = Bundler.load.specs.each_with_object({}) { |s, h| h[s.name] = s.full_gem_path }
ARGV.each do |name|
  path = specs[name]
  puts "#{name}\t#{path}" if path
end
"##;

    let bundle_start = std::time::Instant::now();
    let needs_mise = needs_mise_exec(working_dir);
    let output = if needs_mise {
        Command::new("mise")
            .args(["exec", "--", "ruby", "-e", script, "--"])
            .args(&missing)
            .current_dir(working_dir)
            .output()
            .with_context(|| {
                "Cannot resolve gem paths in batch: `mise exec -- ruby` failed. Ensure mise is installed and bundle is available."
            })?
    } else {
        Command::new("bundle")
            .args(["exec", "--", "ruby", "-e", script, "--"])
            .args(&missing)
            .current_dir(working_dir)
            .output()
            .with_context(|| {
                "Cannot resolve gem paths in batch: `bundle exec ruby` failed. Ensure Bundler is installed and Gemfile is valid."
            })?
    };
    eprintln!(
        "debug: bundle batch gem path resolve ({} gems): {:.0?}",
        missing.len(),
        bundle_start.elapsed()
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Batch gem resolution failed in {}: {}",
            working_dir.display(),
            stderr.trim()
        );
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.splitn(2, '\t');
        let Some(gem_name) = parts.next() else {
            continue;
        };
        let Some(path_str) = parts.next() else {
            continue;
        };
        let path = PathBuf::from(path_str);
        if path.exists() {
            resolved.insert(gem_name.to_string(), path.clone());
            insert_cache_entry(working_dir, lockfile_mtime, gem_name, &path);
            if let Some(ref mut entries) = disk_entries {
                entries.insert(gem_name.to_string(), path);
            }
        }
    }

    if let (Some(stamp), Some(ref entries)) = (lockfile_stamp, disk_entries.as_ref()) {
        write_disk_cache_entries(working_dir, stamp, entries);
    }

    Ok(resolved)
}

fn insert_cache_entry(
    working_dir: &Path,
    lockfile_mtime: Option<SystemTime>,
    gem_name: &str,
    path: &Path,
) {
    let mut cache = GEM_PATH_CACHE.lock().unwrap();
    let c = cache.get_or_insert_with(|| GemPathCache {
        entries: HashMap::new(),
        lockfile_mtime,
        working_dir: working_dir.to_path_buf(),
    });
    // Reset cache if lockfile or working_dir changed
    if c.lockfile_mtime != lockfile_mtime || c.working_dir != working_dir {
        c.entries.clear();
        c.lockfile_mtime = lockfile_mtime;
        c.working_dir = working_dir.to_path_buf();
    }
    c.entries.insert(
        (working_dir.to_path_buf(), gem_name.to_string()),
        path.to_path_buf(),
    );
}

/// Extract all resolved gem paths from the in-process cache.
/// Returns a map of gem_name → gem_root_path.
/// Used by `nitrocop --init` to populate the lockfile.
pub fn drain_resolved_paths() -> HashMap<String, PathBuf> {
    let cache = GEM_PATH_CACHE.lock().unwrap();
    match *cache {
        Some(ref c) => c
            .entries
            .iter()
            .map(|((_, gem_name), path)| (gem_name.clone(), path.clone()))
            .collect(),
        None => HashMap::new(),
    }
}

/// Check if the working directory has a `.ruby-version` or `.tool-versions` file,
/// indicating it may need `mise exec --` to activate the correct Ruby.
/// Only returns true if `mise` is actually available on PATH.
fn needs_mise_exec(working_dir: &Path) -> bool {
    let has_version_file = working_dir.join(".ruby-version").exists()
        || working_dir.join(".tool-versions").exists()
        || working_dir.join(".mise.toml").exists();
    if !has_version_file {
        return false;
    }
    // Check mise is available (cached after first call)
    static MISE_AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *MISE_AVAILABLE.get_or_init(|| {
        Command::new("mise")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bundle_info_output() {
        // Simulate trimming of bundle info output
        let raw = "  /home/user/.gem/ruby/3.2.0/gems/rubocop-shopify-2.15.1  \n";
        let trimmed = raw.trim();
        assert_eq!(
            trimmed,
            "/home/user/.gem/ruby/3.2.0/gems/rubocop-shopify-2.15.1"
        );
        let path = PathBuf::from(trimmed);
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "rubocop-shopify-2.15.1"
        );
    }

    #[test]
    fn cache_key_behavior() {
        // Verify None == None for lockfile mtime comparison
        let a: Option<SystemTime> = None;
        let b: Option<SystemTime> = None;
        assert_eq!(a, b);
    }
}
