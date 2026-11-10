use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

type Assets<'a> = BTreeMap<&'a str, &'a [u8]>;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn validated_assets<'a>(assets: &'a [(&'a str, &'a [u8])]) -> io::Result<Assets<'a>> {
    let mut expected = BTreeMap::new();
    for &(relative, bytes) in assets {
        if relative.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':'])
        }) || expected.insert(relative, bytes).is_some()
        {
            return Err(invalid("Invalid or duplicate frontend asset path"));
        }
    }
    if expected.is_empty() {
        return Err(invalid("Frontend assets are empty"));
    }
    for relative in expected.keys() {
        for (offset, _) in relative.match_indices('/') {
            if expected.contains_key(&relative[..offset]) {
                return Err(invalid("Frontend asset file/directory conflict"));
            }
        }
    }
    Ok(expected)
}

fn content_directory(base: &Path, assets: &Assets<'_>) -> PathBuf {
    let mut hash = Sha256::new();
    hash.update(b"3dmk/frontend/v1\0");
    for (name, bytes) in assets {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    base.join(format!("frontend-{:x}", hash.finalize()))
}

fn verify_complete(directory: &Path, assets: &Assets<'_>) -> io::Result<()> {
    fn visit(dir: &Path, prefix: &str, assets: &Assets<'_>, count: &mut usize) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| invalid("Non-UTF-8 frontend path"))?;
            let relative = format!("{prefix}{name}");
            let kind = entry.file_type()?;
            if kind.is_dir() {
                let child = format!("{relative}/");
                if !assets.keys().any(|path| path.starts_with(&child)) {
                    return Err(invalid("Unexpected frontend directory"));
                }
                visit(&entry.path(), &child, assets, count)?;
            } else if kind.is_file() {
                let expected = assets
                    .get(relative.as_str())
                    .ok_or_else(|| invalid("Unexpected frontend file"))?;
                if entry.metadata()?.len() != expected.len() as u64
                    || fs::read(entry.path())?.as_slice() != *expected
                {
                    return Err(invalid(
                        "Published frontend content does not match this application",
                    ));
                }
                *count += 1;
            } else {
                return Err(invalid(
                    "Frontend entries must be regular files or directories",
                ));
            }
        }
        Ok(())
    }
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(invalid("Frontend destination must be a directory"));
    }
    let mut count = 0;
    visit(directory, "", assets, &mut count)?;
    if count != assets.len() {
        return Err(invalid("Published frontend is incomplete"));
    }
    Ok(())
}

struct Stage(Option<PathBuf>);

impl Drop for Stage {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn stage_assets(base: &Path, assets: &Assets<'_>) -> io::Result<Stage> {
    fs::create_dir_all(base)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let mut stage = None;
    for _ in 0..64 {
        let path = base.join(format!(
            ".frontend-stage-{}-{nonce}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                stage = Some(Stage(Some(path)));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let stage = stage.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Could not allocate a unique frontend stage",
        )
    })?;
    let directory = stage.0.as_ref().expect("new stage");
    for (relative, bytes) in assets {
        let file = directory.join(relative);
        fs::create_dir_all(file.parent().expect("asset parent"))?;
        let mut output = fs::File::create_new(file)?;
        output.write_all(bytes)?;
        output.sync_all()?;
    }
    verify_complete(directory, assets)?;
    Ok(stage)
}

fn publish_staged(
    mut stage: Stage,
    destination: &Path,
    assets: &Assets<'_>,
) -> io::Result<PathBuf> {
    match fs::rename(stage.0.as_ref().expect("unpublished stage"), destination) {
        Ok(()) => {
            stage.0 = None;
        }
        // Another launch may have published the same complete content first.
        Err(error) => {
            if verify_complete(destination, assets).is_err() {
                return Err(error);
            }
        }
    }
    Ok(destination.to_owned())
}

/// Serve one immutable, content-specific frontend per binary; never rewrite a
/// directory that an already-running application might still be serving.
pub fn publish_assets(base: &Path, assets: &[(&str, &[u8])]) -> io::Result<PathBuf> {
    let expected = validated_assets(assets)?;
    let destination = content_directory(base, &expected);
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            verify_complete(&destination, &expected)?;
            Ok(destination)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let stage = stage_assets(base, &expected)?;
            publish_staged(stage, &destination, &expected)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "3dmk-assets-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn changed_asset_content_preserves_the_previous_frontend() {
        let root = TestDir::new();
        let first =
            publish_assets(&root.0, &[("index.html", b"html"), ("app.js", b"old")]).unwrap();
        let second =
            publish_assets(&root.0, &[("index.html", b"html"), ("app.js", b"new")]).unwrap();
        assert_ne!(
            first, second,
            "different content must not share a serving directory"
        );
        assert_eq!(fs::read(first.join("app.js")).unwrap(), b"old");
        assert_eq!(fs::read(second.join("app.js")).unwrap(), b"new");
    }

