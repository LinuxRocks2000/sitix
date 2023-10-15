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
use std::io::Write;

pub mod rasta;


#[derive(Parser)]
struct SitixArgs {
    directory : Option<std::path::PathBuf>,
    output_directory : Option<std::path::PathBuf>
}


fn parse_all_recursive(templates : &Vec<(String, rasta::TreeNode)>, rpath : std::path::PathBuf) {
    for path in std::fs::read_dir(rpath).unwrap() {
        let path_propre = path.as_ref().unwrap().path();
        let dirname = path_propre.file_name().unwrap();
        if dirname == "_templates" || dirname == "output" {
            continue;
        }
        let meta = std::fs::metadata(&path_propre).unwrap();
        if meta.is_dir() {
            parse_all_recursive(templates, path_propre);
        }
        else if meta.is_file() {
            println!(" Rendering {}", (&path_propre).display());
            let r = rasta::TreeNode::parse(path_propre.clone());
            let text = match r {
                Ok(r) => {
                    let string = if r.is_plaintext() {
                        r.plaintext()
                    }
                    else {
                        let sacrifice = rasta::Scope::top().wrap();
                        let content = rasta::Scope::chitlin_w(sacrifice.clone(), "content".to_string());
                        r.render(content); // toss the render result, we only want to fill the scope. clean this up later.
                        let template_name = match sacrifice.borrow().get("content.template".to_string()) {
                            Some(value) => value,
                            None => "default".to_string()
                        };
                        println!("  Parsing with template {}", template_name);
                        let mut template : Option<usize> = None;
                        for (index, pair) in templates.iter().enumerate() {
                            if pair.0 == template_name {
                                template = Some(index);
                            }
                        }
                        if template.is_none() {
                            println!("   Invalid template");
                            continue;
                        }
                        let template = template.unwrap();
                        templates[template].1.render(sacrifice)
                    };
                    string.into_bytes()
                },
                Err(_) => {
                    std::fs::read(&path_propre).unwrap()
                }
            };
            //sacrifice.borrow().draw_tree(0);
            let mut path = std::path::PathBuf::from("output");
            path.push(path_propre);
            println!("creating {:?}", path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut file = std::fs::File::create(path).unwrap();
            
            file.write_all(&text).unwrap();
        }
        else {
            println!("WARNING: Your filesystem tree looks kinda ill. Consider, perhaps, cleaning it up?");
        }
    }
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
    let mut templates : Vec<(String, rasta::TreeNode)> = vec![];
    for path in std::fs::read_dir(templates_dir).unwrap() {
        /*let mut file = match std::fs::File::open(path.as_ref().unwrap().path()) {
            Ok(f) => f,
            Err(_) => {continue;}
        };
        //templates.push(rasta::RastaPage::load(&mut file).unwrap());
        let tokens = rasta::lexer(&mut file);
        let mut tokens = tokens.iter().peekable();
        let r = rasta::TreeNode::congeal(&mut tokens);
        //println!("{}", r.render(rasta::Scope::top().wrap()));*/
        let r = rasta::TreeNode::parse(path.as_ref().unwrap().path()).unwrap(); // unwrap is safe here, because if your templates directory has invalid Rasta you have bigger problems.
        templates.push((path.unwrap().path().file_stem().unwrap().to_str().unwrap().to_string(), r));
    }
    println!("Rendering");
    parse_all_recursive(&templates, std::path::PathBuf::from("."));
}
