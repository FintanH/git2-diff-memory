use std::{ops::Range, path::PathBuf};

const CSS: &str = r#"h2 {
  margin-bottom: 1rem;
}
.event {
  margin-bottom: 1rem;
}
hr {
  margin: 1rem 0;
}
div {
  margin: 1rem 0;
}
.button {
  margin-bottom: 1rem;
}
"#;

const CSS_CHANGE: &str = r#".event {
  margin-bottom: 1rem;
}
h2 {
  margin-bottom: 1rem;
  padding: 1px;
}
hr {
  margin: 1rem 0;
}
div {
  margin: 1rem 0;
}
.button {
  margin-bottom: 2rem;
}
"#;

pub struct DiffLocation {
    /// The old side of the commit diff.
    pub base: git2::Oid,
    /// The new side of the commit diff.
    pub head: git2::Oid,
    /// Path of the file.
    pub path: PathBuf,
    /// The selected section of the diff.
    pub selection: DiffSelection,
}

impl DiffLocation {
    pub(crate) fn find_lines<'a>(
        &self,
        repo: &'a git2::Repository,
    ) -> Result<Vec<git2::DiffLine<'a>>, git2::Error> {
        let old = repo.find_commit(self.base)?.tree()?;
        let new = repo.find_commit(self.head)?.tree()?;
        let old = old.get_path(&self.path)?.to_object(repo)?.peel_to_blob()?;
        let new = new.get_path(&self.path)?.to_object(repo)?.peel_to_blob()?;
        let patch = git2::Patch::from_blobs(&old, None, &new, None, None)?;
        println!("PRINT FROM INSIDE find_lines");
        self.debug_patch(&patch)?;
        self.selection
            .lines
            .clone()
            .map(|i| patch.line_in_hunk(self.selection.hunk, i))
            .collect::<Result<Vec<_>, _>>()
    }

    fn debug_patch(&self, patch: &git2::Patch) -> Result<(), git2::Error> {
        let (hunk, lines) = patch.hunk(self.selection.hunk)?;
        for i in 0..lines {
            let line = patch.line_in_hunk(self.selection.hunk, i)?;
            print_diff_line(&line)
        }
        Ok(())
    }
}

pub struct DiffSelection {
    pub hunk: usize,
    pub lines: Range<usize>,
}

fn commit<'a>(
    repo: &'a git2::Repository,
    content: &[u8],
    tree: Option<&git2::Tree<'a>>,
    parents: &[&git2::Commit<'a>],
) -> Result<git2::Commit<'a>, git2::Error> {
    let blob = repo.blob(content)?;
    let mut tb = repo.treebuilder(tree)?;
    tb.insert("README", blob, git2::FileMode::Blob.into())?;
    let tree = tb.write()?;
    let tree = repo.find_tree(tree)?;
    let signature = repo.signature()?;
    let commit = repo.commit(None, &signature, &signature, "", &tree, parents)?;
    repo.find_commit(commit)
}

fn print_diff_line(line: &git2::DiffLine) {
    let content = String::from_utf8_lossy(line.content());
    let origin = line.origin();
    eprint!("{} {}", origin, content);
}

fn main() {
    let tmp = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(tmp.path().join("line-comments")).unwrap();
    let base = commit(&repo, CSS.as_bytes(), None, &[]).unwrap();
    let change = commit(
        &repo,
        CSS_CHANGE.as_bytes(),
        Some(&base.tree().unwrap()),
        &[&base],
    )
    .unwrap();
    let location = DiffLocation {
        base: base.id().into(),
        head: change.id().into(),
        path: std::path::Path::new("README").to_path_buf(),
        selection: DiffSelection {
            hunk: 0,
            lines: 0..11,
        },
    };
    let lines = location.find_lines(&repo).unwrap();
    println!("PRINT FROM OUTSIDE find_lines");
    for line in lines {
        print_diff_line(&line);
    }
}