    #[test]
    fn interrupted_staging_and_legacy_public_are_never_served() {
        let root = TestDir::new();
        let interrupted = root.0.join(".stage-abandoned");
        fs::create_dir(&interrupted).unwrap();
        fs::write(interrupted.join("index.html"), b"partial").unwrap();
        fs::create_dir(root.0.join("public")).unwrap();
        fs::write(root.0.join("public/index.html"), b"legacy").unwrap();
        let published = publish_assets(&root.0, &[("index.html", b"complete")]).unwrap();
        assert_ne!(published, root.0.join("public"));
        assert_ne!(published, interrupted);
        assert_eq!(fs::read(published.join("index.html")).unwrap(), b"complete");
        assert_eq!(
            fs::read(interrupted.join("index.html")).unwrap(),
            b"partial"
        );
        assert_eq!(
            fs::read(root.0.join("public/index.html")).unwrap(),
            b"legacy"
        );
    }

    #[test]
    fn concurrent_different_versions_cannot_overwrite_each_other() {
        let root = TestDir::new();
        let barrier = Arc::new(Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|version| {
                let base = root.0.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let content = vec![version; 32 * 1024];
                    barrier.wait();
                    let path = publish_assets(&base, &[("index.html", &content)]).unwrap();
                    (path, content)
                })
            })
            .collect();
        for worker in workers {
            let (path, content) = worker.join().unwrap();
            assert!(
                fs::read(path.join("index.html")).unwrap() == content,
                "concurrent versions overwrote one another"
            );
        }
    }

    #[test]
    fn concurrent_same_version_reuses_one_complete_directory() {
        let root = TestDir::new();
        let barrier = Arc::new(Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let base = root.0.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let assets: &[(&str, &[u8])] =
                        &[("index.html", b"html"), ("vendor/module.js", b"module")];
                    let expected = validated_assets(assets).unwrap();
                    let destination = content_directory(&base, &expected);
                    let stage = stage_assets(&base, &expected).unwrap();
                    assert!(!destination.exists(), "staging must not publish early");
                    barrier.wait();
                    publish_staged(stage, &destination, &expected).unwrap()
                })
            })
            .collect();
        let paths: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert!(paths.iter().all(|path| path == &paths[0]));
        assert_eq!(
            fs::read(paths[0].join("vendor/module.js")).unwrap(),
            b"module"
        );
        assert_eq!(
            fs::read_dir(&root.0).unwrap().count(),
            1,
            "losing stages must be removed"
        );
    }

    #[test]
    fn corrupt_published_content_fails_closed_without_rewriting() {
        let root = TestDir::new();
        let assets: &[(&str, &[u8])] = &[("index.html", b"complete")];
        let path = publish_assets(&root.0, assets).unwrap();
        fs::write(path.join("index.html"), b"corrupt").unwrap();
        assert!(publish_assets(&root.0, assets).is_err());
        assert_eq!(fs::read(path.join("index.html")).unwrap(), b"corrupt");
    }

    #[test]
    fn invalid_asset_tree_leaves_no_partial_publication() {
        let root = TestDir::new();
        assert!(publish_assets(
            &root.0,
            &[("index.html", b"html"), ("index.html/nested", b"invalid")]
        )
        .is_err());
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
    }

    #[test]
    fn staging_io_error_leaves_no_partial_publication() {
        let root = TestDir::new();
        let too_long = "z".repeat(300);
        assert!(
            publish_assets(&root.0, &[("index.html", b"html"), (&too_long, b"invalid")]).is_err()
        );
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
    }

    #[test]
    fn asset_order_is_irrelevant_but_paths_are_part_of_identity() {
        let root = TestDir::new();
        let first = publish_assets(&root.0, &[("a.js", b"one"), ("b.js", b"two")]).unwrap();
        let reordered = publish_assets(&root.0, &[("b.js", b"two"), ("a.js", b"one")]).unwrap();
        let renamed = publish_assets(&root.0, &[("a.js", b"one"), ("c.js", b"two")]).unwrap();
        assert_eq!(first, reordered);
        assert_ne!(first, renamed);
    }

    #[test]
    fn embedded_table_covers_every_frontend_file() {
        let public = Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let expected = validated_assets(crate::EMBEDDED_FRONTEND_ASSETS).unwrap();
        verify_complete(&public, &expected).unwrap();
        assert!(expected.contains_key("index.html"));
        assert!(expected.contains_key("scene-glb.js"));
        assert!(expected.contains_key("vendor/three/0.160.0/build/three.module.js"));
    }
}
