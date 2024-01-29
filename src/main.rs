use clap::Parser;
use git2::{ObjectType, Repository, Sort, Commit};
use std::{fmt::Debug, fs, path::Path};
use std::collections::HashMap;
use chrono::{DateTime, NaiveDateTime, Utc};
use minijinja::value::Value;
use minijinja::{context, path_loader, Environment};
use minijinja::{Error, ErrorKind};
use once_cell::sync::Lazy;

use std::{
    io::{self, Write},
    process,
};

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    #[arg(short, long)]
    workpath: Option<std::path::PathBuf>,
}

// jinja function
fn include_file(name: String) -> Result<String, Error> {
    std::fs::read_to_string(&name)
        .map_err(|e| Error::new(
            ErrorKind::InvalidOperation,
            "cannot load file"
        ).with_source(e))
}

fn print_tree_entries<'a>(
    tree: &'a git2::Tree<'a>,
    repo: &'a Repository,
    path: String,
    only_dirs: bool,
) -> Vec<String> {
    let mut files = Vec::new();
    for entry in tree.iter() {
        match entry.kind() {
            Some(ObjectType::Blob) => {
                if !only_dirs {
                    let mut fname = format!("{}/{}", path, entry.name().unwrap());
                    if fname.starts_with("/") {
                        fname = fname.strip_prefix("/").unwrap().to_string();
                    }
                    files.push(fname);
                }
            }
            Some(ObjectType::Tree) => {
                let new_tree = entry.to_object(&repo).unwrap().into_tree().unwrap();
                let dir_name = format!("{}/{}", path, entry.name().unwrap());
                if only_dirs {
                    files.push(dir_name.strip_prefix("/").unwrap().to_string());
                }
                let sub_files = print_tree_entries(&new_tree, &repo, dir_name, only_dirs);
                files.extend(sub_files);
            }
            _ => {}
        }
    }

    files
}

fn summarize_file_types(files: Vec<String>) -> HashMap<String, usize> {
    let mut summary = HashMap::new();
    for file in &files {
        // println!("ext: {}", file);
        let ext = file.split(".").last().unwrap().to_string();
        let count = summary.entry(ext).or_insert(0);
        *count += 1;
    }
    summary
}

fn count_files_in_dirs<'a>(
    dirs: Vec<String>,
    repo: &'a Repository,
) -> HashMap<String, (usize, String)> {
    let mut dir_file_counts = HashMap::new();

    let head = repo.head().unwrap();
    let head_tree = head.peel_to_tree().unwrap();

    for dir in dirs {
        // ignore root
        if dir.starts_with('.') {
            continue;
        }
        let tree_entry = head_tree.get_path(&Path::new(&dir)).unwrap();
        let tree = tree_entry.to_object(&repo).unwrap().into_tree().unwrap();

        let files = print_tree_entries(&tree, &repo, dir.clone(), false);

        let summary_file_types = {
            // summarize file types based off file extension
            let mut summary = HashMap::new();
            for file in &files {
                let ext = file.split(".").last().unwrap();
                let count = summary.entry(ext).or_insert(0);
                *count += 1;
            }
            summary
        };
        // println!("files: {} {:?}", dir, summaryFileTypes);
        let mut s: String = String::new();
        for (ext, count) in summary_file_types {
            s.push_str(&format!("{}: {} ", ext, count));
        }
        dir_file_counts.insert(dir, (files.len(), s));
    }

    dir_file_counts
}

// debug function
fn print_commit_info(repo: &Repository, commit_id: git2::Oid) {
    let commit = repo.find_commit(commit_id).unwrap();

    println!("Commit: {}", commit.id());
    println!("Tree: {}", commit.tree_id());
    println!("Parent count: {}", commit.parent_count());
    println!("Message: {}", commit.message().unwrap_or("No message"));
    println!("Author: {}", commit.author());
    println!("Committer: {}", commit.committer());
    // println!("Time: {}", commit.time());
}

// // Count all commits from main/master branch
// fn count_commits(repo: &Repository) -> usize {
//     let mut count = 0;
//     let mut revwalk = repo.revwalk().unwrap();
    
//     // lookup main/master branch ref; don't assume git workdir branch
//     let mref = repo.resolve_reference_from_short_name("master")
//         .or_else(|_| repo.resolve_reference_from_short_name("main"))
//         .expect("no master or main branch");
//     let commit = mref.peel_to_commit().unwrap();
//     revwalk.push(commit.id()).unwrap();

