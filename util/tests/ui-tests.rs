use std::{
	fs,
	io::Result,
	path::{Path, PathBuf},
};

use trybuild::TestCases;

fn all_files(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
	let mut files = Vec::new();

	for entry in fs::read_dir(dir.as_ref())? {
		let entry = entry?;
		let path = entry.path();

		if path.is_dir() {
			files.extend(all_files(&path)?);
		} else if path.is_file() {
			if let Some(extension) = path.extension() {
				if extension == "rs" {
					files.push(path);
				}
			}
		}
	}

	Ok(files)
}

#[test]
fn ui_tests() -> Result<()> {
	let files = all_files("ui-tests")?;

	let tests = TestCases::new();

	for file in files {
		tests.compile_fail(file);
	}

	Ok(())
}
