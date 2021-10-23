use anyhow::Result;
use clap::Parser;
use opentelemetry::trace::Tracer;
use opentelemetry::KeyValue;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

/// `gh-action-trace` is used to create traces for GitHub Action runs
/// by talking to the GitHub API and getting the metadata. This is
/// intended to be run as a standalone binary.
#[derive(Parser)]
#[clap(
    version = "0.1.0",
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse arguments
    let opts: Opts = Opts::parse();

    // Initialize octocrab instance
    let mut instance = octocrab::Octocrab::builder().build()?;
    if let Some(token) = opts.token {
        instance = octocrab::OctocrabBuilder::new()
            .personal_token(token)
            .build()?;
    }

    // Install a new OpenTelemetry trace pipeline
    //let tracer = stdout::new_pipeline().install_simple();
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(format!("{}/{}", opts.owner, opts.repo))
        .install_simple()?;

    // Create a tracing subscriber with the configured tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer.clone());

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    let _collector = Registry::default().with(telemetry).try_init()?;

    // List workflows
    let workflows = instance
        .workflows(opts.owner.clone(), opts.repo.clone())
        .list()
        .send()
        .await?;
    for workflow in &workflows {
        // TODO: Process more runs
        let runs = instance
            .workflows(opts.owner.clone(), opts.repo.clone())
            .list_runs(workflow.id.to_string())
            .exclude_pull_requests(true)
            .send()
            .await?;
        println!(
            "Processing {} runs out of {} for workflow {}",
            runs.items.len(),
            runs.total_count.unwrap_or(0),
            workflow.name,
        );

        // List Jobs for each workflow
        for run in runs {
            let jobs = instance
                .workflows(opts.owner.clone(), opts.repo.clone())
                .list_jobs(run.id)
                .send()
                .await?;

            let mut last_end_time = run.created_at;

            // Send a Trace for this Run
            for job in jobs {
                // Send a span for each job
                let builder = tracer
                    .span_builder(job.name.clone())
                    .with_span_id(opentelemetry::trace::SpanId::from_hex(
                        job.id.to_string().as_str(),
                    ))
                    .with_trace_id(opentelemetry::trace::TraceId::from_hex(
                        run.id.to_string().as_str(),
                    ))
                    .with_start_time(job.started_at)
                    .with_end_time(job.completed_at.unwrap())
                    .with_attributes(vec![
                        KeyValue {
                            key: "job.id".into(),
                            value: job.id.to_string().into(),
                        },
                        KeyValue {
                            key: "job.name".into(),
                            value: job.name.clone().into(),
                        },
                        KeyValue {
                            key: "job.head_sha".into(),
                            value: job.head_sha.into(),
                        },
                        KeyValue {
                            key: "job.run_id".into(),
                            value: job.run_id.to_string().into(),
                        },
                        KeyValue {
                            key: "job.status".into(),
                            value: job.status.clone().into(),
                        },
                        KeyValue {
                            key: "job.conclusion".into(),
                            value: job.conclusion.unwrap_or_default().into(),
                        },
                        KeyValue {
                            key: "job.started_at".into(),
                            value: job.started_at.to_string().into(),
                        },
                        KeyValue {
                            key: "job.completed_at".into(),
                            value: job.completed_at.unwrap().to_string().into(),
                        },
                        KeyValue {
                            key: "job.url".into(),
                            value: job.url.to_string().into(),
                        },
                        KeyValue {
                            key: "job.html_url".into(),
                            value: job.html_url.to_string().into(),
                        },
                        KeyValue {
                            key: "job.run_url".into(),
                            value: job.run_url.to_string().into(),
                        },
                        KeyValue {
                            key: "job.check_run_url".into(),
                            value: job.check_run_url.to_string().into(),
                        },
                        KeyValue {
                            key: "job.steps".into(),
                            value: job
                                .steps
                                .clone()
                                .into_iter()
                                .map(|s| s.name)
                                .collect::<Vec<_>>()
                                .join(",")
                                .into(),
                        },
                    ])
                    .with_status_message(job.status.to_string());

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
                .span_builder(run.name)
                .with_span_id(opentelemetry::trace::SpanId::from_hex(
                    run.id.to_string().as_str(),
                ))
                .with_trace_id(opentelemetry::trace::TraceId::from_hex(
                    run.id.to_string().as_str(),
                ))
                .with_start_time(run.created_at)
                .with_end_time(last_end_time);

            tracer.build(builder);
        }
    }

    return Ok(());
}
