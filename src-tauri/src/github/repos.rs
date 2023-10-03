use super::user::Repository;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    OctocrabError(#[from] octocrab::Error),
    #[error("No owner was provided")]
    NoOwnerError,
}

pub async fn list_commits(
    token: &str,
    repository: &Repository,
    branch: &str,
) -> Result<Vec<String>, Error> {
    let owner = match repository.clone().owner {
        Some(o) => o,
        None => return Err(Error::NoOwnerError),
    };
    let repo_name = repository.clone().name;
    let octo = octocrab::OctocrabBuilder::default()
        .personal_token(token.into())
        .build()?;
    let mut page = 1u32;
    let mut number_pages = 1u32;
    let mut commits = Vec::new();
    while page <= number_pages {
        let octo = octo.clone();
        let repo = octo.repos(owner.clone(), repo_name.clone());
        let builder = repo
            .list_commits()
            .branch(branch.clone())
            .per_page(100)
            .page(page);
        let commits_page = match builder.send().await {
            Ok(c) => c,
            Err(e) => {
                log::info!(
                    "[list_commits] repo: {}/{}, page: {}",
                    owner,
                    repo_name,
                    page
                );
                return Err(Error::OctocrabError(e));
            }
        };
        number_pages = match commits_page.number_of_pages() {
            Some(n) => n,
            None => 1,
        };
        log::debug!("l_commits: number_pages: {}", number_pages);
        commits.extend(commits_page.into_iter().map(|c| c.sha));

        page += 1;
    }

    Ok(commits)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Relation {
    Ahead,
    Behind,
    Same,
    Null,
}
pub async fn remote_branch_relation(
    token: &str,
    repository: &Repository,
    branch: &str,
    current: &str,
) -> Result<Relation, Error> {
    if !is_remote_branch(token, repository, branch).await? {
        return Ok(Relation::Null);
    }
    let remote_commits = list_commits(token, repository, branch).await?;
    log::info!(
        "[remote_branch_relation] remote_commits: {:?}",
        remote_commits
    );
    log::info!(
        "[remote_branch_relation] branch: {} current: {}, remote_commits.last(): {:?}",
        branch,
        current,
        remote_commits.last()
    );
    if remote_commits.is_empty() {
        Ok(Relation::Null)
    } else if remote_commits.first() == Some(&current.to_string()) {
        Ok(Relation::Same)
    } else if remote_commits.contains(&current.to_string()) {
        Ok(Relation::Behind)
    } else {
        Ok(Relation::Ahead)
    }
}

pub async fn is_remote_branch(
    token: &str,
    repository: &Repository,
    branch: &str,
) -> Result<bool, Error> {
    let owner = match repository.clone().owner {
        Some(o) => o,
        None => return Err(Error::NoOwnerError),
    };
    let repo_name = repository.clone().name;
    let octo = octocrab::OctocrabBuilder::default()
        .personal_token(token.into())
        .build()?;
    let repo = octo.repos(owner.clone(), repo_name.clone());
    let builder = repo.list_branches().per_page(100);
    let branches = match builder.send().await {
        Ok(c) => c,
        Err(e) => {
            log::info!("[is_remote_branch] repo: {}/{}", owner, repo_name);
            return Err(Error::OctocrabError(e));
        }
    };
    Ok(branches.into_iter().any(|b| b.name == branch))
}
