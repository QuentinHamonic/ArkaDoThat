use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use taskhud_core::{
    Project, ProjectRegistry, ProposalStatus, ProposalStore, TodoDocument, TodoError, TodoSnapshot,
    ensure_todo_path,
};

fn command_error(error: TodoError) -> String {
    error.to_string()
}

fn projects_config_path() -> PathBuf {
    std::env::var_os(taskhud_core::CONFIG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(taskhud_core::default_config_path)
}

fn load_project_registry() -> Result<ProjectRegistry, String> {
    ProjectRegistry::load(projects_config_path()).map_err(command_error)
}

fn save_project_registry(registry: &ProjectRegistry) -> Result<(), String> {
    registry.save(projects_config_path()).map_err(command_error)
}

fn internal_project_root_at(config_path: &Path, project_id: &str) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("projects")
        .join(project_id)
}

fn resolve_project_root(
    root: &str,
    project_id: &str,
    config_path: &Path,
) -> Result<PathBuf, String> {
    if root.trim().is_empty() {
        return Ok(internal_project_root_at(config_path, project_id));
    }

    let root = PathBuf::from(root.trim());
    if !root.is_dir() {
        return Err("The project folder does not exist.".to_owned());
    }
    Ok(root)
}

#[tauri::command]
fn read_todo(path: String) -> Result<TodoSnapshot, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    TodoDocument::load(path)
        .map(|document| document.snapshot())
        .map_err(command_error)
}

#[tauri::command]
fn save_todo(
    path: String,
    expected_version: String,
    content: String,
) -> Result<TodoSnapshot, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    TodoDocument::save_if_unchanged(path, &expected_version, &content)
        .map(|document| document.snapshot())
        .map_err(command_error)
}

#[tauri::command]
fn list_projects() -> Result<ProjectRegistry, String> {
    load_project_registry()
}

#[tauri::command]
fn create_project(name: String, root: String) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The project name cannot be empty.".to_owned());
    }

    let mut registry = load_project_registry()?;
    if registry
        .projects
        .iter()
        .any(|project| project.name.eq_ignore_ascii_case(name))
    {
        return Err("A project with this name already exists.".to_owned());
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let project_id = format!("project-{timestamp}");
    let root = resolve_project_root(root.trim(), &project_id, &projects_config_path())?;

    fs::create_dir_all(&root)
        .map_err(|error| format!("Cannot prepare {}: {error}", root.display()))?;

    let todo_path = root.join("TODO.md");
    if !todo_path.exists() {
        fs::write(&todo_path, format!("# {name}\n\n## To do\n\n"))
            .map_err(|error| format!("Cannot create {}: {error}", todo_path.display()))?;
    }

    let project = Project {
        id: project_id,
        name: name.to_owned(),
        root,
    };
    let should_activate = registry.active_project_id.is_none();
    registry.projects.push(project.clone());
    if should_activate {
        registry.active_project_id = Some(project.id.clone());
    }
    save_project_registry(&registry)?;
    Ok(project)
}

#[tauri::command]
fn set_active_project(project_id: String) -> Result<Project, String> {
    let mut registry = load_project_registry()?;
    let project = registry
        .project(&project_id)
        .map_err(command_error)?
        .clone();
    registry
        .set_active_project(project_id)
        .map_err(command_error)?;
    save_project_registry(&registry)?;
    Ok(project)
}

fn proposal_store_for(path: &Path) -> Result<ProposalStore, String> {
    let root = path
        .parent()
        .ok_or_else(|| "the TODO.md file has no parent folder".to_owned())?;
    ProposalStore::open(root).map_err(command_error)
}

#[tauri::command]
fn list_pending_proposals(path: String) -> Result<Vec<taskhud_core::TaskProposal>, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    proposal_store_for(&path).map(|store| store.pending())
}

#[tauri::command]
fn accept_proposal(path: String, proposal_id: String) -> Result<TodoSnapshot, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    let mut store = proposal_store_for(&path)?;
    let proposal = store.get(&proposal_id).map_err(command_error)?.clone();
    if proposal.status != ProposalStatus::Pending {
        return Err("This proposal has already been decided.".to_owned());
    }

    let document = TodoDocument::load(&path).map_err(command_error)?;
    let snapshot = document.snapshot();
    if snapshot.version != proposal.base_version {
        return Err(command_error(TodoError::Conflict {
            expected: proposal.base_version,
            actual: snapshot.version,
        }));
    }
    let content = document
        .apply_changes(&proposal.changes)
        .map_err(command_error)?;
    let saved = TodoDocument::save_if_unchanged(&path, &snapshot.version, &content)
        .map_err(command_error)?;
    store
        .set_status(&proposal.id, ProposalStatus::Accepted)
        .map_err(command_error)?;
    Ok(saved.snapshot())
}

