use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod projects;
mod proposals;

pub use projects::{Project, ProjectRegistry};
pub use proposals::{ProposalStatus, ProposalStore, TaskChange, TaskProposal};

#[derive(Debug, Error)]
pub enum TodoError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("task {0} does not exist")]
    TaskNotFound(String),
    #[error("the file changed since the proposal (expected {expected}, got {actual})")]
    Conflict { expected: String, actual: String },
    #[error("the path must point to a TODO.md file")]
    InvalidTodoPath,
    #[error("project {0} does not exist")]
    ProjectNotFound(String),
    #[error("proposal {0} does not exist")]
    ProposalNotFound(String),
    #[error("invalid configuration {path}: {message}")]
    InvalidConfig { path: PathBuf, message: String },
    #[error("invalid proposal: {0}")]
    InvalidProposal(String),
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

    pub fn apply_changes(&self, changes: &[TaskChange]) -> Result<String, TodoError> {
        if changes.is_empty() {
            return Err(TodoError::InvalidProposal(
                "a proposal must contain at least one change".to_owned(),
            ));
        }

        let known_tasks: std::collections::HashMap<_, _> = self
            .tasks
            .iter()
            .map(|parsed| (parsed.task.id.clone(), parsed.task.clone()))
            .collect();
        let mut targeted_tasks = std::collections::HashSet::new();
        let mut updates = Vec::new();
        let mut deletes = std::collections::HashSet::new();
        let mut additions_after = Vec::new();
        let mut additions_at_end = Vec::new();

        for change in changes {
            if let TaskChange::Delete { task_id } = change {
                let task = known_tasks
                    .get(task_id)
                    .ok_or_else(|| TodoError::TaskNotFound(task_id.clone()))?;
                deletes.insert(task.line_index);
            }
        }

        for change in changes {
            match change {
                TaskChange::Add {
                    title,
                    after_task_id,
                } => {
                    let title = title.trim();
                    if title.is_empty() {
                        return Err(TodoError::InvalidProposal(
                            "an added task must have a title".to_owned(),
                        ));
                    }
                    if let Some(task_id) = after_task_id {
                        let task = known_tasks
                            .get(task_id)
                            .ok_or_else(|| TodoError::TaskNotFound(task_id.clone()))?;
                        if deletes.contains(&task.line_index) {
                            return Err(TodoError::InvalidProposal(
                                "a task cannot be added after a deleted task".to_owned(),
                            ));
                        }
                        additions_after.push((task.line_index, format!("- [ ] {title}")));
                    } else {
                        additions_at_end.push(format!("- [ ] {title}"));
                    }
                }
                TaskChange::Update {
                    task_id,
                    title,
                    checked,
                } => {
                    let task = known_tasks
                        .get(task_id)
                        .ok_or_else(|| TodoError::TaskNotFound(task_id.clone()))?;
                    if title.is_none() && checked.is_none() {
                        return Err(TodoError::InvalidProposal(format!(
                            "the update of {task_id} is empty"
                        )));
                    }
                    if title
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                    {
                        return Err(TodoError::InvalidProposal(format!(
                            "the title of {task_id} cannot be empty"
                        )));
                    }
                    if !targeted_tasks.insert(task_id.clone()) {
                        return Err(TodoError::InvalidProposal(format!(
                            "task {task_id} is changed more than once"
                        )));
                    }
                    updates.push((
                        task.line_index,
                        task.clone(),
                        title.as_deref().map(str::trim),
                        *checked,
                    ));
                }
                TaskChange::Delete { task_id } => {
                    if !known_tasks.contains_key(task_id) {
                        return Err(TodoError::TaskNotFound(task_id.clone()));
                    }
                    if !targeted_tasks.insert(task_id.clone()) {
                        return Err(TodoError::InvalidProposal(format!(
                            "task {task_id} is changed more than once"
                        )));
                    }
                }
            }
        }

        let trailing_newline = self.content.ends_with('\n');
        let mut lines: Vec<String> = self
            .content
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned())
            .collect();
        if trailing_newline {
            let _ = lines.pop();
        }
        if self.content.is_empty() {
            lines.clear();
        }

        let mut result = Vec::with_capacity(lines.len() + changes.len());
        for (line_index, line) in lines.iter().enumerate() {
            if !deletes.contains(&line_index) {
                if let Some((_, task, title, checked)) =
                    updates.iter().find(|(index, ..)| *index == line_index)
                {
                    result.push(render_task_line(
                        line,
                        title.unwrap_or(task.title.as_str()),
                        checked.unwrap_or(task.checked),
                    ));
                } else {
                    result.push(line.clone());
                }
            }

            for (anchor, addition) in additions_after
                .iter()
                .filter(|(anchor, _)| *anchor == line_index)
            {
                let _ = anchor;
                result.push(addition.clone());
            }
        }
        result.extend(additions_at_end);

        let newline = if self.content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut output = result.join(newline);
        if trailing_newline
            || !result.is_empty()
                && changes
                    .iter()
                    .any(|change| matches!(change, TaskChange::Add { .. }))
        {
            output.push_str(newline);
        }
        Ok(output)
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

fn render_task_line(line: &str, title: &str, checked: bool) -> String {
    let marker_start = line.find("- [").unwrap_or(0);
    let checkbox_offset = marker_start + 3;
    format!(
        "{}{}] {}",
        &line[..checkbox_offset],
        if checked { 'x' } else { ' ' },
        title.trim()
    )
}

