use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::Result;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cache::cache_root_dir;

/// Result of file discovery, including which files were explicitly passed.
pub struct DiscoveredFiles {
    pub files: Vec<PathBuf>,
    /// Files passed directly on the command line (not discovered via directory walk).
    /// These bypass AllCops.Exclude unless --force-exclusion is set.
    pub explicit: HashSet<PathBuf>,
    /// Directories containing nested `.rubocop.yml` files discovered while
    /// scanning directory targets.
    pub sub_config_dirs: Vec<PathBuf>,
}

/// Discover Ruby files from the given paths, respecting .gitignore
/// and AllCops.Exclude patterns.
pub fn discover_files(paths: &[PathBuf]) -> Result<DiscoveredFiles> {
    let mut files = Vec::new();
    let mut explicit = HashSet::new();
    let mut sub_config_dirs = Vec::new();

    for path in paths {
        if path.is_file() {
            // Direct file paths bypass extension filtering
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            explicit.insert(canonical);
            files.push(path.clone());
        } else if path.is_dir() {
            let discovered = walk_directory(path)?;
            files.extend(discovered.files);
            sub_config_dirs.extend(discovered.sub_config_dirs);
        } else {
            anyhow::bail!("path does not exist: {}", path.display());
        }
    }

    files.sort();
    files.dedup();
    sub_config_dirs.sort();
    sub_config_dirs.dedup();
    Ok(DiscoveredFiles {
        files,
        explicit,
        sub_config_dirs,
    })
}

/// Exposed for testing only.
struct DiscoveredDirectory {
    files: Vec<PathBuf>,
    sub_config_dirs: Vec<PathBuf>,
}

