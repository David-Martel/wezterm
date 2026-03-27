use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use git2::{Repository, Signature};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use wezterm_benchmarks::git::{
    GitOperations, GitStatusCache, IncrementalGitStatus, ParallelGitStatus,
};

fn create_test_repo(num_files: usize, num_commits: usize) -> (TempDir, Repository) {
    let temp_dir = TempDir::new().expect("create temp dir for git benchmark repo");
    let repo = Repository::init(temp_dir.path()).expect("init git repository in temp dir");

    let sig = Signature::now("Test User", "test@example.com")
        .expect("create git signature for benchmark commits");

    // Create initial files
    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("Initial content {}", i))
            .expect("write initial benchmark file");
    }

    // Create commits
    for commit_num in 0..num_commits {
        // Modify some files
        for i in 0..(num_files / 10).max(1) {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            std::fs::write(
                &file_path,
                format!("Content commit {} file {}", commit_num, i),
            )
            .expect("write modified benchmark file for commit");
        }

        // Stage all changes
        let mut index = repo.index().expect("open git index for benchmark commit");
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .expect("stage all files for benchmark commit");
        index.write().expect("write git index to disk");

        let tree_id = index
            .write_tree()
            .expect("write index tree for benchmark commit");
        let tree = repo
            .find_tree(tree_id)
            .expect("find tree object for benchmark commit");

        let parent_commit = if commit_num == 0 {
            vec![]
        } else {
            vec![repo
                .head()
                .expect("get HEAD for benchmark commit")
                .peel_to_commit()
                .expect("peel HEAD to commit for benchmark")]
        };

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Commit {}", commit_num),
            &tree,
            &parent_commit.iter().collect::<Vec<_>>(),
        )
        .expect("create benchmark commit");
    }

    (temp_dir, repo)
}

fn bench_git_status(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for git status benchmark");

    let mut group = c.benchmark_group("git_status");

    for &num_files in &[10, 100, 1000] {
        let (temp_dir, _repo) = create_test_repo(num_files, 10);
        let repo_path = temp_dir.path().to_path_buf();

        group.bench_with_input(
            BenchmarkId::new("libgit2_status", num_files),
            &repo_path,
            |b, path| {
                b.iter(|| {
                    let ops = GitOperations::new(path);
                    let status = ops.get_status().expect("get git status in benchmark");
                    black_box(status)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_status", num_files),
            &repo_path,
            |b, path| {
                let cache = GitStatusCache::new(std::time::Duration::from_secs(1));
                b.iter(|| {
                    let status = cache
                        .get_status(path)
                        .expect("get cached git status in benchmark");
                    black_box(status)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel_status", num_files),
            &repo_path,
            |b, path| {
                b.to_async(&rt).iter(|| async {
                    let ops = ParallelGitStatus::new();
                    let status = ops
                        .get_status(path)
                        .await
                        .expect("get parallel git status in benchmark");
                    black_box(status)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("incremental_status", num_files),
            &repo_path,
            |b, path| {
                let ops = IncrementalGitStatus::new(path);
                b.iter(|| {
                    let changes = ops
                        .get_changes()
                        .expect("get incremental git changes in benchmark");
                    black_box(changes)
                });
            },
        );
    }

    group.finish();
}

fn bench_git_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_diff");

    for &num_files in &[10, 100, 500] {
        let (temp_dir, _repo) = create_test_repo(num_files, 5);
        let repo_path = temp_dir.path().to_path_buf();

        // Modify files for diff
        for i in 0..(num_files / 2) {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            std::fs::write(&file_path, format!("Modified content {}", i))
                .expect("write modified file for git diff benchmark");
        }

        group.bench_with_input(
            BenchmarkId::new("full_diff", num_files),
            &repo_path,
            |b, path| {
                b.iter(|| {
                    let ops = GitOperations::new(path);
                    let diff = ops.get_diff().expect("get git diff in benchmark");
                    black_box(diff)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_diff", num_files),
            &repo_path,
            |b, path| {
                let cache = GitStatusCache::new(std::time::Duration::from_secs(1));
                b.iter(|| {
                    let diff = cache
                        .get_diff(path)
                        .expect("get cached git diff in benchmark");
                    black_box(diff)
                });
            },
        );
    }

    group.finish();
}

fn bench_git_log(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_log");
    group.sample_size(10);

    for &num_commits in &[10, 100, 500] {
        let (temp_dir, _repo) = create_test_repo(10, num_commits);
        let repo_path = temp_dir.path().to_path_buf();

        group.bench_with_input(
            BenchmarkId::new("full_log", num_commits),
            &repo_path,
            |b, path| {
                b.iter(|| {
                    let ops = GitOperations::new(path);
                    let log = ops.get_log(100).expect("get git log in benchmark");
                    black_box(log)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_log", num_commits),
            &repo_path,
            |b, path| {
                let cache = GitStatusCache::new(std::time::Duration::from_secs(5));
                b.iter(|| {
                    let log = cache
                        .get_log(path, 100)
                        .expect("get cached git log in benchmark");
                    black_box(log)
                });
            },
        );
    }

    group.finish();
}

fn bench_git_blame(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_blame");
    group.sample_size(10);

    let (temp_dir, _repo) = create_test_repo(10, 50);
    let file_path = temp_dir.path().join("file_0.txt");

    group.bench_function("blame_file", |b| {
        b.iter(|| {
            let ops = GitOperations::new(temp_dir.path());
            let blame = ops
                .blame_file(&file_path)
                .expect("blame file in git benchmark");
            black_box(blame)
        });
    });

    group.bench_function("cached_blame", |b| {
        let cache = GitStatusCache::new(std::time::Duration::from_secs(10));
        b.iter(|| {
            let blame = cache
                .blame_file(temp_dir.path(), &file_path)
                .expect("get cached git blame in benchmark");
            black_box(blame)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_git_status,
    bench_git_diff,
    bench_git_log,
    bench_git_blame
);
criterion_main!(benches);
