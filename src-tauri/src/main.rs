// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use github::repos;
use project::{Project, Projects};
use tauri::async_runtime::block_on;
use uuid::Uuid;

use crate::github::user::ListParameters;

mod github;
mod project;

struct TokenState(Arc<Mutex<Option<String>>>);
struct ProjectDirState(Arc<Mutex<Vec<PathBuf>>>);
struct ProjectsState(Arc<Mutex<Projects>>);

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
    #[error("Error occured while listing remote commits")]
    RemoteCommitsError,
    #[error("Error occured while getting changes")]
    ChangesError,
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
    projects.extend(new_projects.into_iter());
    Ok(())
}

#[tauri::command]
fn project_ids(projects_state: tauri::State<ProjectsState>) -> Result<Vec<Uuid>, Error> {
    let projects = &*projects_state.0.lock().unwrap();
    Ok(projects.ids())
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
    Ok(prj.and_then(|p| p.remote_name()))
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
    Ok(prj.and_then(|p| p.local_name()))
}

#[tauri::command]
fn project_local_commits(
    projects_state: tauri::State<ProjectsState>,
    id: String,
) -> Result<Option<HashMap<String, String>>, Error> {
    let key = Uuid::try_parse(&id).map_err(|e| {
        log::error!("{:?}", e);
        Error::UuidParseError
    })?;
    let prj;
    {
        let projects = projects_state.0.lock().unwrap();
        prj = Some(projects.get(&key).ok_or(Error::UuidNoMatch)?.clone());
    }
    Ok(prj.and_then(|p| p.local_commits()))
}

#[tauri::command]
fn branch_relation(
    token_state: tauri::State<TokenState>,
    projects_state: tauri::State<ProjectsState>,
    id: String,
    branch: String,
    current: String,
) -> Result<repos::Relation, Error> {
    log::info!("branch_relation");
    log::info!("id: {}", id);
    log::info!("branch: {}", branch);
    log::info!("current: {}", current);
    let key = Uuid::try_parse(&id).map_err(|e| {
        log::error!("{:?}", e);
        Error::UuidParseError
    })?;
    let prj;
    {
        let projects = projects_state.0.lock().unwrap();
        prj = projects.get(&key).ok_or(Error::UuidNoMatch)?.clone();
    }
    let repository = match prj.remote.clone() {
        Some(r) => r,
        None => return Ok(repos::Relation::Null),
    };
    let thread_token = token_state.0.clone();
    let token_guard = thread_token.lock().unwrap();
    let token = match &*token_guard {
        Some(t) => t,
        None => return Err(Error::NoTokenError),
    };

    let task = match block_on(repos::remote_branch_relation(
        token,
        &repository,
        &*branch,
        &current,
    )) {
        Ok(c) => c,
        Err(e) => {
            log::error!("{:?}", e);
            return Err(Error::RemoteCommitsError);
        }
    };

    Ok(task)
}

#[tauri::command]
fn project_changes(
    projects_state: tauri::State<ProjectsState>,
    id: String,
) -> Result<usize, Error> {
    let key = Uuid::try_parse(&id).map_err(|e| {
        log::error!("{:?}", e);
        Error::UuidParseError
    })?;
    let prj;
    {
        let projects = projects_state.0.lock().unwrap();
        prj = projects.get(&key).ok_or(Error::UuidNoMatch)?.clone();
    }
    let c = prj.number_of_changes();
    match c {
        Ok(c) => Ok(c),
        Err(e) => {
            log::error!("{}", e);
            Err(Error::ChangesError)
        }
    }
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
            r"/Users/sebastian/Documents/prj/",
        )]))))
        .manage(ProjectsState(Arc::new(Mutex::new(Projects::default()))))
        .invoke_handler(tauri::generate_handler![
            update_projects,
            project_ids,
            project_remote_name,
            project_local_name,
            project_local_commits,
            branch_relation,
            project_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
