use std::{error::Error, time::Duration};

use args::Args;
use clap::Parser;
use dialoguer::Select;
use pipeline::Build;

mod args;
mod build;
mod pipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.min_query > args.max_query {
        return Err("minimum query should be less than max query".into());
    }

    // Resolve the builder and an optional fixed build number.
    let (builder, discovered_number) = match &args.builder {
        Some(b) => (b.clone(), None),
        None => {
            if args.number.is_some() {
                return Err("--number requires --builder to be specified as well".into());
            }
            let b = discover_builder(&args).await?;
            let n = discover_build(&args, &b).await?;
            (b, Some(n))
        }
    };

    // Explicit --number wins; otherwise use the discovered one (if any).
    let effective_number = args.number.or(discovered_number);
    // Follow (loop for new builds) only when no specific build was chosen.
    let follow = effective_number.is_none();

    let mut last_build = Option::<u32>::None;
    loop {
        let build = match effective_number {
            Some(number) => Ok(Build::new(number)),
            None => pipeline::get_last_build(&args, &builder).await,
        };

        let Ok(build) = build else {
            tokio::time::sleep(Duration::from_millis(args.max_query)).await;
            continue;
        };

        if let Some(last) = last_build
            && last == build.get_number()
        {
            tokio::time::sleep(Duration::from_millis(args.max_query)).await;
            continue;
        }

        last_build = Some(build.get_number());

        if !build::stream_build(&args, &builder, build.get_number(), follow).await? {
            break;
        }
    }
    Ok(())
}

async fn discover_builder(args: &Args) -> Result<String, Box<dyn Error>> {
    let jobs = pipeline::list_jobs(args).await?;
    if jobs.is_empty() {
        return Err("No Jenkins jobs found on this domain".into());
    }

    let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
    let selection = Select::new()
        .with_prompt("Select a builder")
        .items(&names)
        .default(0)
        .interact()?;

    Ok(jobs[selection].name.clone())
}

async fn discover_build(args: &Args, builder: &str) -> Result<u32, Box<dyn Error>> {
    let builds = pipeline::list_recent_builds(args, builder).await?;
    if builds.is_empty() {
        return Err(format!("No builds found for job '{builder}'").into());
    }

    let labels: Vec<String> = builds
        .iter()
        .map(|b| format!("#{} — {}", b.number, b.triggered_by))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a build")
        .items(&labels)
        .default(0)
        .interact()?;

    Ok(builds[selection].number)
}
