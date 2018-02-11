extern crate clap;
extern crate csv;
extern crate git2;
#[macro_use]
extern crate serde_derive;

use clap::{App, Arg};
use git2::Repository;
use std::{io, process};
use std::error::Error;

#[derive(Debug, Serialize)]
struct DiffStat {
    commit_id: String,
    insertions: usize,
    deletions: usize,
}

fn diff_stats(count: usize) -> Result<Vec<DiffStat>, git2::Error> {
    let repo = Repository::discover("./")?;

    let mut revwalker = repo.revwalk()?;
    revwalker.push_head()?;

    // We want to get `count` diffs, so we have to look at `count + 1` commits.
    //
    // To get one diff, you need two commits.
    let mut commits = revwalker
        .take(count + 1)
        .map(|rev| rev.ok())
        .collect::<Vec<_>>();

    // We may request more commits than the repository contains. In this case we add `None` to the
    // back of the list to later compare the last commit to an empty tree.
    if commits.len() <= count {
        commits.push(None);
    }

    let diff_stats = commits
        .as_slice()
        .windows(2)
        .map(|window| {
            let current_commit = repo.find_commit(window[0].unwrap())?;
            let current_tree = current_commit.tree()?;

            let diff = if let Some(oid) = window[1] {
                let previous_commit = repo.find_commit(oid)?;
                let previous_tree = previous_commit.tree()?;

                repo.diff_tree_to_tree(Some(&previous_tree), Some(&current_tree), None)?
            } else {
                repo.diff_tree_to_tree(None, Some(&current_tree), None)?
            };

            let stats = diff.stats()?;

            let id = format!("{}", current_commit.id());

            Ok(DiffStat {
                commit_id: id,
                insertions: stats.insertions(),
                deletions: stats.deletions(),
            })
        })
        .map(|stat: Result<DiffStat, git2::Error>| stat.unwrap())
        .collect::<Vec<_>>();

    Ok(diff_stats)
}

fn run(count: usize) -> Result<(), Box<Error>> {
    if let Ok(stats) = diff_stats(count) {
        let mut writer = csv::Writer::from_writer(io::stdout());

        for stat in stats {
            writer.serialize(stat)?;
        }

        writer.flush()?;
    }

    Ok(())
}

fn main() {
    let matches = App::new("git-diff-stat")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Christoph Rüßler <christoph.ruessler@mailbox.org>")
        .about("exports git diff stats (insertions, deletions) as CSV")
        .after_help("git-diff-stat searches for a git repository the same way git does.")
        .arg(
            Arg::with_name("count")
                .short("n")
                .long("count")
                .help("number of commits to show diff stats for, defaults to 10")
                .takes_value(true),
        )
        .get_matches();

    let count = matches
        .value_of("count")
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(10);

    if let Err(err) = run(count) {
        println!("{}", err);
        process::exit(1);
    }
}
