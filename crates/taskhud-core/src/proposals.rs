use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{TodoError, atomic_write};

/// How long a writer waits for another process to release `proposals.json.lock`.
const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY: Duration = Duration::from_millis(10);
/// A lock older than this was left by a process that died while holding it.
const STALE_LOCK_AGE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum TaskChange {
    Add {
        title: String,
        after_task_id: Option<String>,
    },
    Update {
        task_id: String,
        title: Option<String>,
        checked: Option<bool>,
    },
    Delete {
        task_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskProposal {
    pub id: String,
    pub project_id: String,
    pub base_version: String,
    pub reason: String,
    pub changes: Vec<TaskChange>,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProposalFile {
    proposals: Vec<TaskProposal>,
}

/// Proposals of one project, stored in `<root>/.taskhud/proposals.json`.
///
/// The overlay, `taskhud-mcp` and `taskhud-functions` may write the same file.
/// Every write therefore takes an exclusive lock file, reloads the file and
/// applies its change to what is on disk, so no process loses another's write.
#[derive(Debug, Clone)]
pub struct ProposalStore {
    path: PathBuf,
    proposals: Vec<TaskProposal>,
}

impl ProposalStore {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, TodoError> {
        let path = project_root
            .as_ref()
            .join(".taskhud")
            .join("proposals.json");
        let proposals = read_proposals(&path)?;
        Ok(Self { path, proposals })
    }

    pub fn list(&self) -> &[TaskProposal] {
        &self.proposals
    }

    pub fn pending(&self) -> Vec<TaskProposal> {
        self.proposals
            .iter()
            .filter(|proposal| proposal.status == ProposalStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn get(&self, proposal_id: &str) -> Result<&TaskProposal, TodoError> {
        self.proposals
            .iter()
            .find(|proposal| proposal.id == proposal_id)
            .ok_or_else(|| TodoError::ProposalNotFound(proposal_id.to_owned()))
    }

    pub fn add(
        &mut self,
        project_id: String,
        base_version: String,
        reason: String,
        changes: Vec<TaskChange>,
    ) -> Result<TaskProposal, TodoError> {
        let _lock = self.lock()?;
        self.proposals = read_proposals(&self.path)?;

        if let Some(existing) = self.proposals.iter().find(|proposal| {
            proposal.status == ProposalStatus::Pending
                && proposal.project_id == project_id
                && proposal.base_version == base_version
                && proposal.reason == reason
                && proposal.changes == changes
        }) {
            return Ok(existing.clone());
        }

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();
        let proposal = TaskProposal {
            id: format!("proposal-{created_at}-{}", self.proposals.len()),
            project_id,
            base_version,
            reason,
            changes,
            status: ProposalStatus::Pending,
            created_at,
        };
        self.proposals.push(proposal.clone());
        self.save()?;
        Ok(proposal)
    }

    pub fn set_status(
        &mut self,
        proposal_id: &str,
        status: ProposalStatus,
    ) -> Result<TaskProposal, TodoError> {
        let _lock = self.lock()?;
        self.proposals = read_proposals(&self.path)?;

        let proposal = self
            .proposals
            .iter_mut()
            .find(|proposal| proposal.id == proposal_id)
            .ok_or_else(|| TodoError::ProposalNotFound(proposal_id.to_owned()))?;
        proposal.status = status;
        let updated = proposal.clone();
        self.save()?;
        Ok(updated)
    }

    fn lock(&self) -> Result<StoreLock, TodoError> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| TodoError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
        StoreLock::acquire(self.path.with_extension("json.lock"))
    }

    fn save(&self) -> Result<(), TodoError> {
        let content = serde_json::to_string_pretty(&ProposalFile {
            proposals: self.proposals.clone(),
        })
        .map_err(|source| TodoError::InvalidConfig {
            path: self.path.clone(),
            message: source.to_string(),
        })?;
        atomic_write(&self.path, &format!("{content}\n"))
    }
}

fn read_proposals(path: &Path) -> Result<Vec<TaskProposal>, TodoError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(TodoError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let file: ProposalFile =
        serde_json::from_str(&content).map_err(|source| TodoError::InvalidConfig {
            path: path.to_path_buf(),
            message: source.to_string(),
        })?;
    Ok(file.proposals)
}

/// Exclusive lock held by creating a file with `create_new`; removed on drop.
struct StoreLock {
    path: PathBuf,
}

impl StoreLock {
    fn acquire(path: PathBuf) -> Result<Self, TodoError> {
        let deadline = SystemTime::now() + LOCK_TIMEOUT;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    if is_stale(&path) {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if SystemTime::now() >= deadline {
                        return Err(TodoError::Write {
                            path: path.clone(),
                            source: std::io::Error::new(
                                ErrorKind::TimedOut,
                                "another TaskHUD process is still writing the proposals",
                            ),
                        });
                    }
                    thread::sleep(LOCK_RETRY);
                }
                // Windows reports a lock file being deleted by its owner as access denied.
                Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                    if SystemTime::now() >= deadline {
                        return Err(TodoError::Write {
                            path: path.clone(),
                            source: error,
                        });
                    }
                    thread::sleep(LOCK_RETRY);
                }
                Err(source) => {
                    return Err(TodoError::Write {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
    }
}

fn is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > STALE_LOCK_AGE)
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "taskhud-proposals-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn add_change(title: &str) -> Vec<TaskChange> {
        vec![TaskChange::Add {
            title: title.to_owned(),
            after_task_id: None,
        }]
    }

    #[test]
    fn two_stores_on_the_same_root_keep_each_other_writes() {
        let root = temp_root("two-stores");
        let mut app = ProposalStore::open(&root).unwrap();
        let mut connector = ProposalStore::open(&root).unwrap();

        let first = app
            .add(
                "demo".into(),
                "v1".into(),
                "From the app".into(),
                add_change("A"),
            )
            .unwrap();
        // `connector` was opened before `first` existed.
        let second = connector
            .add(
                "demo".into(),
                "v1".into(),
                "From a connector".into(),
                add_change("B"),
            )
            .unwrap();
        assert_ne!(first.id, second.id);

        // `app` has never seen `second`, `connector` has a stale `first`.
        app.set_status(&second.id, ProposalStatus::Rejected)
            .unwrap();
        connector
            .set_status(&first.id, ProposalStatus::Accepted)
            .unwrap();

        let on_disk = ProposalStore::open(&root).unwrap();
        assert_eq!(on_disk.list().len(), 2);
        assert_eq!(
            on_disk.get(&first.id).unwrap().status,
            ProposalStatus::Accepted
        );
        assert_eq!(
            on_disk.get(&second.id).unwrap().status,
            ProposalStatus::Rejected
        );
        assert!(!root.join(".taskhud").join("proposals.json.lock").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_writers_do_not_lose_proposals() {
        let root = temp_root("concurrent");
        let writers: Vec<_> = (0..4)
            .map(|writer| {
                let root = root.clone();
                std::thread::spawn(move || {
                    let mut store = ProposalStore::open(&root).unwrap();
                    for round in 0..10 {
                        store
                            .add(
                                "demo".into(),
                                "v1".into(),
                                format!("writer {writer} round {round}"),
                                add_change("Task"),
                            )
                            .unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }

        let on_disk = ProposalStore::open(&root).unwrap();
        assert_eq!(on_disk.list().len(), 40);
        let mut ids: Vec<_> = on_disk.list().iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 40, "proposal ids must be unique");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_stale_lock_is_taken_over() {
        let root = temp_root("stale-lock");
        let lock = root.join(".taskhud").join("proposals.json.lock");
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(&lock, "").unwrap();
        let old = SystemTime::now() - STALE_LOCK_AGE - std::time::Duration::from_secs(1);
        fs::File::options()
            .write(true)
            .open(&lock)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let mut store = ProposalStore::open(&root).unwrap();
        store
            .add(
                "demo".into(),
                "v1".into(),
                "After a crash".into(),
                add_change("A"),
            )
            .unwrap();
        assert!(!lock.exists());
        let _ = fs::remove_dir_all(root);
    }
}
