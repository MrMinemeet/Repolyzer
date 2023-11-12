use chrono::{DateTime as DT, Datelike as DL, Local};
use git2::Repository;
use piechart::{Chart, Color, Data};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{path::PathBuf, process::exit};
use url::Url;

// ------------------------- Constants
const HELP: &str = "Usage: repolyzer [OPTIONS] <PATH>

Analyze a Git repository and display statistics about it.

OPTIONS:
    -c, --commit-graph        Enable the commit graph (similar to GitHub's)
    -e, --extended-overview  *Enables the extended overview instead of the general one
    -n, --no-overview         Disable the general overview
    -p, --pie-chart           Enable the pie chart
    -w, --week-day-stats     *Enable the week day stats

Options marked with a '*' may take more time and resources to compute, depending on the size of the repository.

PATH:
    The path to the Git repository to analyze. This can be a local path or a remote URL.
    If a remote URL is provided, the repository will be cloned to a temporary directory.";
const UNKNOWN_AUTHOR: &str = ">UNKNOWN<";
const SECONDS_PER_YEAR: u64 = 31_536_000;
const SECONDS_PER_DAY: u64 = 86_400;
const CHECKERBOARD_SYMBOL_AMOUNT: usize = 5;
// None, low, more, even more, a lot
const SYMBOLS: [char; CHECKERBOARD_SYMBOL_AMOUNT] = ['~', '·', '▪', '●', '⬟'];
// -------------------------

/// Holds the location for a given local or remote git repository
enum GitLocation {
    Local(PathBuf),
    Remote(Url),
}

/// Holds parsed app arguments
struct AppArgs {
    location: GitLocation,

    // Flags
    general_overview: bool,
    extended_overview: bool,
    pie_chart: bool,
    commit_graph: bool,
    weekday_stats: bool,
}

struct RepositoryStats {
    // General stats
    commit_count: usize,
    last_commit: u64,
    contributors: HashMap<String, u64>,

    // Extended stats
    total_files_changes: usize,
    total_lines_inserted: usize,
    total_lines_removed: usize,

    // Checkerboard stats
    commits_last_year: usize,
    longest_commit_streak: usize,
    current_commit_streak: usize,
    max_commits_a_day: usize,
    commits_per_day_last_year: [usize; 365],

    // Weekday stats
    commits_per_weekday: [usize; 7],
}

fn main() {
    println!("Hello, world!");

    let app_args: AppArgs = parse_args();
    let repository: Repository = load_repository(&app_args.location);

    let stats = gather_stats(repository, &app_args);

    if app_args.general_overview && !app_args.extended_overview {
        print_general_overview(&stats)
    }

    if app_args.extended_overview {
        print_extended_overview(&stats);
    }

    if app_args.pie_chart {
        print_pie_chart(&stats);
    }

    if app_args.commit_graph {
        print_commit_checker_board(&stats);
    }

    if app_args.weekday_stats {
        print_weekday_stats(&stats);
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
        repo.unwrap()
    } else if let GitLocation::Remote(url) = location {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("repolyzer");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards!")
            .as_nanos()
            .to_string();
        temp_dir.push(timestamp);

        let repo = Repository::clone(url.as_str(), temp_dir);
        if repo.is_err() {
            println!("Failed to clone and open repository!");
            exit(2);
        }
        repo.unwrap()
    } else {
        println!("Unknown Git Location!");
        exit(3);
    }
}

/// Parses the program arguments in order to get the location and other flags.
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
        extended_overview: false,
        pie_chart: false,
        commit_graph: false,
        weekday_stats: false,
    };

    // ----------------- Parse flags
    for arg in &args {
        if arg.starts_with('-') {
            match arg.as_str() {
                "-c" | "--commit-graph" => app_args.commit_graph = true,
                "-e" | "--extended-overview" => app_args.extended_overview = true,
                "-n" | "--no-overview" => app_args.general_overview = false,
                "-p" | "--pie-chart" => app_args.pie_chart = true,
                "-w" | "--week-day-stats" => app_args.weekday_stats = true,
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

    app_args
}

fn gather_stats(repository: Repository, app_args: &AppArgs) -> RepositoryStats {
    let mut diff_options = git2::DiffOptions::new();
    diff_options.include_unmodified(false);
    diff_options.include_untracked(false);
    diff_options.ignore_submodules(true);
    diff_options.ignore_blank_lines(true);

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let mut stats = RepositoryStats {
        commit_count: 0,
        last_commit: 0,
        contributors: HashMap::new(),

        total_files_changes: 0,
        total_lines_inserted: 0,
        total_lines_removed: 0,

        commits_last_year: 0,
        longest_commit_streak: 0,
        current_commit_streak: 0,
        max_commits_a_day: 0,
        commits_per_day_last_year: [0; 365],

        commits_per_weekday: [0; 7],
    };

    let mut prev_commit_time: u64 = 0;
    let mut current_streak: usize = 0;

    let mut revwalk = repository.revwalk().expect("Failed to get 'revwalk'");
    revwalk.push_head().expect("Failed to push HEAD!");

    // Loop over all commit_ids with the help of revwalk
    for commit_id in revwalk {
        let commit_id = commit_id.expect("Failed to get commit ID");
        let commit = repository
            .find_commit(commit_id)
            .expect("Could not find commit");

        // A commit was found
        stats.commit_count += 1;

        // Add contributor to hashmap and update commit amount
        {
            let author = commit.author();
            let author = author.name();
            let author = if let Some(author) = author {
                author
            } else {
                UNKNOWN_AUTHOR
            };

            stats.contributors.entry(author.to_string()).or_insert(0);
            stats.contributors.insert(
                author.to_string(),
                stats.contributors.get(&author.to_string()).unwrap() + 1,
            );
        }

        let commit_time = commit.time().seconds() as u64;
        if stats.last_commit < commit_time {
            stats.last_commit = commit_time;
        }

        // Collect stats for extended overview
        if app_args.extended_overview {
            // TODO: Optimize or use multithreading for this
            let parent = commit.parent(0);
            if parent.is_err() {
                // This is the first commit, so there is no parent
                continue;
            }
            let diff = repository
                .diff_tree_to_tree(
                    Some(&parent.unwrap().tree().unwrap()),
                    //Some(&p_tree.as_ref().unwrap()),
                    Some(&commit.tree().unwrap()),
                    None,
                )
                .expect("Failed to get diff");
            let diff_stats = diff.stats().expect("Failed to get stats");

            stats.total_files_changes += diff.deltas().count();
            stats.total_lines_inserted += diff_stats.insertions();
            stats.total_lines_removed += diff_stats.deletions();
        }

        if app_args.commit_graph {
            // Gather commits per day
            if commit_time > current_time - SECONDS_PER_YEAR {
                // Commit was made in the last year
                let day_of_year = (commit_time / SECONDS_PER_DAY) % 365;
                stats.commits_per_day_last_year[day_of_year as usize] += 1;
            }

            // Check if the current commit was made within the last 24 hours of the previous commit
            if prev_commit_time != 0 && commit_time > prev_commit_time - SECONDS_PER_DAY {
                current_streak += 1;
            } else {
                if current_streak > stats.longest_commit_streak {
                    stats.current_commit_streak = current_streak;
                }
                current_streak = 0;
                prev_commit_time = commit_time;
            }
        }

        if app_args.weekday_stats {
            // Gather commits per weekday
            let weekday = DT::from_timestamp(commit_time as i64, 0).unwrap().weekday();
            stats.commits_per_weekday[weekday.num_days_from_monday() as usize] += 1;
        }
    }

    if app_args.commit_graph {
        // Calculate max commits a day
        stats.max_commits_a_day = *stats.commits_per_day_last_year.iter().max().unwrap();

        // Calculate commits in the last year
        for i in 0..365 {
            stats.commits_last_year += stats.commits_per_day_last_year[i];
        }

        // Calculate longest streak
        current_streak = 0;
        for i in 0..365 {
            if stats.commits_per_day_last_year[i] > 0 {
                current_streak += 1;
            } else {
                if current_streak > stats.longest_commit_streak {
                    stats.longest_commit_streak = current_streak;
                }
                current_streak = 0;
            }
        }
    }

    // Clean up data
    temp_dir_cleanup(repository, &app_args.location);

    stats
}

fn print_general_overview(stats: &RepositoryStats) {
    let dt = DT::from_timestamp(stats.last_commit as i64, 0).unwrap();

    println!("-------------------------------------");
    println!("Overall commit stats:");
    println!("Commit amount ......... {}", stats.commit_count);
    println!("Last commit ........... {}", dt.format("%d-%m-%Y %H:%M:%S"));
    println!("Contributor amount .... {}", stats.contributors.len());
    println!("-------------------------------------");
}

fn print_extended_overview(stats: &RepositoryStats) {
    let dt = DT::from_timestamp(stats.last_commit as i64, 0).unwrap();
    println!("-------------------------------------");
    println!("Overall commit stats:");
    println!("Commit amount ......... {}", stats.commit_count);
    println!("Last commit ........... {}", dt.format("%d-%m-%Y %H:%M:%S"));
    println!("Contributor amount .... {}", stats.contributors.len());
    println!("Files changed ......... {}", stats.total_files_changes);
    println!("Lines inserted......... {}", stats.total_lines_inserted);
    println!("Lines removed ......... {}", stats.total_lines_removed);
    println!(
        "Total lines (delta) ... {}",
        stats.total_lines_inserted - stats.total_lines_removed
    );
    println!(
        "Add./Del. ratio........ {:.2}",
        stats.total_lines_inserted as f64 / stats.total_lines_removed as f64
    );
    println!("-------------------------------------");
}

fn print_pie_chart(stats: &RepositoryStats) {
    const NAMED_COMMITS_IN_CHART: usize = 5;
    const SYMBOLS: [char; 6] = ['•', '▪', '▴', '◆', '⬟', '◆'];
    println!("Commit pie chart:");

    let colors = [
        Color::RGB(255, 99, 132),  // Red
        Color::RGB(54, 162, 235),  // Blue
        Color::RGB(255, 206, 86),  // Yellow
        Color::RGB(75, 192, 192),  // Teal
        Color::RGB(153, 102, 255), // Purple
        Color::RGB(255, 159, 64),  // Orange
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

fn print_commit_checker_board(stats: &RepositoryStats) {
    let distribution = calculate_symbol_distribution(stats);

    println!("╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("║\tCommits in the last year: {} | Longest Streak: {} days | Current Streak: {} days | Max a day: {}"
        , stats.commits_last_year, stats.longest_commit_streak, stats.current_commit_streak, stats.max_commits_a_day);
    println!("╠═══════════════════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("║      Jan      Feb      Mar      Apr      May      Jun      Jul      Aug      Sep      Oct      Nov     Dec");
    println!(
        "║ Mon\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Mon, &distribution)
    );
    println!(
        "║ Tue\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Tue, &distribution)
    );
    println!(
        "║ Wed\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Wed, &distribution)
    );
    println!(
        "║ Thu\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Thu, &distribution)
    );
    println!(
        "║ Fri\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Fri, &distribution)
    );
    println!(
        "║ Sat\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Sat, &distribution)
    );
    println!(
        "║ Sun\t{}",
        calculate_day_commit_graph(stats, chrono::Weekday::Sun, &distribution)
    );
    println!("╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════");
}

fn print_weekday_stats(stats: &RepositoryStats) {
    // Limit to 20 bars per weekday
    let max_commits = stats.commits_per_weekday.iter().max().unwrap();

    println!("-------------------------------------");
    println!("Commits per weekday:");
    for i in 0..7 {
        let percentage =
            (stats.commits_per_weekday[i] as f64 / *max_commits as f64 * 20.0) as usize;
        let weekday = match i {
            0 => "Mon",
            1 => "Tue",
            2 => "Wed",
            3 => "Thu",
            4 => "Fri",
            5 => "Sat",
            6 => "Sun",
            _ => "???", // Should/Can never happen
        };
        println!(
            "\t{}\t{}\t|{}",
            weekday,
            stats.commits_per_weekday[i],
            "█".repeat(percentage)
        );
    }
}

/// Calculates the distribution borders for the commit checker board
fn calculate_symbol_distribution(stats: &RepositoryStats) -> [usize; CHECKERBOARD_SYMBOL_AMOUNT] {
    // Get the max commits a day
    let mut max_commits_a_day = 0;
    for commits in stats.commits_per_day_last_year.iter() {
        if *commits > max_commits_a_day {
            max_commits_a_day = *commits;
        }
    }

    // Calculate the symbol distribution borders
    let range_size = max_commits_a_day / CHECKERBOARD_SYMBOL_AMOUNT;
    let low = range_size;
    let more = range_size * 2;
    let even_more = range_size * 3;
    let a_lot = range_size * 4;

    let distribution = [0, low, more, even_more, a_lot];

    // Print distribution
    println!("-------------------------------------");
    print!("Distribution: ");
    print!("{} = {} | ", SYMBOLS[0], distribution[0]);
    for i in 1..distribution.len() - 1 {
        print!("{} for <= {}, ", SYMBOLS[i], distribution[i]);
    }
    println!(
        "{} for > {}",
        SYMBOLS[distribution.len() - 1],
        distribution[distribution.len() - 1]
    );
    println!();

    distribution
}

fn calculate_day_commit_graph(
    stats: &RepositoryStats,
    weekday: chrono::Weekday,
    symbol_dist: &[usize; CHECKERBOARD_SYMBOL_AMOUNT],
) -> String {
    let today = Local::now();

    let mut num_of_weekdays = 0;
    for i in 0..365 {
        let day = today - chrono::Duration::days(i);
        if day.weekday() == weekday {
            num_of_weekdays += 1;
        }
    }

    let mut graph_line = String::new();
    for i in 0..num_of_weekdays {
        let day_index = 7 * i + weekday.num_days_from_monday() as usize;
        if day_index >= 365 {
            break;
        }

        let commits_on_day = stats.commits_per_day_last_year[day_index];

        // Get symbol for this day
        let mut symbol = ' ';
        for j in 0..symbol_dist.len() {
            if commits_on_day <= symbol_dist[j] {
                symbol = SYMBOLS[j];
                break;
            }
        }
        if symbol == ' ' {
            // If no symbol was found, use the last one (as it then is > symbol_dist[CHECKERBOARD_SYMBOL_AMOUNT - 1])
            symbol = SYMBOLS[SYMBOLS.len() - 1];
        }

        graph_line.push(' ');
        graph_line.push(symbol);
    }
    graph_line
}

/// Cleans up the temporary directory if the repository was cloned
fn temp_dir_cleanup(repository: Repository, location: &GitLocation) {
    if let GitLocation::Remote(_) = location {
        let path = repository.path().parent().unwrap();
        std::fs::remove_dir_all(path).expect("Failed to remove temporary directory!");
    }
}