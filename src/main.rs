use anyhow::Result;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use octocrab::models::workflows::Run;
use opentelemetry::trace::Tracer;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;

/// `gh-action-trace` is used to create traces for GitHub Action runs
/// by talking to the GitHub API and getting the metadata. This is
/// intended to be run as a standalone binary.
#[derive(Parser)]
#[clap(
    version = "0.2.0",
    author = "Tarun Pothulapati <tarunpothulapati@outlook.com>"
)]
struct Opts {
    /// Organization or owner name of the GitHub repository
    #[clap(short, long)]
    owner: String,
    /// Name of the GitHub repository
    #[clap(short, long)]
    repo: String,
    /// Token to interact with the GitHub API
    /// Will fallback to interacting without an API, which might
    /// cause timeouts
    #[clap(short, long)]
    token: Option<String>,
    /// Number of runs to retrieve per workflow. Defaults to 30
    #[clap(long, default_value = "30")]
    runs: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse arguments
    let mut opts: Opts = Opts::parse();
    if opts.token.is_none() {
        match std::env::var("GITHUB_ACCESS_TOKEN") {
            Ok(token) => {
                opts.token = Some(token);
            }
            Err(_) => {
                println!("No token provided, falling back to no-auth");
            }
        }
    }
    let m = MultiProgress::new();
    let spinner_style = ProgressStyle::default_bar()
        .progress_chars("=> ")
        .template("{wide_msg} {pos}/{len} [{bar:40}] ({eta})");

    // Initialize octocrab instance
    let mut instance = octocrab::Octocrab::builder().build()?;
    if let Some(token) = opts.token {
        instance = octocrab::OctocrabBuilder::new()
            .personal_token(token)
            .build()?;
    }

    // Install a new OpenTelemetry trace pipeline
    //let tracer = stdout::new_pipeline().install_simple();
    let trace_provider = opentelemetry_jaeger::new_pipeline()
        .with_service_name(format!("{}/{}", opts.owner, opts.repo))
        .build_simple()?;

    let tracer = trace_provider.tracer("gh-action-trace", Some(env!("CARGO_PKG_VERSION")));

    // List workflows
    let workflows = instance
        .workflows(opts.owner.clone(), opts.repo.clone())
        .list()
        .send()
        .await?
        .into_iter();

    for (i, workflow) in workflows.clone().enumerate() {
        // Create a new progress bar for each workflow
        let pb = m.add(
            ProgressBar::new(128)
                .with_style(spinner_style.clone())
                .with_prefix(format!("[{}/{}]", i + 1, workflows.len())),
        );

        tokio::spawn(process_workflow(
            instance.clone(),
            tracer.clone(),
            pb,
            opts.runs,
            workflow.clone(),
            opts.owner.clone(),
            opts.repo.clone(),
        ));
    }

    m.join()?;
    return Ok(());
}

async fn process_workflow(
    instance: octocrab::Octocrab,
    tracer: opentelemetry::sdk::trace::Tracer,
    pb: ProgressBar,
    runs: u32,
    workflow: octocrab::models::workflows::WorkFlow,
    owner: String,
    repo: String,
) -> Result<()> {
    let runs = retrieve_runs(
        runs,
        &instance,
        &workflow.id.to_string(),
        owner.as_str(),
        repo.as_str(),
    )
    .await
    .unwrap();

    pb.set_message(format!("Processing workflow/{}", workflow.name,));
    pb.set_length(runs.len() as u64);

    // List Jobs for each workflow
    for run in runs {
        let job_result = instance
            .workflows(owner.clone(), repo.clone())
            .list_jobs(run.id)
            // set max
            .per_page(100)
            .send()
            .await;

        if let Err(_) = job_result {
            println!("Err retrieving jobs for {} workflow run", run.id);
            continue;
        }

        let mut last_end_time = run.created_at;

        // Send a Trace for this Run
        for job in job_result.unwrap() {
            // Send a span for each job
            let mut builder = tracer
                .span_builder(job.name.clone())
                .with_span_id(opentelemetry::trace::SpanId::from_hex(
                    job.id.to_string().as_str(),
                ))
                .with_trace_id(opentelemetry::trace::TraceId::from_hex(
                    run.id.to_string().as_str(),
                ))
                .with_start_time(job.started_at.unwrap())
                .with_attributes(value_to_vec(&serde_json::to_value(&job).unwrap()))
                .with_status_message(job.status.to_string());
            // Attach end time only if its not None
            if let Some(completed_at) = job.completed_at {
                builder = builder.with_end_time(completed_at);
            }

            tracer.build(builder);

            // Update last_end_time
            if let Some(completed_at) = job.completed_at {
                if completed_at > last_end_time {
                    last_end_time = completed_at;
                }
            }
            // TODO: Send a span for each step?
        }

        let builder = tracer
            .span_builder(run.name.clone())
            .with_span_id(opentelemetry::trace::SpanId::from_hex(
                run.id.to_string().as_str(),
            ))
            .with_trace_id(opentelemetry::trace::TraceId::from_hex(
                run.id.to_string().as_str(),
            ))
            .with_start_time(run.created_at)
            .with_end_time(last_end_time)
            .with_attributes(value_to_vec(&serde_json::to_value(&run).unwrap()));

        tracer.build(builder);
        pb.inc(1);
    }
    pb.finish_with_message(format!("Completed workflow {}", workflow.name));
    Ok(())
}

async fn retrieve_runs(
    mut n: u32,
    instance: &octocrab::Octocrab,
    workflow: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<Run>> {
    let mut runs = Vec::new();
    let mut page: u32 = 1;
    loop {
        // Break when all are retrieved
        if n == 0 {
            break;
        }

        let mut per_page = 100;
        if n <= per_page {
            per_page = n;
        }

        let mut runs_page = instance
            .workflows(owner, repo)
            .list_runs(workflow)
            .page(page)
            // per_page is always <= 100
            .per_page(per_page as u8)
            .send()
            .await?;

        if runs_page.items.is_empty() {
            // Completed all?
            break;
        }

        if runs_page.items.len() > 0 {
            n = n - (runs_page.items.len() as u32);
        }
        page = page + 1;
        runs.append(&mut runs_page.items);
    }
    Ok(runs)
}

// value_to_vec converts a serde Value into a Vec of KeyValue
// that can be passed in as SpanAttributes
fn value_to_vec(value: &serde_json::Value) -> Vec<KeyValue> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| KeyValue {
            key: k.to_string().into(),
            value: v.to_string().into(),
        })
        .collect()
}