const GIT_DISCOVERY_CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryStamp {
    rel_path: String,
    mtime_secs: u64,
    mtime_nanos: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitDiscoveryCache {
    version: u32,
    ruby_files: Vec<String>,
    sub_config_dirs: Vec<String>,
    watched_dirs: Vec<DiscoveryStamp>,
    watched_files: Vec<DiscoveryStamp>,
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

fn mtime_parts(path: &Path) -> Option<(u64, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    Some(systemtime_to_parts(meta.modified().ok()))
}

fn resolve_rel_path(root: &Path, rel: &str) -> PathBuf {
    if rel == "." {
        root.to_path_buf()
    } else {
        root.join(rel)
    }
}

fn git_discovery_cache_path(dir: &Path) -> PathBuf {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    cache_root_dir()
        .join("discovery")
        .join(format!("{}.json", &hash[..16]))
}

fn load_git_discovery_cache(dir: &Path) -> Option<DiscoveredDirectory> {
    let cache_path = git_discovery_cache_path(dir);
    let bytes = std::fs::read(cache_path).ok()?;
    let cache: GitDiscoveryCache = serde_json::from_slice(&bytes).ok()?;
    if cache.version != GIT_DISCOVERY_CACHE_VERSION {
        return None;
    }

    let stamps_valid = cache
        .watched_dirs
        .iter()
        .chain(cache.watched_files.iter())
        .all(|stamp| {
            let path = resolve_rel_path(dir, &stamp.rel_path);
            mtime_parts(&path)
                .is_some_and(|(secs, nanos)| secs == stamp.mtime_secs && nanos == stamp.mtime_nanos)
        });

    if !stamps_valid {
        return None;
    }

    let files = cache
        .ruby_files
        .into_iter()
        .map(|rel| dir.join(rel))
        .collect();
    let sub_config_dirs = cache
        .sub_config_dirs
        .into_iter()
        .map(|rel| dir.join(rel))
        .collect();

    Some(DiscoveredDirectory {
        files,
        sub_config_dirs,
    })
}

fn write_git_discovery_cache(
    dir: &Path,
    ruby_files: &[String],
    sub_config_dirs: &[String],
    watched_dirs: &HashSet<String>,
    watched_files: &HashSet<String>,
) {
    let mut watched_dir_stamps: Vec<DiscoveryStamp> = watched_dirs
        .iter()
        .filter_map(|rel| {
            let path = resolve_rel_path(dir, rel);
            let (mtime_secs, mtime_nanos) = mtime_parts(&path)?;
            Some(DiscoveryStamp {
                rel_path: rel.clone(),
                mtime_secs,
                mtime_nanos,
            })
        })
        .collect();
    watched_dir_stamps.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let mut watched_file_stamps: Vec<DiscoveryStamp> = watched_files
        .iter()
        .filter_map(|rel| {
            let path = resolve_rel_path(dir, rel);
            let (mtime_secs, mtime_nanos) = mtime_parts(&path)?;
            Some(DiscoveryStamp {
                rel_path: rel.clone(),
                mtime_secs,
                mtime_nanos,
            })
        })
        .collect();
    watched_file_stamps.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let cache = GitDiscoveryCache {
        version: GIT_DISCOVERY_CACHE_VERSION,
        ruby_files: ruby_files.to_vec(),
        sub_config_dirs: sub_config_dirs.to_vec(),
        watched_dirs: watched_dir_stamps,
        watched_files: watched_file_stamps,
    };

    let Ok(json) = serde_json::to_vec(&cache) else {
        return;
    };

    let cache_path = git_discovery_cache_path(dir);
    if let Some(parent) = cache_path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = std::fs::write(&cache_path, json);
}

fn walk_directory(dir: &Path) -> Result<DiscoveredDirectory> {
    // Fast path for git repos: one `git ls-files` call includes tracked files
    // (even if ignored) plus untracked non-ignored files.
    if let Some(discovered) = git_ls_ruby_files(dir) {
        return Ok(discovered);
    }

    let mut builder = WalkBuilder::new(dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .follow_links(true);

    // NOTE: We intentionally do NOT use the `ignore` crate's OverrideBuilder
    // for AllCops.Exclude patterns. The OverrideBuilder uses gitignore-style
    // override semantics where `!pattern` = whitelist (include), not exclude,
    // and positive patterns exclude ALL non-matching files. Instead, we filter
    // discovered files manually against the global exclude GlobSet, which is
    // already compiled in CopFilterSet::is_globally_excluded().

    let mut files = Vec::new();
    let mut sub_config_dirs = Vec::new();
    for entry in builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // skip broken symlinks / permission errors
        };
        let path = entry.path();
        if entry.file_type().is_some_and(|ft| ft.is_file()) && entry.file_name() == ".rubocop.yml" {
            if let Some(parent) = path.parent() {
                if parent != dir {
                    sub_config_dirs.push(parent.to_path_buf());
                }
            }
        }
        if path.is_file() && is_ruby_file(path) {
            files.push(path.to_path_buf());
        }
    }

    sub_config_dirs.sort();
    sub_config_dirs.dedup();

    Ok(DiscoveredDirectory {
        files,
        sub_config_dirs,
    })
}

/// Returns Ruby files for a git-backed directory using a single ls-files query.
///
/// The result includes tracked files (including tracked files under ignored
/// directories) and untracked files that are not ignored.
fn git_ls_ruby_files(dir: &Path) -> Option<DiscoveredDirectory> {
    if let Some(cached) = load_git_discovery_cache(dir) {
        return Some(cached);
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let mut files = Vec::new();
    let mut sub_config_dirs = Vec::new();
    let mut ruby_files_rel = Vec::new();
    let mut sub_config_dirs_rel = Vec::new();
    let mut watched_dirs: HashSet<String> = HashSet::from([".".to_string()]);
    let mut watched_files: HashSet<String> = HashSet::new();

    for line in String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|line| !line.is_empty())
    {
        // Track all parent directories so we can detect file-set changes
        // (create/delete/rename) on later runs without re-running git first.
        let mut parent = line;
        while let Some(idx) = parent.rfind('/') {
            parent = &parent[..idx];
            watched_dirs.insert(parent.to_string());
        }

        if line.ends_with(".gitignore") {
            watched_files.insert(line.to_string());
        }

        if line.ends_with("/.rubocop.yml") {
            let parent_rel = &line[..line.len() - "/.rubocop.yml".len()];
            if !parent_rel.is_empty() {
                sub_config_dirs.push(dir.join(parent_rel));
                sub_config_dirs_rel.push(parent_rel.to_string());
            }
        }

        let name = line.rsplit('/').next().unwrap_or(line);
        if is_ruby_filename_or_extension(name) {
            files.push(dir.join(line));
            ruby_files_rel.push(line.to_string());
            continue;
        }

        // Unknown extensionless filename: check shebang (e.g. bin/console).
        if name.contains('.') {
            continue;
        }
        let path = dir.join(line);
        if has_ruby_shebang(&path) {
            files.push(path);
            ruby_files_rel.push(line.to_string());
        }
    }

    let git_info_exclude = dir.join(".git").join("info").join("exclude");
    if git_info_exclude.exists() {
        watched_files.insert(".git/info/exclude".to_string());
    }

    sub_config_dirs.sort();
    sub_config_dirs.dedup();
    sub_config_dirs_rel.sort();
    sub_config_dirs_rel.dedup();

    write_git_discovery_cache(
        dir,
        &ruby_files_rel,
        &sub_config_dirs_rel,
        &watched_dirs,
        &watched_files,
    );

    Some(DiscoveredDirectory {
        files,
        sub_config_dirs,
    })
}

/// RuboCop-compatible Ruby file extensions (from AllCops.Include defaults).
const RUBY_EXTENSIONS: &[&str] = &[
    "rb",
    "arb",
    "axlsx",
    "builder",
    "fcgi",
    "gemfile",
    "gemspec",
    "god",
    "jb",
    "jbuilder",
    "mspec",
    "opal",
    "pluginspec",
    "podspec",
    "rabl",
    "rake",
    "rbuild",
    "rbw",
    "rbx",
    "ru",
    "ruby",
    "schema",
    "spec",
    "thor",
    "watchr",
];

/// Extensionless filenames that RuboCop treats as Ruby (from AllCops.Include defaults).
const RUBY_FILENAMES: &[&str] = &[
    ".irbrc",
    ".pryrc",
    ".simplecov",
    "buildfile",
    "Appraisals",
    "Berksfile",
    "Brewfile",
    "Buildfile",
    "Capfile",
    "Cheffile",
    "Dangerfile",
    "Deliverfile",
    "Fastfile",
    "Gemfile",
    "Guardfile",
    "Jarfile",
    "Mavenfile",
    "Podfile",
    "Puppetfile",
    "Rakefile",
    "rakefile",
    "Schemafile",
    "Snapfile",
    "Steepfile",
    "Thorfile",
    "Vagabondfile",
    "Vagrantfile",
];

/// Fast filename-only Ruby detection (extension, known names, Fastfile patterns,
/// and dotfile pseudo-extensions like `.gemfile`).
fn is_ruby_filename_or_extension(name: &str) -> bool {
    if RUBY_FILENAMES.contains(&name) {
        return true;
    }

    // Also match *Fastfile pattern (e.g., Matchfile, Appfile that end in Fastfile)
    if name.ends_with("Fastfile") || name.ends_with("fastfile") {
        return true;
    }

    // Extension-based match (case-insensitive)
    if let Some(ext) = name.rsplit('.').next() {
        if ext != name && RUBY_EXTENSIONS.iter().any(|&r| r.eq_ignore_ascii_case(ext)) {
            return true;
        }
    }

    // Dotfiles like `.gemfile` have no extension in Rust (Path::extension() returns None).
    // Check if the name after the leading dot matches a known Ruby extension.
    if let Some(after_dot) = name.strip_prefix('.') {
        if !after_dot.is_empty()
            && !after_dot.contains('.')
            && RUBY_EXTENSIONS
                .iter()
                .any(|&r| r.eq_ignore_ascii_case(after_dot))
        {
            return true;
        }
    }

    false
}

fn is_ruby_file(path: &Path) -> bool {
    // Fast filename/extension checks first.
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if is_ruby_filename_or_extension(name) {
            return true;
        }
    }

    // For extensionless files not in the known list, check for Ruby shebang.
    // This catches scripts like bin/console, bin/rails, etc.
    if path.extension().is_none() && has_ruby_shebang(path) {
        return true;
    }
    false
}

