use clap::{load_yaml, App};
use colored::Colorize;
use dialoguer::{Confirm, Input, MultiSelect};
use lazy_static::lazy_static;
use native_dialog::FileDialog;
use reqwest::{
    blocking::{multipart, Client},
    StatusCode,
};
use rustyline::{error::ReadlineError, Editor};
use serde::Deserialize;
use serde_yaml::Value;
use std::{
    env,
    fmt::Display,
    process::exit,
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard, TryLockError},
    time::Duration,
};

lazy_static! {
    static ref USER_AGENT: String = format!(
        "gamify-rust / {} / {}",
        env!("CARGO_PKG_VERSION"),
        env::consts::OS
    );
}

#[non_exhaustive]
struct ENDPOINT;
impl ENDPOINT {
    const BASE_LINK: &'static str = "http://localhost:8080/GamifyUser/";
    const CAMPAIGN_IMAGES: &'static str = "uploads/campaignImages/";
    const LOGIN: &'static str = "CheckLogin";
    const ADMIN_LIST: &'static str = "admin/listQuestionnaires";
    const ADMIN_CREATE: &'static str = "admin/create";
}

#[derive(Deserialize)]
struct Questionnaire {
    questionnaireId: i32,
    datetime: String,
    image: String,
    name: String,
}

impl Display for Questionnaire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date0: Vec<&str> = self.datetime.split(",").collect();
        let mut namel: usize = self.name.len();
        if namel > 30 as usize {
            namel = 30;
        }
        let mut qIdl: usize = self.questionnaireId.to_string().len();
        if qIdl > 6 as usize {
            qIdl = 6;
        }
        let mut datel: Vec<&str> = Vec::with_capacity(28);
        datel.push(date0[0]);
        datel.push(date0[1]);

        write!(
            f,
            "│ {: ^5} │ {: ^30} │ {: >16} │",
            &self.questionnaireId.to_string()[0..qIdl].blue(),
            &self.name[0..namel].bright_blue().bold(),
            format!{"{:?}{:?}", datel[0], datel[1]},
        )
    }
}

fn main() {
    println!(
        "{} {}",
        "Gamify Admin CLI v.".bold().bright_blue(),
        env!("CARGO_PKG_VERSION").bright_blue()
    );
    println!(
        "{}",
        "https://github.com/darklamp/gamify-rust".italic().white()
    );

    let mut rl = Arc::new(Mutex::new(rustyline::Editor::<()>::new()));
    let yaml = load_yaml!("../cli.yaml");
    let matches = App::from(yaml).get_matches();
    let f = std::fs::File::open("config.yaml").expect("Config file not found");
    let config: Value = serde_yaml::from_reader(f).expect("Config not readable");
    let mut is_logged_in: bool = false;

    let client = reqwest::blocking::Client::builder()
        .user_agent(&*USER_AGENT)
        .cookie_store(true)
        .build()
        .expect("Error in client creation");

    if login(
        &client,
        config["username"].as_str().unwrap().to_string(),
        config["password"].as_str().unwrap().to_string(),
    ) {
        println!("{}", "Login OK".bold().green());
        is_logged_in = true;
    } else {
        println!("{}", "Login KO".bold().red());
        clean_exit();
    }

    /*   if let Some(ref matches) = matches.subcommand_matches("admin") {
        if matches.is_present("create") {
            println!(
                "{}",
                &*USER_AGENT.italic().white()
            );
        } else if matches.is_present("list") {
            println!("{}", "asd".italic().white())
        } else {
            println!("{}", "Unimplemented :(".bright_blue());
            return;
        }
    }*/

    let commands_tree: Arc<RwLock<Vec<&str>>> = Arc::new(RwLock::new(vec![]));
    let help = |st| match st {
        "na" => println!("{}", "Available commands: admin, user.".yellow()),
        "admin" => print!(
            "{}",
            "Available commands: create, list, delete, back.".yellow()
        ),
        _ => print!("{}", "Available commands: admin, user.".yellow()),
    };
    let get_formatted_readline_prompt = |cm: &RwLockWriteGuard<Vec<&str>>| {
        let mut out = ">> ".bright_blue().to_string();
        for c in (**cm).as_slice() {
            out = format!("{}{}", c, out);
        }
        out
    };
    let read = |promptstr| {
        let x = rl.try_lock();
        match x {
            Ok(mut ed) => {
                let cmtree = commands_tree.write();
                match cmtree {
                    Ok(mut cm) => {
                        if promptstr != "" {
                            cm.push(promptstr)
                        };
                        ed.readline(&*get_formatted_readline_prompt(&cm))
                    }
                    _ => Err(ReadlineError::Utf8Error),
                }
            }
            Err(e) => Err(ReadlineError::Utf8Error),
        }
    };

    let admin = || {
        let mut readline = read("admin ");
        loop {
            match readline {
                Ok(line) => {
                    match &*line {
                        "b" | "back" => {
                            let ct = commands_tree.write();
                            match ct {
                                Ok(mut c) => {
                                    (*c).pop();
                                }
                                _ => {}
                            }
                            break;
                        }

                        "create" => {
                            let name: String = Input::new()
                                .with_prompt("Questionnaire name")
                                .interact_text()
                                .unwrap();
                            let date: String = Input::new()
                                .with_prompt("Date (YYYY-MM-DD)")
                                .interact_text()
                                .unwrap();

                            // TODO add a config option that switches headless on / off

                            /* HEADLESS let image: String = Input::new()
                            .with_prompt("Image [ex. /home/ale/Desktop/img.jpeg]")
                            .interact_text()
                            .unwrap();*/

                            println!("Loaading image picker.. ");
                            let image = FileDialog::new()
                                .set_location("~/Desktop")
                                .add_filter("Image", &["png", "jpg", "jpeg", "heic"])
                                .show_open_single_file()
                                .unwrap()
                                .unwrap()
                                .into_os_string()
                                .into_string()
                                .unwrap();

                            let mut questions: Vec<String> = Vec::new();
                            let mut question: String;
                            let mut question_count: u8 = 0;
                            loop {
                                question = Input::new()
                                    .with_prompt(format!(
                                        "{}{}",
                                        "Question #".blue(),
                                        question_count.to_string().blue()
                                    ))
                                    .interact_text()
                                    .unwrap();
                                if !question.is_empty() {
                                    questions.push(question);
                                    if !Confirm::new().with_prompt("Continue?").interact().unwrap()
                                    {
                                        break;
                                    } else {
                                        question_count += 1;
                                    }
                                }
                            }
                            if create_questionnaire(&client, name, date, image, questions) {
                                println!(
                                    "{}",
                                    "Questionnaire submitted successfully!".bright_green()
                                );
                                break;
                            } else {
                                println!("{}", "Questionnaire submission failed!".bright_red());
                            }
                        }

                        "list" => {
                            let start: String = Input::new()
                                .with_prompt("Start from [default: 0]")
                                .default("0".into())
                                .interact_text()
                                .unwrap();
                            let size: String = Input::new()
                                .with_prompt("Size (10,25,50,100")
                                .default("100".into())
                                .interact_text()
                                .unwrap();
                            if !list(&client, start, size, false) {
                                println!("{}", "Error retrieving list".red());
                            }
                        }

                        "Ctrl-C" | "Ctrl-D" => clean_exit(),

                        _ => help("admin"),
                    };
                    print!("\n");
                }
                Err(ReadlineError::Interrupted) => {
                    clean_exit();
                }
                _ => {
                    println!(
                        "{}",
                        "Error in command. Please enter correct command or press CTRL+C to exit"
                            .bold()
                            .red()
                    )
                }
            }
            readline = read("");
        }
    };

    loop {
        let readline = read("");
        match readline {
            Ok(line) => {
                match &*line {
                    "CTRL+C" | "CTRL+D" | "SHIFT+B" => {
                        clean_exit();
                    }

                    "admin" => admin(),

                    "user" => {}

                    _ => help("na"),
                };
            }
            Err(ReadlineError::Interrupted) => {
                clean_exit();
            }
            _ => {
                println!(
                    "{}",
                    "Error in command. Please enter correct command or press CTRL+C to exit"
                        .bold()
                        .red()
                )
            }
        }
    }
}