//     for id in revwalk {
//         let id = id.unwrap();
//         let object = repo.find_object(id, None).unwrap();
//         if object.kind() == Some(git2::ObjectType::Commit) {
//             count += 1;
//         }
//     }

//     count
// }


fn count_commits(repo: &Repository) -> usize {
    let mut count = 0;
    let commit_count = |commit: &Commit| {
        count += 1;
    };
    walk_commits(&repo, commit_count);
    count
}

fn count_commits_by_year(repo: &Repository) -> HashMap<i64, i32> {
    let mut count = HashMap::new();

    
    let commit_count = |commit: &Commit| {
        let timestamp = commit.time().seconds();
        let approx_year = 1970 + timestamp / 31_556_952;
        count.insert(approx_year, count.get(&approx_year).unwrap_or(&0) + 1);
    };
    walk_commits(&repo, commit_count);
    count
}

// Convenience function to walk all commits from main/master branch
fn walk_commits<F>(repo: &Repository, mut action: F)
where
    F: FnMut(&Commit),
{
    let mut revwalk = repo.revwalk().unwrap();
    
    // lookup main/master branch ref; don't assume git workdir branch
    let mref = repo.resolve_reference_from_short_name("origin/master")
        .or_else(|_| repo.resolve_reference_from_short_name("origin/main"))
        .or_else(|_| repo.resolve_reference_from_short_name("origin/dev"))
        .expect("no master or main branch");
    let commit = mref.peel_to_commit().unwrap();
    revwalk.push(commit.id()).unwrap();

    for id in revwalk {
        let id = id.unwrap();
        let object = repo.find_object(id, None).unwrap();
        if object.kind() == Some(git2::ObjectType::Commit) {
            let commit = object.as_commit().unwrap();
            action(&commit);
        }
    }
}

fn get_tags_ordered_by_date(repo: &Repository) -> HashMap<String, i64> {
    // tags order defaults from oldest to newest
    let mut tags = HashMap::new();

    // repo.tag_foreach(|id, name| {
    //     // trim "refs/tags/" from tag name
    //     let name = std::str::from_utf8(&name[10..]).unwrap().to_owned();
    //     let commit = repo.find_commit(id);
    //     if commit.is_err() {
    //         println!("err: {} {:?}", name, commit);
    //         return true;
    //     } else {
    //         let commit = commit.unwrap();
    //          // Create a NaiveDateTime from the timestamp
    //         let naive = NaiveDateTime::from_timestamp_opt(commit.time().seconds(), 0).unwrap();

    //         // Create a normal DateTime from the NaiveDateTime
    //         // let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

    //         // Format the datetime how you want
    //         let newdate = naive.format("%Y-%m-%d %H:%M:%S").to_string();

    //         tags.insert(String::from(name), newdate);
    //     }

    //     true
    // }).unwrap();

    let m = repo.tag_names(Some("*")).expect("no tags found");
    for name in &m {
        let name = name.unwrap();
        let obj = repo.revparse_single(name).unwrap();
        let commit = match obj.kind() {
            Some(git2::ObjectType::Tag) => {
            // Some(tag) = obj.as_tag() {
                let tag = obj.as_tag().unwrap();
                // handle lightweight tags; need to lookup commit from tag
                let commit = repo.find_commit(tag.target_id()).unwrap();
                // println!("commit {} {}", name, commit.id());
                commit.to_owned()
            }
            Some(git2::ObjectType::Commit) => {
                let commit = obj.as_commit().unwrap();
            // Some(commit) = obj.as_commit() {
                // println!("commit {} {}", name, commit.id());
                commit.to_owned()
            }
            _ => continue,
        };

        // let naive = NaiveDateTime::from_timestamp_opt(commit.time().seconds(), 0).unwrap();
        // let newdate = naive.format("%Y-%m-%d %H:%M:%S").to_string();
        tags.insert(name.to_string(), commit.time().seconds());
    }

    // let tags = repo.tag_names(None).unwrap();
    // for tag in tags.iter() {
    //     let tag = tag.unwrap();
    //     // let tag = tag_ref.peel_to_tag().unwrap();
    //     // let target = tag.target().unwrap();
    //     // repo.find_commit(target.id()).unwrap()
    // }

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_glob("refs/tags/*").unwrap();
    revwalk.set_sorting(Sort::TIME).unwrap();

    for id in revwalk {
        let id = id.unwrap();
        // println!("id: {:?}", id);
        let object = repo.find_object(id, None).unwrap();

        // print_commit_info(repo, id);
        // let tag = repo.find_tag(id).expect("tag not found");
        // let time = match object.kind() {
        //     Some(git2::ObjectType::Commit) => {
        //         let commit = object.as_commit().unwrap();
        //         let secs = commit.time().seconds();
        //         let datetime = NaiveDateTime::from_timestamp_opt(secs, 0);
        //         // datetime.format("%Y-%m-%d").to_string();
        //         ()
        //     }
        //     Some(git2::ObjectType::Tag) => {
        //         let target = tag.target().unwrap();
        //         let commit = target.as_commit().unwrap();
        //         commit.time().seconds()
        //     }
        // _ => continue,
        // };

        // let timestamp = time.seconds();
        // let naive_datetime = NaiveDateTime::from_timestamp(timestamp, 0);
        // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
        // let date_string = datetime.format("%Y-%m-%d").to_string();
        // tags.insert(tag.name().unwrap().to_string(), String::from("date"));
    }

    tags
}

