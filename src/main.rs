use std::{path::PathBuf, process::exit};
use url::Url;

const HELP: &str = "Usage: repolyzer [OPTIONS] <PATH>

Analyze a Git repository and display statistics about it.

OPTIONS:
    -c, --commit-graph     Enable the commit graph (similar to GitHub's)
    -n, --no-overview      Disable the general overview
    -p, --pie-chart        Enable the pie chart
    -w, --week-day-stats   Enable the week day stats (May take a while to compute)

PATH:
    The path to the Git repository to analyze. This can be a local path or a remote URL.
    If a remote URL is provided, the repository will be cloned to a temporary directory.";

/// Holds the location for a given local or remote git repository
enum GitLocation {
    Local(PathBuf),
    Remote(Url)
}

/// Holds parsed app argumnets
struct AppArgs {
    location: GitLocation,

    // Flags
    general_overview: bool,
    pie_chart: bool,
    commit_graph: bool,
    weekday_stats: bool,
}


fn main() {
    println!("Hello, world!");

    let app_args = parse_args();
}

/// Parses the programm arguments in order to get the location and other flags.
fn parse_args() -> AppArgs {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        // No argument was passed (args[0] is the tool itself)
        println!("{}", HELP);
        exit(2);
    }

    // Create initial AppArgs struct
    let mut app_args = AppArgs {
        location: GitLocation::Local(PathBuf::from("")),

        general_overview: true,
        pie_chart: false,
        commit_graph: false,
        weekday_stats: false
    };
    
    // ----------------- Parse flags
    for arg in &args {
        if arg.starts_with('-') {
            match arg.as_str() {
                "-n" | "--no-overview" => app_args.general_overview = false,
                "-p" | "--pie-chart" => app_args.pie_chart = true,
                "-w" | "--week-day-stats" => app_args.weekday_stats = true,
                "-c" | "--commit-graph" => app_args.commit_graph = true,
                _ => {
                    println!("Unknown argument: {}", arg);
                    println!("{}", HELP);
                    exit(2);
                }
            }
        }
    }

    // ----------------- Retrieve path from args

    // Filter out any argument that is not the first one and does not start with a '-'
    let repository_path = args.iter().skip(1).find(|&arg| !arg.starts_with('-'));
    if let Some(repository_path) = repository_path {
        if repository_path.starts_with("http") {
            // Remote HTTP(s) URL
            let url: Url = Url::parse(repository_path).expect("Could not detect valid URL");
            app_args.location = GitLocation::Remote(url);
        } else if repository_path.starts_with("git@") {
            // Remote SSH URL
            println!("The provided path seems to be using SSH, which is not supported yet!");
            exit(2);
        } else {
            // Assume a local path then
            let local_path: PathBuf = PathBuf::from(repository_path);
            if !local_path.exists() || !local_path.is_dir() {
                println!("The provided path either does not exist, or is not a directory!");
                exit(2);
            }
            app_args.location = GitLocation::Local(local_path);
        }
    } else {
        println!("No path provided!");
        exit(2);
    }
    
    return app_args;
}