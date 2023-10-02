use std::{collections::HashSet, fs, path::PathBuf};

use crate::github::user::{self, list_repos, ListParameters, Repository};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalProject {
    pub path: PathBuf,
    pub git: Option<GitInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub local: Option<LocalProject>,
    pub remote: Option<Repository>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitInfo {
    pub changes: Vec<FileInfo>,
    pub remotes: Vec<RemoteInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: Option<String>,
    pub status: FileStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub url_type: RemoteUrlType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RemoteUrlType {
    HTTP,
    SSH,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FileStatus {
    Current,
    IndexNew,
    IndexModified,
    IndexDeleted,
    IndexRenamed,
    IndexTypeChange,
    WorkTreeNew,
    WorkTreeModified,
    WorkTreeDeleted,
    WorkTreeTypeChange,
    WorkTreeRenamed,
    Ignored,
    Conflict,
}

impl From<git2::Status> for FileStatus {
    fn from(status: git2::Status) -> Self {
        match status {
            git2::Status::CURRENT => FileStatus::Current,
            git2::Status::INDEX_NEW => FileStatus::IndexNew,
            git2::Status::INDEX_MODIFIED => FileStatus::IndexModified,
            git2::Status::INDEX_DELETED => FileStatus::IndexDeleted,
            git2::Status::INDEX_RENAMED => FileStatus::IndexRenamed,
            git2::Status::INDEX_TYPECHANGE => FileStatus::IndexTypeChange,
            git2::Status::WT_NEW => FileStatus::WorkTreeNew,
            git2::Status::WT_MODIFIED => FileStatus::WorkTreeModified,
            git2::Status::WT_DELETED => FileStatus::WorkTreeDeleted,
            git2::Status::WT_TYPECHANGE => FileStatus::WorkTreeTypeChange,
            git2::Status::WT_RENAMED => FileStatus::WorkTreeRenamed,
            git2::Status::IGNORED => FileStatus::Ignored,
            git2::Status::CONFLICTED => FileStatus::Conflict,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    GithubError(#[from] user::Error),
}

pub fn list_local_projects(path: &PathBuf) -> Result<Vec<LocalProject>, Error> {
    let v = fs::read_dir(path)?
        .filter_map(|dir| {
            let dir = dir
                .or_else(|e| {
                    log::error!("{:?}", e);
                    Err(e)
                })
                .ok()?;
            let path = PathBuf::from(dir.path());
            if !path.is_dir() {
                return None;
            }
            if !path.join(".git").is_dir() {
                return Some(LocalProject { path, git: None });
            }

            let repository = git2::Repository::open(path.clone())
                .or_else(|e| {
                    log::error!("{:?}", e);
                    Err(e)
                })
                .ok()?;

            let remote_names = match repository.remotes() {
                Ok(rs) => rs
                    .iter()
                    .filter_map(|r| r.map(|r| r.to_string()))
                    .collect::<Vec<String>>(),
                Err(e) => {
                    log::error!("{:?}", e);
                    Vec::new()
                }
            };
            let remotes = remote_names
                .iter()
                .filter_map(|name| {
                    let remote = repository.find_remote(name).ok()?;
                    let url = remote.url()?.to_string();
                    let url_type = match url.get(0..4) {
                        Some("http") => RemoteUrlType::HTTP,
                        Some("git@") => RemoteUrlType::SSH,
                        _ => return None,
                    };

                    Some(RemoteInfo {
                        name: name.clone(),
                        url,
                        url_type,
                    })
                })
                .collect();
            let changes = match repository.statuses(None) {
                Ok(ss) => ss
                    .iter()
                    .map(|s| FileInfo {
                        path: s.path().map(|s| s.to_string()),
                        status: s.status().into(),
                    })
                    .collect::<Vec<FileInfo>>(),
                Err(e) => {
                    log::error!("{:?}", e);
                    Vec::new()
                }
            };

            let git = Some(GitInfo { changes, remotes });

            Some(LocalProject { path, git })
        })
        .collect::<Vec<LocalProject>>();

    Ok(v)
}

pub async fn list_projects(
    github_token: &str,
    paths: &[PathBuf],
    remote_params: &ListParameters,
) -> Result<Vec<Project>, Error> {
    let local_projects = paths
        .iter()
        .filter_map(|pth| {
            list_local_projects(pth)
                .or_else(|e| {
                    log::error!("{:?}", e);
                    Err(e)
                })
                .ok()
        })
        .flatten()
        .collect::<Vec<LocalProject>>();

    let remote_projects = list_repos(github_token, remote_params).await?;

    let mut id_matches: HashSet<u32> = HashSet::new();
    let mut projects: Vec<Project> = Vec::new();

    for lp in local_projects {
        let mut project = Project {
            local: Some(lp.clone()),
            remote: None,
        };
        let remote = match lp
            .clone()
            .git
            .and_then(|g| g.remotes.get(0).cloned())
            .map(|r| r.clone())
        {
            Some(r) => r,
            None => {
                projects.push(project);
                continue;
            }
        };
        let url = remote.url.clone();
        let rp = match remote.url_type {
            RemoteUrlType::HTTP => remote_projects.iter().find(|r| match r.url.clone() {
                Some(u) => match_remote_url(&*u, &*url.clone()),
                None => false,
            }),
            RemoteUrlType::SSH => remote_projects.iter().find(|r| match r.ssh_url.clone() {
                Some(u) => match_remote_url(&*u, &*url.clone()),
                None => false,
            }),
        };

        project.remote = rp.map(|r| {
            id_matches.insert(r.id);
            r.clone()
        });

        projects.push(project)
    }

    for rp in remote_projects {
        if id_matches.contains(&rp.id) {
            continue;
        }
        projects.push(Project {
            local: None,
            remote: Some(rp.clone()),
        });
    }

    Ok(projects)
}

pub fn match_remote_url(url_a: &str, url_b: &str) -> bool {
    let mut a = if url_a.ends_with(".git") {
        url_a.to_string()
    } else {
        url_a.to_string() + ".git"
    };
    let mut b = if url_b.ends_with(".git") {
        url_b.to_string()
    } else {
        url_b.to_string() + ".git"
    };

    a = if a.starts_with("https://") {
        a[8..].to_string()
    } else if a.starts_with("http://") {
        a[7..].to_string()
    } else {
        a
    };
    b = if b.starts_with("https://") {
        b[8..].to_string()
    } else if a.starts_with("http://") {
        b[7..].to_string()
    } else {
        b
    };
    a == b
}