#[tauri::command]
fn reject_proposal(
    path: String,
    proposal_id: String,
) -> Result<Vec<taskhud_core::TaskProposal>, String> {
    let path = ensure_todo_path(Path::new(&path)).map_err(command_error)?;
    let mut store = proposal_store_for(&path)?;
    let proposal = store.get(&proposal_id).map_err(command_error)?.clone();
    if proposal.status != ProposalStatus::Pending {
        return Err("This proposal has already been decided.".to_owned());
    }
    store
        .set_status(&proposal.id, ProposalStatus::Rejected)
        .map_err(command_error)?;
    Ok(store.pending())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_todo,
            save_todo,
            list_projects,
            create_project,
            set_active_project,
            list_pending_proposals,
            accept_proposal,
            reject_proposal
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use taskhud_core::TaskChange;

    fn temp_project(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "taskhud-tauri-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn an_empty_project_root_uses_taskhud_storage() {
        let root = temp_project("standalone");
        let config = root.join("config.json");

        let resolved = resolve_project_root("", "project-test", &config).unwrap();

        assert_eq!(resolved, root.join("projects").join("project-test"));
        assert!(!resolved.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn proposal_commands_apply_all_task_operations_and_record_human_decision() {
        let root = temp_project("decision");
        let todo = root.join("TODO.md");
        fs::write(
            &todo,
            "# Revision\n\n- [ ] Reread the lesson\n- [ ] Make a summary sheet\n- [x] Old exercise\n",
        )
        .unwrap();
        let snapshot = TodoDocument::load(&todo).unwrap().snapshot();
        let mut store = ProposalStore::open(&root).unwrap();
        let proposal = store
            .add(
                "maths".to_owned(),
                snapshot.version.clone(),
                "Revise the four operations".to_owned(),
                vec![
                    TaskChange::Add {
                        title: "Do a review exercise".to_owned(),
                        after_task_id: None,
                    },
                    TaskChange::Update {
                        task_id: "task-2".to_owned(),
                        title: Some("Reread the lesson (revised)".to_owned()),
                        checked: None,
                    },
                    TaskChange::Update {
                        task_id: "task-3".to_owned(),
                        title: None,
                        checked: Some(true),
                    },
                    TaskChange::Delete {
                        task_id: "task-4".to_owned(),
                    },
                ],
            )
            .unwrap();

        let saved = accept_proposal(todo.display().to_string(), proposal.id.clone()).unwrap();
        assert_eq!(saved.tasks.len(), 3);
        assert_eq!(
            fs::read_to_string(&todo).unwrap(),
            "# Revision\n\n- [ ] Reread the lesson (revised)\n- [x] Make a summary sheet\n- [ ] Do a review exercise\n"
        );
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

    #[test]
    fn rejecting_a_proposal_does_not_change_todo() {
        let root = temp_project("reject");
        let todo = root.join("TODO.md");
        let original = "- [ ] Reread the lesson\n";
        fs::write(&todo, original).unwrap();
        let snapshot = TodoDocument::load(&todo).unwrap().snapshot();
        let mut store = ProposalStore::open(&root).unwrap();
        let proposal = store
            .add(
                "maths".to_owned(),
                snapshot.version,
                "Ignore this addition".to_owned(),
                vec![TaskChange::Add {
                    title: "Do not add".to_owned(),
                    after_task_id: None,
                }],
            )
            .unwrap();

        let pending = reject_proposal(todo.display().to_string(), proposal.id.clone()).unwrap();
        assert!(pending.is_empty());
        assert_eq!(fs::read_to_string(&todo).unwrap(), original);
        assert_eq!(
            ProposalStore::open(&root)
                .unwrap()
                .get(&proposal.id)
                .unwrap()
                .status,
            ProposalStatus::Rejected
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepting_a_stale_proposal_does_not_overwrite_external_changes() {
        let root = temp_project("stale");
        let todo = root.join("TODO.md");
        fs::write(&todo, "- [ ] Reread the lesson\n").unwrap();
        let snapshot = TodoDocument::load(&todo).unwrap().snapshot();
        let mut store = ProposalStore::open(&root).unwrap();
        let proposal = store
            .add(
                "maths".to_owned(),
                snapshot.version,
                "Add an exercise".to_owned(),
                vec![TaskChange::Add {
                    title: "Do an exercise".to_owned(),
                    after_task_id: None,
                }],
            )
            .unwrap();

        let external_content = "- [ ] Reread the lesson\n- [ ] External change\n";
        fs::write(&todo, external_content).unwrap();
        let error = accept_proposal(todo.display().to_string(), proposal.id.clone()).unwrap_err();

        assert!(error.contains("the file changed"));
        assert_eq!(fs::read_to_string(&todo).unwrap(), external_content);
        assert_eq!(
            ProposalStore::open(&root)
                .unwrap()
                .get(&proposal.id)
                .unwrap()
                .status,
            ProposalStatus::Pending
        );

        let _ = fs::remove_dir_all(root);
    }
}
