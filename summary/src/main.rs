use clap::Parser;
use git2::{ObjectType, Repository};
use std::{fs, fmt};

use askama::Template; // bring trait in scope

#[derive(Template)] // this will generate the code...
#[template(path = "index.html")] // using the template in this path, relative


// to the `templates` dir in the crate root
struct HelloTemplate<'a> {
    // the name of the struct can be anything
    name: &'a str, // the field name should match the variable name
                   // in your template
    repo_link: String,
    files: Vec<&'a str>,
}

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    #[arg(short, long)]
    workpath: Option<std::path::PathBuf>,
}

fn print_tree_entries<'a>(tree: &'a git2::Tree<'a>, repo: &'a Repository, path: String) -> Vec<&'a str> {
    let mut files = Vec::new();
    for entry in tree.iter() {
        match entry.kind() {
            Some(ObjectType::Blob) => {
                let fname = format!("{}/{}", path, entry.name().unwrap());
                let fname = fname.strip_prefix("/").unwrap();
                files.push(fname);
            }
            Some(ObjectType::Tree) => {
                let new_tree = entry.to_object(&repo).unwrap().into_tree().unwrap();
                let sub_files = print_tree_entries(
                    &new_tree,
                    &repo,
                    format!("{}/{}", path, entry.name().unwrap()),
                );
                files.extend(sub_files);
            }
            _ => {}
        }
    }

    files
}

fn main() {
    let args = Cli::parse();

    // Use globbing

    

    let mut cwd = std::env::current_dir().unwrap();
    if args.workpath.is_some() {
        cwd = args.workpath.unwrap();
    }
    println!("The current directory is {}", cwd.display());
    println!("hello summary");

    // attempt to read git directory
    let repo = match Repository::open(cwd) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    println!("repo: {:?}", repo.path());

    let head = repo.head().unwrap();
    let tree = head.peel(ObjectType::Tree).unwrap().into_tree().unwrap();

    let files = print_tree_entries(&tree, &repo, String::new());

    let strings = vec!["A", "alfa", "1"];
    let origin = repo.find_remote("origin").unwrap();
    let repo_url = origin.url().unwrap();
    // assume git@github.com/user/repo format
    let repo_link = repo_url
        .split_terminator("@")
        .last()
        .unwrap()
        .replace(":", "/");

    let hello = HelloTemplate { 
        name: "world",
        repo_link: repo_link,
        files: files,
    }; // instantiate your struct
    // println!("{}", hello.render().unwrap()); // then render it.
    // Using the tera Context struct
    // context.insert("product", &"Product 1");
    // Write to a file
    let filepath = "output.html";
    fs::write(filepath, hello.render().unwrap()).expect("Unable to write file");
}
