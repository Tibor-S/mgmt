// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use project::Project;
use tauri::async_runtime::block_on;
use uuid::Uuid;

use crate::github::user::ListParameters;

mod github;
mod project;

struct TokenState(Arc<Mutex<Option<String>>>);
struct ProjectDirState(Arc<Mutex<Vec<PathBuf>>>);
struct ProjectsState(Arc<Mutex<HashMap<Uuid, Project>>>);

#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum Error {
    #[error("Error occured while querying repos")]
    QueryReposError,
    #[error("No personal access token was provided")]
    NoTokenError,
    #[error("Error occured while acquiring projects")]
    AcquireProjectsError,
    #[error("Error occured when parsing uuid")]
    UuidParseError,
    #[error("Error occured when matching uuid to project")]
    UuidNoMatch,
    #[error("Could not parse path to string")]
    PathParseError,
    #[error("Invalid path")]
    InvalidPathError,
}

#[tauri::command]
fn update_projects(
    token_state: tauri::State<TokenState>,
    projects_state: tauri::State<ProjectsState>,
    project_dirs: tauri::State<ProjectDirState>,
) -> Result<(), Error> {
    log::debug!("update_projects");
    let thread_token = token_state.0.clone();
    let token_guard = thread_token.lock().unwrap();
    let token = match &*token_guard {
        Some(t) => t,
        None => return Err(Error::NoTokenError),
    };

    let thread_dirs = project_dirs.0.clone();
    let dirs = &*thread_dirs.lock().unwrap();

    let task = block_on(project::list_projects(
        &token,
        &dirs,
        &ListParameters {
            visibility: Some("all".into()),
            affiliation: None,
            repo_type: None,
            sort: None,
            direction: None,
            per_page: Some(100),
            page: None,
            since: None,
            before: None,
        },
    ));
    let new_projects = match task {
        Ok(p) => p,
        Err(e) => {
            log::error!("{:?}", e);
            return Err(Error::AcquireProjectsError);
        }
    };
    let projects = &mut *projects_state.0.lock().unwrap();
    projects.clear();
    for p in new_projects.iter() {
        let key = Uuid::new_v4();
        projects.insert(key, p.clone());
    }
    Ok(())
}

#[tauri::command]
fn project_ids(projects_state: tauri::State<ProjectsState>) -> Result<Vec<Uuid>, Error> {
    let projects = &*projects_state.0.lock().unwrap();
    Ok(projects.keys().cloned().collect())
}

#[tauri::command]
fn project_remote_name(
    projects_state: tauri::State<ProjectsState>,
    id: String,
) -> Result<Option<String>, Error> {
    let key = Uuid::try_parse(&id).map_err(|e| {
        log::error!("{:?}", e);
        Error::UuidParseError
    })?;
    let prj;
    {
        let projects = projects_state.0.lock().unwrap();
        prj = Some(projects.get(&key).ok_or(Error::UuidNoMatch)?.clone());
    }
    let remote = prj.clone().and_then(|p| p.remote);
    let remote_name = remote.map(|r| r.name);
    Ok(remote_name)
}

#[tauri::command]
fn project_local_name(
    projects_state: tauri::State<ProjectsState>,
    id: String,
) -> Result<Option<String>, Error> {
    let key = Uuid::try_parse(&id).map_err(|e| {
        log::error!("{:?}", e);
        Error::UuidParseError
    })?;
    let prj;
    {
        let projects = projects_state.0.lock().unwrap();
        prj = Some(projects.get(&key).ok_or(Error::UuidNoMatch)?.clone());
    }
    let local = match prj.clone().and_then(|p| p.local) {
        Some(l) => l,
        None => return Ok(None),
    };
    let path = local.path;
    let file_name = path
        .file_name()
        .ok_or(Error::InvalidPathError)?
        .to_str()
        .ok_or(Error::PathParseError)?;
    let name = file_name.to_string();
    Ok(Some(name))
}

fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let token = match dotenv::var("token") {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("{}", e);
            None
        }
    };
    log::debug!("{:?}", token);
    tauri::Builder::default()
        .manage(TokenState(Arc::new(Mutex::new(token))))
        .manage(ProjectDirState(Arc::new(Mutex::new(vec![PathBuf::from(
            r"C:\Users\Sebastian\Documents\Projects",
        )]))))
        .manage(ProjectsState(Arc::new(Mutex::new(HashMap::new()))))
        .invoke_handler(tauri::generate_handler![
            update_projects,
            project_ids,
            project_remote_name,
            project_local_name,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
