use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TodoError {
    #[error("impossible de lire {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("impossible d'écrire {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("la tâche {0} n'existe pas")]
    TaskNotFound(String),
    #[error("le fichier a changé depuis la proposition (attendu {expected}, obtenu {actual})")]
    Conflict { expected: String, actual: String },
    #[error("le chemin doit viser un fichier TODO.md")]
    InvalidTodoPath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub checked: bool,
    pub line_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TodoSnapshot {
    pub path: String,
    pub content: String,
    pub version: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct TodoDocument {
    path: PathBuf,
    content: String,
    version: String,
    tasks: Vec<ParsedTask>,
}

#[derive(Debug, Clone)]
struct ParsedTask {
    task: Task,
    checkbox_offset: usize,
}

impl TodoDocument {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, TodoError> {
        let path = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path).map_err(|source| TodoError::Read {
            path: path.clone(),
            source,
        })?;
        Self::from_content(path, content)
    }

    pub fn from_content(path: impl Into<PathBuf>, content: String) -> Result<Self, TodoError> {
        let path = path.into();
        let tasks = parse_tasks(&content);
        let version = content_version(&content);
        Ok(Self {
            path,
            content,
            version,
            tasks,
        })
    }

    pub fn snapshot(&self) -> TodoSnapshot {
        TodoSnapshot {
            path: self.path.display().to_string(),
            content: self.content.clone(),
            version: self.version.clone(),
            tasks: self.tasks.iter().map(|task| task.task.clone()).collect(),
        }
    }

    pub fn toggle_task(&self, task_id: &str) -> Result<String, TodoError> {
        let task = self
            .tasks
            .iter()
            .find(|candidate| candidate.task.id == task_id)
            .ok_or_else(|| TodoError::TaskNotFound(task_id.to_owned()))?;

        let mut content = self.content.clone();
        let next_marker = if content.as_bytes()[task.checkbox_offset] == b'x' {
            b' '
        } else {
            b'x'
        };
        content.replace_range(
            task.checkbox_offset..task.checkbox_offset + 1,
            &String::from(next_marker as char),
        );
        Ok(content)
    }

    pub fn append_task(&self, title: &str) -> Result<String, TodoError> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(self.content.clone());
        }

        let newline = if self.content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let separator = if self.content.is_empty() || self.content.ends_with(['\n', '\r']) {
            ""
        } else {
            newline
        };
        Ok(format!("{}{separator}- [ ] {title}{newline}", self.content))
    }

    pub fn save_if_unchanged(
        path: impl AsRef<Path>,
        expected_version: &str,
        content: &str,
    ) -> Result<Self, TodoError> {
        let path = path.as_ref().to_path_buf();
        let current = fs::read_to_string(&path).map_err(|source| TodoError::Read {
            path: path.clone(),
            source,
        })?;
        let actual = content_version(&current);
        if actual != expected_version {
            return Err(TodoError::Conflict {
                expected: expected_version.to_owned(),
                actual,
            });
        }

        atomic_write(&path, content)?;
        Self::from_content(path, content.to_owned())
    }
}

pub fn ensure_todo_path(path: impl AsRef<Path>) -> Result<PathBuf, TodoError> {
    let path = path.as_ref();
    let is_todo = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("TODO.md"));
    if !is_todo {
        return Err(TodoError::InvalidTodoPath);
    }
    Ok(path.to_path_buf())
}

fn parse_tasks(content: &str) -> Vec<ParsedTask> {
    let mut tasks = Vec::new();
    let mut offset = 0;

    for (line_index, segment) in content.split_inclusive('\n').enumerate() {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(marker_start) = line.find("- [") {
            let checkbox_offset = marker_start + 3;
            let marker = line.as_bytes().get(checkbox_offset).copied();
            let closing = line.as_bytes().get(checkbox_offset + 1).copied();
            if matches!(marker, Some(b' ' | b'x' | b'X')) && closing == Some(b']') {
                let title = line[checkbox_offset + 2..].trim();
                tasks.push(ParsedTask {
                    task: Task {
                        id: format!("task-{line_index}"),
                        title: title.to_owned(),
                        checked: marker != Some(b' '),
                        line_index,
                    },
                    checkbox_offset: offset + checkbox_offset,
                });
            }
        }
        offset += segment.len();
    }

    tasks
}

