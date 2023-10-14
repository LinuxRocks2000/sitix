/*
    The goal here is to be a proprietary, professional-ish Jekyll alternative.
    Written in Rust for speed.

    Goalpost 1: templating - make arbitrary HTML pages that can be templated
    Goalpost 2: render MarkDown or something like it. Probably something very custom.
    Goalpost 3: nested configuration (where priority increases with directory depth and based on a naming scheme); ability to access that configuration
    Goalpost 4: interpreted language of some sort built-in that allows very advanced code.
    Goalpost 5: complex element packs (for things like galleries) that can be inline-templated and nested and whatnot.

    "Sitix" (pronounced "Site-eyeX") - fancy names are fun
*/

use clap::Parser;

pub mod rasta;


#[derive(Parser)]
struct SitixArgs {
    directory : Option<std::path::PathBuf>,
    output_directory : Option<std::path::PathBuf>
}


fn main() {
    println!("** Sitix v{} by Tyler Clarke **", env!("CARGO_PKG_VERSION"));
    let mut args = SitixArgs::parse();
    if args.directory.is_none() {
        args.directory = Some(std::path::PathBuf::from("."));
    }
    if args.output_directory.is_none() {
        args.output_directory = Some(args.directory.as_ref().unwrap().clone());
        args.output_directory.as_mut().unwrap().push("output");
    }
    let mut templates_dir = args.directory.unwrap().clone();
    templates_dir.push("_templates");
    println!("Checking project validity.");
    if templates_dir.exists() {
        println!(" Templates directory exists; project is valid.");
    }
    else {
        println!(" Templates directory does not exist. Exiting.");
        return;
    }
    println!("Cleaning output directory.");
    println!(" Removing old output directory.");
    if args.output_directory.as_ref().unwrap().exists() {
        std::fs::remove_dir_all(args.output_directory.as_ref().unwrap()).expect("Failed to clean output directory.");
        println!("  Successfully removed old output directory.");
    }
    else {
        println!("  Nothing to do! Selected output dir does not exist.");
    }
    println!(" Creating empty output directory.");
    std::fs::create_dir_all(args.output_directory.as_ref().unwrap()).expect("Failed to create output directory.");
    println!("  Successfully created new empty output direcory.");
    println!("Output clean successful.");
    println!("Creating template list");
    //let mut templates : Vec <rasta::RastaTemplate> = Vec::new();
    for path in std::fs::read_dir(templates_dir).unwrap() {
        let mut file = match std::fs::File::open(path.unwrap().path()) {
            Ok(f) => f,
            Err(_) => {continue;}
        };
        //templates.push(rasta::RastaPage::load(&mut file).unwrap());
        let tokens = rasta::lexer(&mut file);
        let mut tokens = tokens.iter().peekable();
        let r = rasta::TreeNode::congeal(&mut tokens);
        println!("{}", r.render(rasta::Scope::top().wrap()));
    }
}
