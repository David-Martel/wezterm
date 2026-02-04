fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // If a file named `.tag` is present, we'll take its contents for the
    // version number that we report in wezterm -h.
    let mut ci_tag = String::new();
    if let Ok(tag) = std::fs::read("../.tag") {
        if let Ok(s) = String::from_utf8(tag) {
            ci_tag = s.trim().to_string();
            println!("cargo:rerun-if-changed=../.tag");
        }
    } else {
        // Otherwise we'll derive it from the git information
        // Check if we're in a git repository using simple file existence check
        // to avoid libgit2 linking issues on Windows
        let git_dir = std::path::Path::new("../.git");
        let is_git_repo = git_dir.exists() || git_dir.is_file(); // .git can be a file for worktrees

        if is_git_repo {
            // Set up cache invalidation for git HEAD
            let head_file = git_dir.join("HEAD");
            if head_file.exists() {
                println!("cargo:rerun-if-changed={}", head_file.display());

                // Try to read the HEAD ref and watch that too
                if let Ok(head_contents) = std::fs::read_to_string(&head_file) {
                    if let Some(ref_path) = head_contents.strip_prefix("ref: ") {
                        let ref_file = git_dir.join(ref_path.trim());
                        if ref_file.exists() {
                            println!("cargo:rerun-if-changed={}", ref_file.display());
                        }
                    }
                }
            }

            if let Ok(output) = std::process::Command::new("git")
                .args(&[
                    "-c",
                    "core.abbrev=8",
                    "show",
                    "-s",
                    "--format=%cd-%h",
                    "--date=format:%Y%m%d-%H%M%S",
                ])
                .output()
            {
                let info = String::from_utf8_lossy(&output.stdout);
                ci_tag = info.trim().to_string();
            }
        }
    }

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=WEZTERM_TARGET_TRIPLE={}", target);
    println!("cargo:rustc-env=WEZTERM_CI_TAG={}", ci_tag);
}