fn content_version(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn atomic_write(path: &Path, content: &str) -> Result<(), TodoError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("TODO.md");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(".{file_name}.{nonce}.taskhud.tmp"));
    let backup_path = parent.join(format!(".{file_name}.{nonce}.taskhud.bak"));

    let mut temp = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .map_err(|source| TodoError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    temp.write_all(content.as_bytes())
        .map_err(|source| TodoError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    temp.sync_all().map_err(|source| TodoError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    drop(temp);

    let had_original = path.exists();
    if had_original {
        fs::rename(path, &backup_path).map_err(|source| TodoError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }

    if let Err(source) = fs::rename(&temp_path, path) {
        if had_original {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(TodoError::Write {
            path: path.to_path_buf(),
            source,
        });
    }

    if had_original {
        let _ = fs::remove_file(backup_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_todo_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("l'horloge système doit être disponible")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "taskhud-core-{label}-{}-{nonce}-TODO.md",
            std::process::id()
        ))
    }

    #[test]
    fn parse_tasks_preserves_ids_and_crlf_content() {
        let content = "# Aujourd'hui\r\n\r\n- [ ] Écrire la démo\r\n- [x] Relire le contrat\r\n";
        let document = TodoDocument::from_content("TODO.md", content.to_owned()).unwrap();
        let snapshot = document.snapshot();

        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.tasks[0].id, "task-2");
        assert_eq!(snapshot.tasks[0].title, "Écrire la démo");
        assert!(!snapshot.tasks[0].checked);
        assert_eq!(snapshot.tasks[1].id, "task-3");
        assert!(snapshot.tasks[1].checked);
        assert_eq!(snapshot.content, content);
    }

    #[test]
    fn toggle_task_changes_only_the_checkbox() {
        let content = "- [ ] Première\n- [x] Deuxième\n";
        let document = TodoDocument::from_content("TODO.md", content.to_owned()).unwrap();

        assert_eq!(
            document.toggle_task("task-0").unwrap(),
            "- [x] Première\n- [x] Deuxième\n"
        );
        assert_eq!(
            document.toggle_task("task-1").unwrap(),
            "- [ ] Première\n- [ ] Deuxième\n"
        );
        assert!(matches!(
            document.toggle_task("task-99"),
            Err(TodoError::TaskNotFound(_))
        ));
    }

    #[test]
    fn append_task_keeps_the_existing_newline_style() {
        let document =
            TodoDocument::from_content("TODO.md", "- [ ] Départ\r\n".to_owned()).unwrap();
        assert_eq!(
            document.append_task("  Nouvelle tâche  ").unwrap(),
            "- [ ] Départ\r\n- [ ] Nouvelle tâche\r\n"
        );
    }

    #[test]
    fn save_if_unchanged_writes_and_rejects_stale_versions() {
        let path = temp_todo_path("save");
        let initial = "- [ ] Départ\n";
        fs::write(&path, initial).unwrap();

        let document = TodoDocument::load(&path).unwrap();
        let saved =
            TodoDocument::save_if_unchanged(&path, &document.snapshot().version, "- [x] Départ\n")
                .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [x] Départ\n");
        assert_eq!(saved.snapshot().tasks[0].checked, true);

        fs::write(&path, "- [ ] Modification externe\n").unwrap();
        let error = TodoDocument::save_if_unchanged(
            &path,
            &saved.snapshot().version,
            "- [x] Écrasement interdit\n",
        )
        .unwrap_err();
        assert!(matches!(error, TodoError::Conflict { .. }));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Modification externe\n"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_todo_path_rejects_other_files() {
        assert!(ensure_todo_path("notes.md").is_err());
        assert!(ensure_todo_path("todo.md").is_ok());
        assert!(ensure_todo_path("TODO.md").is_ok());
    }
}
