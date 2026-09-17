use crate::args::Args;
use crate::pipeline::build_client;
use reqwest::Response;
use std::{error::Error, str::FromStr, time::Duration};

fn backoff(mut dur: Duration, text: &str, args: &Args) -> Duration {
    if text.is_empty() {
        dur *= 2;
        std::cmp::min(dur, Duration::from_millis(args.max_query))
    } else {
        Duration::from_millis(args.min_query)
    }
}

fn get_header<T: FromStr>(response: &Response, name: &str) -> Result<T, String> {
    Ok(
        match match response
            .headers()
            .get(name)
            .ok_or_else(|| format!("Missing {name} header"))?
            .to_str()
        {
            Ok(it) => it,
            Err(_) => return Err("Cannot convert to string".into()),
        }
        .parse::<T>()
        {
            Ok(it) => it,
            Err(_) => return Err("Cannot convert to type".into()),
        },
    )
}

pub async fn stream_build(
    args: &Args,
    builder: &str,
    build: u32,
    follow: bool,
) -> Result<bool, Box<dyn Error>> {
    println!("Streaming build {build}");
    let client = build_client()?;
    let mut start = 0;
    let mut wait_time = backoff(Duration::from_millis(0), "start", args);
    println!("{}/job/{}/{}", args.domain, builder, build);
    tokio::time::sleep(Duration::from_secs(1)).await;
    loop {
        let url = format!(
            "{}/job/{}/{}/logText/progressiveText?start={}",
            args.domain, builder, build, start
        );

        let response = client.get(&url).send().await?;

        match get_header::<i32>(&response, "x-text-size") {
            Ok(s) => start = s,
            Err(_) if args.wait => {
                println!("Waiting");
                tokio::time::sleep(Duration::from_millis(args.max_query)).await;
                continue;
            }
            Err(error) => {
                return Err(error.into());
            }
        }

        let more =
            get_header::<bool>(&response, "x-more-data").or(Result::<bool, String>::Ok(false))?;

        let text = response.text().await?;
        print!("{text}");

        if !more {
            println!("Done");
            println!("{}/job/{}/{}", args.domain, builder, build);
            return Ok(follow);
        }

        wait_time = backoff(wait_time, &text, args);
        tokio::time::sleep(wait_time).await;
    }
}