pub(crate) fn atomic_write(path: &Path, content: &str) -> Result<(), TodoError> {
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
            .expect("the system clock must be available")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "taskhud-core-{label}-{}-{nonce}-TODO.md",
            std::process::id()
        ))
    }

    #[test]
    fn parse_tasks_preserves_ids_and_crlf_content() {
        let content =
            "# Today\r\n\r\n- [ ] Write the demo — draft\r\n- [x] Review the contract\r\n";
        let document = TodoDocument::from_content("TODO.md", content.to_owned()).unwrap();
        let snapshot = document.snapshot();

        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.tasks[0].id, "task-2");
        assert_eq!(snapshot.tasks[0].title, "Write the demo — draft");
        assert!(!snapshot.tasks[0].checked);
        assert_eq!(snapshot.tasks[1].id, "task-3");
        assert!(snapshot.tasks[1].checked);
        assert_eq!(snapshot.content, content);
    }

    #[test]
    fn toggle_task_changes_only_the_checkbox() {
        let content = "- [ ] First\n- [x] Second\n";
        let document = TodoDocument::from_content("TODO.md", content.to_owned()).unwrap();

        assert_eq!(
            document.toggle_task("task-0").unwrap(),
            "- [x] First\n- [x] Second\n"
        );
        assert_eq!(
            document.toggle_task("task-1").unwrap(),
            "- [ ] First\n- [ ] Second\n"
        );
        assert!(matches!(
            document.toggle_task("task-99"),
            Err(TodoError::TaskNotFound(_))
        ));
    }

    #[test]
    fn append_task_keeps_the_existing_newline_style() {
        let document = TodoDocument::from_content("TODO.md", "- [ ] Start\r\n".to_owned()).unwrap();
        assert_eq!(
            document.append_task("  New task  ").unwrap(),
            "- [ ] Start\r\n- [ ] New task\r\n"
        );
    }

    #[test]
    fn save_if_unchanged_writes_and_rejects_stale_versions() {
        let path = temp_todo_path("save");
        let initial = "- [ ] Start\n";
        fs::write(&path, initial).unwrap();

        let document = TodoDocument::load(&path).unwrap();
        let saved =
            TodoDocument::save_if_unchanged(&path, &document.snapshot().version, "- [x] Start\n")
                .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [x] Start\n");
        assert!(saved.snapshot().tasks[0].checked);

        fs::write(&path, "- [ ] External change\n").unwrap();
        let error = TodoDocument::save_if_unchanged(
            &path,
            &saved.snapshot().version,
            "- [x] Overwrite forbidden\n",
        )
        .unwrap_err();
        assert!(matches!(error, TodoError::Conflict { .. }));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] External change\n"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_todo_path_rejects_other_files() {
        assert!(ensure_todo_path("notes.md").is_err());
        assert!(ensure_todo_path("todo.md").is_ok());
        assert!(ensure_todo_path("TODO.md").is_ok());
    }

    #[test]
    fn proposals_are_persisted_and_duplicate_pending_changes_are_reused() {
        let root = std::env::temp_dir().join(format!(
            "taskhud-core-proposals-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut store = ProposalStore::open(&root).unwrap();
        let changes = vec![TaskChange::Add {
            title: "Add a check".to_owned(),
            after_task_id: None,
        }];

        let first = store
            .add(
                "demo".to_owned(),
                "version-1".to_owned(),
                "Audit finished".to_owned(),
                changes.clone(),
            )
            .unwrap();
        let duplicate = store
            .add(
                "demo".to_owned(),
                "version-1".to_owned(),
                "Audit finished".to_owned(),
                changes,
            )
            .unwrap();
        assert_eq!(first, duplicate);
        assert_eq!(store.list().len(), 1);

        let reloaded = ProposalStore::open(&root).unwrap();
        assert_eq!(reloaded.get(&first.id).unwrap(), &first);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_changes_updates_deletes_and_adds_tasks_without_rewriting_other_lines() {
        let content = "# Revision\n\n- [ ] Read the lesson\n- [x] Do an exercise\n";
        let document = TodoDocument::from_content("TODO.md", content.to_owned()).unwrap();
        let changes = vec![
            TaskChange::Update {
                task_id: "task-2".to_owned(),
                title: Some("Reread the lesson".to_owned()),
                checked: Some(true),
            },
            TaskChange::Delete {
                task_id: "task-3".to_owned(),
            },
            TaskChange::Add {
                title: "Make a summary sheet".to_owned(),
                after_task_id: Some("task-2".to_owned()),
            },
        ];

        assert_eq!(
            document.apply_changes(&changes).unwrap(),
            "# Revision\n\n- [x] Reread the lesson\n- [ ] Make a summary sheet\n"
        );
    }

    #[test]
    fn proposal_status_is_persisted_and_pending_list_excludes_decided_items() {
        let root = std::env::temp_dir().join(format!(
            "taskhud-core-proposal-status-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut store = ProposalStore::open(&root).unwrap();
        let proposal = store
            .add(
                "demo".to_owned(),
                "version-1".to_owned(),
                "Revise".to_owned(),
                vec![TaskChange::Add {
                    title: "Make a summary sheet".to_owned(),
                    after_task_id: None,
                }],
            )
            .unwrap();

        assert_eq!(store.pending().len(), 1);
        let updated = store
            .set_status(&proposal.id, ProposalStatus::Accepted)
            .unwrap();
        assert_eq!(updated.status, ProposalStatus::Accepted);
        assert!(store.pending().is_empty());
        assert_eq!(
            ProposalStore::open(&root)
                .unwrap()
                .get(&proposal.id)
                .unwrap()
                .status,
            ProposalStatus::Accepted
        );

        let _ = fs::remove_dir_all(root);
    }
}
