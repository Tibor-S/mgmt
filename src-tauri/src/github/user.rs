#[derive(Debug, Clone, std::default::Default)]
pub struct ListParameters {
    pub visibility: Option<String>,
    pub affiliation: Option<String>,
    pub repo_type: Option<String>,
    pub sort: Option<String>,
    pub direction: Option<String>,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
    pub since: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub before: Option<chrono::DateTime<chrono::offset::Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    OctocrabError(#[from] octocrab::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Repository {
    pub id: u32,
    pub name: String,
    pub url: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub ssh_url: Option<String>,
    pub visibility: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::offset::Utc>>,
}
pub async fn list_repos(token: &str, params: &ListParameters) -> Result<Vec<Repository>, Error> {
    let octo = octocrab::OctocrabBuilder::default()
        .personal_token(token.into())
        .build()?;
    let mut builder = octo.current().list_repos_for_authenticated_user();
    if let Some(a) = &params.visibility {
        builder = builder.visibility(a.as_str());
    }
    if let Some(a) = &params.affiliation {
        builder = builder.affiliation(a.as_str());
    }
    if let Some(a) = &params.repo_type {
        builder = builder.type_(a.as_str());
    }
    if let Some(a) = &params.sort {
        builder = builder.sort(a.as_str());
    }
    if let Some(a) = &params.direction {
        builder = builder.direction(a.as_str());
    }
    if let Some(a) = &params.per_page {
        builder = builder.per_page(*a as u8);
    }
    if let Some(a) = &params.page {
        builder = builder.page(*a as u8);
    }
    if let Some(a) = &params.since {
        builder = builder.since(*a);
    }
    if let Some(a) = &params.before {
        builder = builder.before(*a);
    }
    let page = builder.send().await?;

    let repos = page
        .items
        .iter()
        .map(|repo| {
            let id = repo.id.0 as u32;
            let name = repo.name.clone();
            let url = repo
                .full_name
                .clone()
                .and_then(|name| Some(String::from("github.com/") + name.as_str()));
            let owner = repo.owner.clone().and_then(|owner| Some(owner.login));
            let description = repo.description.clone();
            let ssh_url = repo.ssh_url.clone();
            let visibility = repo.visibility.clone();
            let created_at = repo.created_at;
            let updated_at = repo.updated_at;

            Repository {
                id,
                name,
                url,
                owner,
                description,
                ssh_url,
                visibility,
                created_at,
                updated_at,
            }
        })
        .collect::<Vec<Repository>>();

    Ok(repos)
}