fn login(client: &Client, username: String, password: String) -> bool {
    let params = [("username", username), ("pwd", password)];
    let res = client
        .post(&format!("{}{}", ENDPOINT::BASE_LINK, ENDPOINT::LOGIN))
        .form(&params)
        .timeout(Duration::from_secs(10))
        .send();
    match res.unwrap().status() {
        StatusCode::OK => return true,
        _ => return false,
    };
}

fn list(client: &Client, start: String, size: String, past: bool) -> bool {
    let params = [("start", start), ("size", size), ("past", past.to_string())];
    let res = client
        .get(&format!("{}{}", ENDPOINT::BASE_LINK, ENDPOINT::ADMIN_LIST))
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send();
    let res1 = res.unwrap();
    match res1.status() {
        StatusCode::OK => {
            let result: Vec<Questionnaire> = res1.json().unwrap();
            println!("┌─ ID ──┬───────────── Name ─────────────┬────── Date ──────┐");
            for r in result {
                println!("{}", r);
            }
            println!("└─ ID ──┴───────────── Name ─────────────┴────── Date ──────┘");
            return true;
        }
        _ => return false,
    };
}

fn create_questionnaire(
    client: &Client,
    name: String,
    date: String,
    image: String,
    questions: Vec<String>,
) -> bool {
    let mut form = multipart::Form::new()
        .text("name", name)
        .text("date", date)
        .file("image", image)
        .unwrap();

    let mut counter: usize = 0;

    //TODO Find better way of doing this
    const qnames: [&str; 6] = [
        "Question0",
        "Question1",
        "Question2",
        "Question3",
        "Question4",
        "Question5",
    ];

    for q in questions {
        form = form.text(qnames[counter], q);
        counter += 1;
    }
    let res = client
        .post(&format!(
            "{}{}",
            ENDPOINT::BASE_LINK,
            ENDPOINT::ADMIN_CREATE
        ))
        .multipart(form)
        .send();

    match res.unwrap().status() {
        StatusCode::OK => return true,
        /* StatusCode::UNAUTHORIZED => {
            //TODO
            return false;
        }*/
        _ => return false,
    };
}

fn clean_exit() {
    println!("{}", "bye <3".bright_blue().italic());
    exit(0);
}
