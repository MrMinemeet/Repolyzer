use std::collections::HashMap;
use std::{path::PathBuf, process::exit};
use std::time::{SystemTime, UNIX_EPOCH};
use git2::Repository;
use url::Url;
use chrono::DateTime;
use piechart::{Chart, Color, Data};


// ------------------------- Constants
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
const UNKNOWN_AUTHOR: &str = ">UNKNOWN<";
// ------------------------- 

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

struct RepositoryStats {
    commit_count: usize,
    last_commit: i64,
    contributors: HashMap<String, u64>
}

fn main() {
    println!("Hello, world!");

    let app_args: AppArgs = parse_args();
    let repository: Repository = load_repository(&app_args.location);

    let stats = gather_stats(repository);


    if app_args.general_overview {
        print_general_overview(&stats)
    }

    if app_args.pie_chart {
        print_pie_chart(&stats);
    }
}

/// Downloads or load the repository depending on the type of location
fn load_repository(location: &GitLocation) -> Repository {
    if let GitLocation::Local(path) = location {
        let repo = Repository::open(path);
        if repo.is_err() {
            println!("Could not open the local repository!");
            exit(2);
        }
        return repo.unwrap();
    } else if let GitLocation::Remote(url) = location {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("repolyzer");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards!")
            .as_nanos().to_string();
        temp_dir.push(timestamp);

        let repo = Repository::clone(url.as_str(), temp_dir);
        if repo.is_err() {
            println!("Failed to clone and open repository!");
            exit(2);
        }
        return repo.unwrap();
    } else {
        println!("Unknown Git Location!");
        exit(3);
    }
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

fn gather_stats(repository: Repository) -> RepositoryStats {
    let mut stats = RepositoryStats {
        commit_count: 0,
        last_commit: 0,
        contributors: HashMap::new(),
    };

    let mut revwalk = repository.revwalk()
        .expect("Failed to get 'revwalk'");
    revwalk.push_head()
        .expect("Failed to push HEAD!");

    // Loop over all commit_ids with the help of revwalker
    for commit_id in revwalk {
        let commit_id = commit_id
            .expect("Failed to get commit ID");
        let commit = repository.find_commit(commit_id)
            .expect("Could not find commit");

    
        // A commit was found
        stats.commit_count += 1;

        // Add contributor to hashmap and update commit amount
        {
            let author = commit.author();
            let author =  author.name();
            let author = if author.is_some() { author.unwrap() } else { UNKNOWN_AUTHOR };

            if stats.contributors.contains_key(&author.to_string()) {
                stats.contributors.insert(author.to_string(), stats.contributors.get(&author.to_string()).unwrap() + 1);
            } else {
                stats.contributors.insert(author.to_string(), 1);
            }
        }

        let commit_time = commit.time().seconds();
        if stats.last_commit < commit_time {
            stats.last_commit = commit_time;
        }

    }


    return stats;
}

fn print_general_overview(stats: &RepositoryStats) {
    let dt = DateTime::from_timestamp(stats.last_commit, 0).unwrap();

    println!("-------------------------------------");
    println!("Overall commit stats:");
    println!("Commit amount ......... {}", stats.commit_count);
    println!("Last commit ........... {}" , dt.format("%d-%m-%Y %H:%M:%S"));
    println!("Contributor amount .... {}", stats.contributors.len());
    println!("-------------------------------------");
}

fn print_pie_chart(stats: &RepositoryStats) {
    const NAMED_COMMITS_IN_CHART: usize = 5;
    const SYMBOLS: [char; 6] = [ '•', '▪', '▴', '◆', '⬟', '◆' ];
    println!("Commit pie chart:");

    let colors = [
        Color::RGB(255, 99, 132),   // Red
        Color::RGB(54, 162, 235),   // Blue
        Color::RGB(255, 206, 86),   // Yellow
        Color::RGB(75, 192, 192),   // Teal
        Color::RGB(153, 102, 255),  // Purple
        Color::RGB(255, 159, 64),   // Orange
    ];

    // Sort descending by commit amount
    let mut top_contributors: Vec<(&String, &u64)> = stats.contributors.iter().collect();
    top_contributors.sort_by(|a, b| b.1.cmp(a.1));
    top_contributors.truncate(NAMED_COMMITS_IN_CHART);

    // Add "Others" if there are more than NAMED_COMMITS_IN_CHART contributors
    let mut others = 0;
    for (_, commits) in stats.contributors.iter().skip(NAMED_COMMITS_IN_CHART) {
        others += commits;
    }
    let others_contributors = (&"Others".to_string(), &others);
    if others > 0 {
        top_contributors.push(others_contributors);
    }

    // Create data vector
    let mut top_data: Vec<Data> = Vec::new();
    for i in 0..top_contributors.len() {
        let (name, commits) = top_contributors[i];
        let data = Data {
            label: name.to_string(),
            value: *commits as f32,
            color: Some(colors[i].into()),
            fill: SYMBOLS[i % SYMBOLS.len()],
        };
        top_data.push(data);
    }

    // Create chart
    Chart::new()
        .radius(9)
        .aspect_ratio(3)
        .legend(true)
        .draw(&top_data);
    



}