// Filter to convert a string into a link, returns HTML
fn urlize(value: String) -> Value {
    let v = format!("<a href=\"https:{}\">{}</a>", &value, &value);
    Value::from_safe_string(v)
}

// Filter to format unix timestamp into a date string
fn format_date(value: i64) -> Value {
    let naive = NaiveDateTime::from_timestamp_opt(value, 0).unwrap();
    // let newdate = naive.format("%Y-%m-%d %H:%M:%S").to_string();
    Value::from_safe_string(naive.format("%Y-%m-%d %H:%M:%S").to_string())
    // let naive_datetime = NaiveDateTime::from_timestamp(value, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    // let date_string = datetime.format("%Y-%m-%d").to_string();
    // Value::from_safe_string(date_string)
}

// minijinja lazy templates
static ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env.add_filter("urlize", urlize);
    env.add_filter("format_date", format_date);
    env.add_function("include_file", include_file);
    env
});

// load from embed
fn env_load() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_filter("urlize", urlize);
    env.add_filter("format_date", format_date);
    // env.add_function("include_file", include_file);
    minijinja_embed::load_templates!(&mut env, "main");
    env
}

fn compile_css() {
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=src/templates/input.css");

    match process::Command::new("sh")
        .arg("-c")
        .arg("npx tailwindcss -i src/templates/input.css -o srctemplates/output.css")
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let _ = io::stdout().write_all(&output.stdout);
                let _ = io::stdout().write_all(&output.stderr);
                panic!("Tailwind error");
            }
        }
        Err(e) => panic!("Tailwind error: {:?}", e),
    };
}

fn main() {
    // minijinja_embed::embed_templates!("templates");
    let args = Cli::parse();

    compile_css();
    
    let mut cwd = std::env::current_dir().unwrap();
    if args.workpath.is_some() {
        cwd = args.workpath.unwrap();
    }
    println!("The current directory is {}", cwd.display());
    // println!("hello summary");

    // attempt to read git directory
    let repo = match Repository::open(cwd) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    println!("repo: {:?}", repo.path());

    let head = repo.head().unwrap();
    let tree = head.peel(ObjectType::Tree).unwrap().into_tree().unwrap();

    let dirs = print_tree_entries(&tree, &repo, String::new(), true);
    let counts = count_files_in_dirs(dirs.clone(), &repo);

    let _strings = vec!["A", "alfa", "1"];
    let origin = repo.find_remote("origin").unwrap();
    let repo_url = origin.url().unwrap();
    // assume git@github.com/user/repo format
    let repo_link = repo_url
        .split_terminator("@")
        .last()
        .unwrap()
        .replace(":", "/");

    let tags = get_tags_ordered_by_date(&repo);
    let all_files = print_tree_entries(&tree, &repo, String::new(), false);
    let summarize_file_types = summarize_file_types(all_files.clone());
    let commit_count = count_commits(&repo);
    let commit_year_counts = count_commits_by_year(&repo);

    // let env = &ENV;
    let env = env_load();
    let tmpl = env.get_template("index.html").unwrap();
    let ctx = context!(
        name => "World",
        all_files => all_files,
        commit_count => commit_count,
        commit_year_counts => commit_year_counts,
        repo_link => repo_link,
        counts => counts,
        tags => tags,
        file_types => summarize_file_types,
    );

    // println!("context: {:?}", ctx);
    // Write to a file
    let filepath = "output.html";
    fs::write(filepath, tmpl.render(ctx).unwrap()).expect("Unable to write file");
}