/// Ruby interpreter names recognized in shebang lines.
/// Matches RuboCop's `AllCops.RubyInterpreters` default: ruby, macruby, rake, jruby, rbx.
const RUBY_INTERPRETERS: &[&str] = &["ruby", "macruby", "rake", "jruby", "rbx"];

/// Check if a file starts with a Ruby shebang line (e.g. `#!/usr/bin/env ruby`).
/// Recognizes all interpreters in `RUBY_INTERPRETERS` (matching RuboCop's
/// `AllCops.RubyInterpreters`), not just `ruby`.
/// Only reads the first line to avoid expensive I/O during file discovery.
fn has_ruby_shebang(path: &Path) -> bool {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).is_err() {
        return false;
    }
    // Match standard shebangs (#!) and malformed ones with extra leading hashes (##!).
    let trimmed = first_line.trim_start_matches('#');
    if !trimmed.starts_with('!') {
        return false;
    }
    // Check if any recognized Ruby interpreter appears in the shebang line.
    // For shebangs like `#!/usr/bin/env ruby` or `#!/usr/bin/ruby`, the
    // interpreter name appears as a whitespace-delimited token or path component.
    RUBY_INTERPRETERS
        .iter()
        .any(|interp| first_line.contains(interp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn setup_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nitrocop_test_fs_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    #[test]
    fn is_ruby_file_dotfile_extensions() {
        // Rust's Path::extension() returns None for dotfiles like .gemfile.
        // is_ruby_file should still recognize them by checking after the leading dot.
        assert!(
            is_ruby_file(Path::new(".gemfile")),
            ".gemfile should be a Ruby file"
        );
        assert!(
            is_ruby_file(Path::new("some/path/.gemfile")),
            "some/path/.gemfile should be a Ruby file"
        );
        assert!(is_ruby_file(Path::new(".rb")), ".rb should be a Ruby file");
        assert!(
            is_ruby_file(Path::new(".rake")),
            ".rake should be a Ruby file"
        );
        // Regular extensions still work
        assert!(is_ruby_file(Path::new("foo.gemfile")));
        assert!(is_ruby_file(Path::new("Gemfile")));
        // Non-ruby dotfiles should not match
        assert!(!is_ruby_file(Path::new(".gitignore")));
        assert!(!is_ruby_file(Path::new(".env")));
    }

    #[test]
    fn discovers_rb_files_in_directory() {
        let dir = setup_dir("discover");
        fs::write(dir.join("a.rb"), "").unwrap();
        fs::write(dir.join("b.rb"), "").unwrap();
        fs::write(dir.join("c.txt"), "").unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        assert_eq!(discovered.files.len(), 2);
        assert!(
            discovered
                .files
                .iter()
                .all(|f| f.extension().unwrap() == "rb")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn direct_file_bypasses_extension_filter() {
        let dir = setup_dir("direct");
        let txt = dir.join("script");
        fs::write(&txt, "puts 'hi'").unwrap();

        let discovered = discover_files(&[txt.clone()]).unwrap();

        assert_eq!(discovered.files.len(), 1);
        assert_eq!(discovered.files[0], txt);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonexistent_path_errors() {
        let result = discover_files(&[PathBuf::from("/no/such/path")]);
        assert!(result.is_err());
    }

    #[test]
    fn results_are_sorted_and_deduped() {
        let dir = setup_dir("sorted");
        fs::write(dir.join("z.rb"), "").unwrap();
        fs::write(dir.join("a.rb"), "").unwrap();
        fs::write(dir.join("m.rb"), "").unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        let names: Vec<_> = discovered
            .files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a.rb", "m.rb", "z.rb"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_ruby_shebang_files() {
        let dir = setup_dir("shebang");
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(dir.join("app.rb"), "puts 'hi'").unwrap();
        fs::write(bin.join("console"), "#!/usr/bin/env ruby\nputs 'hi'\n").unwrap();
        fs::write(bin.join("setup"), "#!/bin/bash\necho hi\n").unwrap();
        fs::write(bin.join("server"), "#!/usr/bin/env ruby\nputs 'serve'\n").unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        assert_eq!(
            discovered.files.len(),
            3,
            "Should find app.rb + 2 ruby shebang scripts"
        );
        let names: Vec<_> = discovered
            .files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"app.rb".to_string()));
        assert!(names.contains(&"console".to_string()));
        assert!(names.contains(&"server".to_string()));
        assert!(!names.contains(&"setup".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn has_ruby_shebang_recognizes_all_interpreters() {
        use std::io::Write;

        let dir = setup_dir("shebang_interpreters");
        fs::create_dir_all(&dir).unwrap();

        // All RubyInterpreters from RuboCop's default config should be recognized
        let cases = [
            ("script_ruby", "#!/usr/bin/env ruby\nputs 'hi'\n", true),
            ("script_rbx", "#!/usr/bin/env rbx\nputs 'hi'\n", true),
            ("script_jruby", "#!/usr/bin/env jruby\nputs 'hi'\n", true),
            (
                "script_macruby",
                "#!/usr/bin/env macruby\nputs 'hi'\n",
                true,
            ),
            ("script_rake", "#!/usr/bin/env rake\nputs 'hi'\n", true),
            ("script_bash", "#!/bin/bash\necho hi\n", false),
            (
                "script_python",
                "#!/usr/bin/env python\nprint('hi')\n",
                false,
            ),
            (
                "script_direct_rbx",
                "#!/usr/local/bin/rbx\nputs 'hi'\n",
                true,
            ),
        ];

        for (name, content, expected) in &cases {
            let path = dir.join(name);
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
            assert_eq!(
                has_ruby_shebang(&path),
                *expected,
                "has_ruby_shebang({name}) should be {expected}"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_rbx_shebang_files() {
        // Regression test for brixen/poetics FN: bin/poetics with #!/usr/bin/env rbx
        let dir = setup_dir("shebang_rbx");
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("poetics"), "#!/usr/bin/env rbx\nputs 'hi'\n").unwrap();
        fs::write(bin.join("setup"), "#!/bin/bash\necho hi\n").unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        let names: Vec<_> = discovered
            .files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(
            names.contains(&"poetics".to_string()),
            "Should discover bin/poetics with rbx shebang; found: {names:?}"
        );
        assert!(
            !names.contains(&"setup".to_string()),
            "Should not discover bin/setup with bash shebang"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn debug_doorkeeper_bin_console() {
        use ignore::WalkBuilder;

        let doorkeeper_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/repos/doorkeeper");
        if !doorkeeper_dir.exists() {
            eprintln!("Skipping: doorkeeper not cloned");
            return;
        }

        let bin_console = doorkeeper_dir.join("bin/console");
        assert!(bin_console.exists(), "bin/console must exist");
        assert!(
            has_ruby_shebang(&bin_console),
            "bin/console must have ruby shebang"
        );
        assert!(
            is_ruby_file(&bin_console),
            "bin/console must be detected as ruby file"
        );

        // Walk with same settings as walk_directory
        let mut builder = WalkBuilder::new(&doorkeeper_dir);
        builder.hidden(true).git_ignore(true).git_global(true);

        let mut found_bin_console = false;
        let mut all_bin_files = Vec::new();
        for entry in builder.build() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.starts_with(doorkeeper_dir.join("bin")) {
                all_bin_files.push(path.to_path_buf());
            }
            if path == bin_console {
                found_bin_console = true;
            }
        }
        eprintln!("All entries under bin/: {:?}", all_bin_files);
        eprintln!("Found bin/console: {}", found_bin_console);

        // Now try without git_global
        let mut builder2 = WalkBuilder::new(&doorkeeper_dir);
        builder2.hidden(true).git_ignore(true).git_global(false);

        let mut found_without_global = false;
        for entry in builder2.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_global = true;
            }
        }
        eprintln!(
            "Found bin/console without git_global: {}",
            found_without_global
        );

        // Try without git_ignore too
        let mut builder3 = WalkBuilder::new(&doorkeeper_dir);
        builder3.hidden(true).git_ignore(false).git_global(false);

        let mut found_without_gitignore = false;
        for entry in builder3.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_gitignore = true;
            }
        }
        eprintln!(
            "Found bin/console without any git ignoring: {}",
            found_without_gitignore
        );

        // Try without parents
        let mut builder4 = WalkBuilder::new(&doorkeeper_dir);
        builder4
            .hidden(true)
            .git_ignore(true)
            .git_global(true)
            .parents(false);

        let mut found_without_parents = false;
        for entry in builder4.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_parents = true;
            }
        }
        eprintln!(
            "Found bin/console without parents: {}",
            found_without_parents
        );

        assert!(found_bin_console, "Walker must yield bin/console");
    }

    #[test]
    fn follows_symlinked_directories() {
        let dir = setup_dir("symlinks");
        // Create real directory with a Ruby file
        let shared = dir.join("shared").join("models");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("user.rb"), "class User; end\n").unwrap();

        // Create symlinked directory: app/models -> ../../shared/models
        let app = dir.join("app");
        fs::create_dir_all(&app).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../shared/models", app.join("models")).unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        // Should discover user.rb via both the real path AND the symlink path
        let paths: Vec<String> = discovered
            .files
            .iter()
            .map(|f| f.strip_prefix(&dir).unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            paths.contains(&"shared/models/user.rb".to_string()),
            "Should find real path: {paths:?}"
        );
        assert!(
            paths.contains(&"app/models/user.rb".to_string()),
            "Should find symlink path: {paths:?}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn skips_broken_symlinks() {
        let dir = setup_dir("broken_symlinks");
        fs::write(dir.join("good.rb"), "# ok\n").unwrap();
        // Create a broken symlink: bad.rb -> nonexistent.rb
        std::os::unix::fs::symlink("nonexistent.rb", dir.join("bad.rb")).unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        let names: Vec<String> = discovered
            .files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            names.contains(&"good.rb".to_string()),
            "Should find good.rb: {names:?}"
        );
        assert!(
            !names.contains(&"bad.rb".to_string()),
            "Should skip broken symlink: {names:?}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_nested_rb_files() {
        let dir = setup_dir("nested");
        let sub = dir.join("lib");
        fs::create_dir_all(&sub).unwrap();
        fs::write(dir.join("top.rb"), "").unwrap();
        fs::write(sub.join("nested.rb"), "").unwrap();

        let discovered = discover_files(&[dir.clone()]).unwrap();

        assert_eq!(discovered.files.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_tracked_hidden_ruby_file() {
        if !git_available() {
            eprintln!("Skipping: git not available");
            return;
        }

        let dir = setup_dir("tracked_hidden");
        fs::write(dir.join(".irbrc"), "IO.read('x')\n").unwrap();
        git(&dir, &["init", "-q"]);
        git(&dir, &["add", ".irbrc"]);

        let discovered = discover_files(&[dir.clone()]).unwrap();
        let contains = discovered
            .files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(".irbrc"));
        assert!(contains, "tracked .irbrc should be discovered");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_tracked_dotgemfile() {
        if !git_available() {
            eprintln!("Skipping: git not available");
            return;
        }

        let dir = setup_dir("tracked_dotgemfile");
        fs::write(
            dir.join(".gemfile"),
            "source 'https://rubygems.org'\ngem 'rails'\n",
        )
        .unwrap();
        git(&dir, &["init", "-q"]);
        git(&dir, &["add", ".gemfile"]);

        let discovered = discover_files(&[dir.clone()]).unwrap();
        let contains = discovered
            .files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(".gemfile"));
        assert!(contains, "tracked .gemfile should be discovered");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_tracked_gitignored_ruby_file() {
        if !git_available() {
            eprintln!("Skipping: git not available");
            return;
        }

        let dir = setup_dir("tracked_gitignored");
        let sandbox_dir = dir.join("work").join("sandbox");
        fs::create_dir_all(&sandbox_dir).unwrap();
        fs::write(dir.join(".gitignore"), "work/sandbox\n").unwrap();
        fs::write(sandbox_dir.join("multiton2.rb"), "Marshal.load(str)\n").unwrap();

        git(&dir, &["init", "-q"]);
        git(&dir, &["add", ".gitignore"]);
        git(&dir, &["add", "-f", "work/sandbox/multiton2.rb"]);

        let discovered = discover_files(&[dir.clone()]).unwrap();
        let contains = discovered
            .files
            .iter()
            .any(|p| p.ends_with(Path::new("work/sandbox/multiton2.rb")));
        assert!(
            contains,
            "tracked gitignored Ruby files should be discovered"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
