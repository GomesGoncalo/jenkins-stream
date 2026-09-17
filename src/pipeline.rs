use crate::args::Args;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Build {
    number: u32,
    url: Option<String>,
}

impl Build {
    pub fn new(number: u32) -> Self {
        Self { number, url: None }
    }
    pub fn get_number(&self) -> u32 {
        self.number
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub name: String,
    /// Most recent timestamp (ms) of a build triggered by the current user, for sorting.
    pub last_used_by_me: Option<i64>,
}

/// A build with creator information, for interactive discovery.
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub number: u32,
    pub triggered_by: String,
}

// ── API shapes ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Cause {
    #[serde(rename = "userId")]
    user_id: Option<String>,
    #[serde(rename = "shortDescription")]
    short_description: Option<String>,
}

#[derive(Deserialize)]
struct Action {
    causes: Option<Vec<Cause>>,
}

#[derive(Deserialize)]
struct RecentBuild {
    number: u32,
    #[serde(default)]
    timestamp: i64,
    #[serde(default)]
    actions: Vec<Action>,
}

#[derive(Deserialize)]
struct JobDetail {
    name: String,
    #[serde(default)]
    builds: Vec<RecentBuild>,
}

#[derive(Deserialize)]
struct JobDetailList {
    jobs: Vec<JobDetail>,
}

#[derive(Deserialize)]
struct Me {
    id: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn build_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

async fn get_current_user(client: &Client, domain: &str) -> Result<String, Box<dyn Error>> {
    let response = client.get(format!("{domain}/me/api/json")).send().await?;
    let me: Me = serde_json::from_str(&response.text().await?)?;
    Ok(me.id)
}

fn cause_label(actions: &[Action]) -> String {
    for action in actions {
        if let Some(causes) = &action.causes {
            for cause in causes {
                if let Some(uid) = &cause.user_id {
                    return uid.clone();
                }
                if let Some(desc) = &cause.short_description {
                    return desc.clone();
                }
            }
        }
    }
    "unknown".to_string()
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Lists all jobs sorted by when the current API-token user last triggered a build.
/// Jobs never triggered by that user appear at the end.
pub async fn list_jobs(args: &Args) -> Result<Vec<Job>, Box<dyn Error>> {
    let client = build_client()?;

    let current_user = get_current_user(&client, &args.domain).await.ok();

    // Fetch jobs with the 10 most-recent builds and their causes.
    let url = format!(
        "{}/api/json?tree=jobs[name,builds[number,timestamp,actions[causes[userId,shortDescription]]]{{,10}}]",
        args.domain
    );
    let response = client.get(&url).send().await?;
    let job_details: JobDetailList = serde_json::from_str(&response.text().await?)?;

    let mut jobs: Vec<Job> = job_details
        .jobs
        .into_iter()
        .map(|jd| {
            let last_used_by_me = current_user.as_deref().and_then(|me| {
                jd.builds.iter().find_map(|b| {
                    let triggered_by_me = b.actions.iter().any(|a| {
                        a.causes
                            .as_deref()
                            .unwrap_or_default()
                            .iter()
                            .any(|c| c.user_id.as_deref() == Some(me))
                    });
                    triggered_by_me.then_some(b.timestamp)
                })
            });
            Job {
                name: jd.name,
                last_used_by_me,
            }
        })
        .collect();

    // Most recently used by current user first; never-used go last.
    jobs.sort_by_key(|a| std::cmp::Reverse(a.last_used_by_me));

    Ok(jobs)
}

/// Lists the 20 most recent builds for a job, including who triggered each one.
pub async fn list_recent_builds(
    args: &Args,
    builder: &str,
) -> Result<Vec<BuildInfo>, Box<dyn Error>> {
    let client = build_client()?;
    let url = format!(
        "{}/job/{}/api/json?tree=builds[number,actions[causes[userId,shortDescription]]]{{,20}}",
        args.domain, builder
    );
    let response = client.get(&url).send().await?;

    #[derive(Deserialize)]
    struct JobBuilds {
        builds: Vec<RecentBuild>,
    }

    let job: JobBuilds = serde_json::from_str(&response.text().await?)?;
    let builds = job
        .builds
        .into_iter()
        .map(|b| {
            let triggered_by = cause_label(&b.actions);
            BuildInfo {
                number: b.number,
                triggered_by,
            }
        })
        .collect();
    Ok(builds)
}

pub async fn get_last_build(args: &Args, builder: &str) -> Result<Build, Box<dyn Error>> {
    let client = build_client()?;

    #[derive(Deserialize)]
    struct BuildList {
        builds: Vec<Build>,
    }

    let response = client
        .get(format!("{}/job/{}/api/json", args.domain, builder))
        .send()
        .await?;
    let mut json: BuildList = serde_json::from_str(&response.text().await?)?;
    json.builds.sort_by_key(|a| std::cmp::Reverse(a.number));
    let Some(n) = json.builds.first() else {
        return Err("no builds".into());
    };
    Ok(n.clone())
}
