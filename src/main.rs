use clap::{load_yaml, App};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
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
    vec,
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
    const ADMIN_DELETE: &'static str = "admin/delete";
    const ADMIN_INSPECT: &'static str = "admin/listQuestionnaireCompletedUsers";
    const ADMIN_INSPECT_CANCELED: &'static str = "admin/listQuestionnaireCanceledUsers";
    const ADMIN_ANSWERS_RETRIEVAL: &'static str = "admin/getAnswers";
}

#[derive(Deserialize)]
struct Questionnaire {
    questionnaireId: i32,
    datetime: String,
    image: String,
    name: String,
}

#[derive(Deserialize)]
struct AnswerList {
    stats: Vec<Option<String>>,
    opt: Vec<OptionalAnswer>,
}

#[derive(Deserialize)]
struct OptionalAnswer {
    question: String,
    content: String,
}

impl std::fmt::Debug for OptionalAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Question: {0}, Answer: {1}\n",
            self.question, self.content
        )
    }
}
#[derive(Deserialize)]
struct User {
    userId: i32,
    birth: String,
    sex: String,
    username: String,
}

impl Display for AnswerList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut stats: Vec<String> = vec![];
        for s in &self.stats {
            let mut sp: String = "N/A".to_string();
            if s.is_some() {
                sp = s.as_deref().unwrap().to_string();
            }
            stats.push(sp);
        }
        let mut _x = write!(
            f,
            "{}",
            "\n ~~~~~~~ Statistical answers ~~~~~~\n\n"
                .bold()
                .bright_blue()
        );
        _x = write!(
            f,
            "Age: {}, Sex: {}, Experience: {}\n\n",
            stats[0], stats[1], stats[2]
        );
        _x = write!(
            f,
            "{}",
            " ~~~~~~~   Optional answers  ~~~~~~\n\n"
                .bold()
                .bright_blue()
        );
        write!(f, "{:?}", self.opt)
    }
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
            format! {"{:?}{:?}", datel[0], datel[1]},
        )
    }
}
#[derive(Debug, Deserialize)]
struct Config {
    username: String,
    password: String,
    debug: bool,
    history: bool,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date0: Vec<&str> = self.birth.split(",").collect();
        let mut datel: Vec<&str> = Vec::with_capacity(28);
        datel.push(date0[0]);
        datel.push(date0[1]);

        write!(
            f,
            "│ {: ^5} │ {: ^30} │ {: >16} │ {: ^7} │",
            &self.userId.to_string(),
            &self.username,
            format! {"{:?}{:?}", datel[0], datel[1]},
            &self.sex
        )
    }
}

fn main() {
    let logo = r#"     
     ___           ___           ___                       ___               
    /  /\         /  /\         /__/\        ___          /  /\        ___   
   /  /:/_       /  /::\       |  |::\      /  /\        /  /:/_      /__/|  
  /  /:/ /\     /  /:/\:\      |  |:|:\    /  /:/       /  /:/ /\    |  |:|  
 /  /:/_/::\   /  /:/~/::\   __|__|:|\:\  /__/::\      /  /:/ /:/    |  |:|  
/__/:/__\/\:\ /__/:/ /:/\:\ /__/::::| \:\ \__\/\:\__  /__/:/ /:/   __|__|:|  
\  \:\ /~~/:/ \  \:\/:/__\/ \  \:\~~\__\/    \  \:\/\ \  \:\/:/   /__/::::\  
 \  \:\  /:/   \  \::/       \  \:\           \__\::/  \  \::/       ~\~~\:\ 
  \  \:\/:/     \  \:\        \  \:\          /__/:/    \  \:\         \  \:\
   \  \::/       \  \:\        \  \:\         \__\/      \  \:\         \__\/
    \__\/         \__\/         \__\/                     \__\/              "#;
    println!(
        "{} {}\n",
        logo.bold().on_black().bright_blue(),
        env!("CARGO_PKG_VERSION").bright_blue()
    );
    println!("{}\n", "https://github.com/darklamp/gamify-rust".magenta());

    println!(
        "{}\n",
        "Press CTRL+C to exit, write b or back to go back, anything to get help."
            .italic()
            .white()
    );

    let f = std::fs::File::open("config.yaml").expect("Config file not found");
    let config: Config = serde_yaml::from_reader(f).expect("Config not readable");

    let mut rl = rustyline::Editor::<()>::new();

    // try and load history file if history option is on
    if config.history {
        if rl.load_history(".gamify_history.txt").is_err() && config.debug {
            println!("No previous history.");
        }
    }

    //let yaml = load_yaml!("../cli.yaml");
    // let matches = App::from(yaml).get_matches();

    let client = reqwest::blocking::Client::builder()
        .user_agent(&*USER_AGENT)
        .cookie_store(true)
        .build()
        .expect("Error in client creation");

    let mut role: &str = "admin";
    let foo: String;

    match login(&client, &config.username, &config.password) {
        Ok(res) => {
            foo = res.clone();
            role = foo.strip_prefix("/GamifyUser/").unwrap();
            println!("{}", "Login OK".bold().green());
        }
        _ => {
            println!("{}", "Login KO".bold().red());
            clean_exit();
        }
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

    let help = |st| match st {
        "na" => println!("{}", "Available commands: admin, user.".yellow()),
        "admin" => print!(
            "{}",
            "Available commands: create, list, delete, inspect, back.".yellow()
        ),
        _ => print!("{}", "Available commands: admin, user.".yellow()),
    };

    let mut admin = || {
        let prompt = format!("{}{}", &(config.username).blue(), " >> ".blue());
        loop {
            let readline = rl.readline(&*prompt);
            match readline {
                Ok(line) => {
                    if config.history {
                        rl.add_history_entry(line.as_str());
                    }
                    let mut toks = line.split(' ').fuse();
                    match toks.next() {
                        Some("b") | Some("back") => {
                            clean_exit();
                        }

                        Some("create") => {
                            let name: String = Input::new()
                                .with_prompt("Questionnaire name")
                                .interact_text()
                                .unwrap();
                            let date: String = Input::new()
                                .with_prompt("Date (YYYY-MM-DD)")
                                .interact_text()
                                .unwrap();

                            let image_picker = FileDialog::new()
                                .set_location("~/Desktop")
                                .add_filter("Image", &["png", "jpg", "jpeg", "heic"])
                                .show_open_single_file();

                            let image: String;

                            // checks if image picker errors out, for example on an headless machine

                            if image_picker.is_err() {
                                image = Input::new()
                                    .with_prompt("Image [ex. /home/ale/Desktop/img.jpeg]")
                                    .interact_text()
                                    .unwrap();
                            } else {
                                println!("Loading image picker.. ");
                                image = image_picker
                                    .unwrap()
                                    .unwrap()
                                    .into_os_string()
                                    .into_string()
                                    .unwrap();
                            }

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
                            } else {
                                println!("{}", "Questionnaire submission failed!".bright_red());
                            }
                        }

                        Some("list") => {
                            let mut start: String = String::from("");
                            let mut size: String = String::from("");
                            let mut past: String = String::from("");

                            let mut initialized: [bool; 3] = [false, false, false];
                            loop {
                                match toks.next() {
                                    Some(a) => match a {
                                        "default" => {
                                            start = "0".to_string();
                                            size = "100".to_string();
                                            past = "n".to_string();
                                            initialized = [true, true, true];
                                            break;
                                        }
                                        "start" => {
                                            if initialized[0] {
                                                break;
                                            }
                                            let x = toks.next();
                                            if x.is_none() {
                                                break;
                                            } else {
                                                start = x.unwrap().to_string();
                                                initialized[0] = true;
                                            }
                                        }
                                        "size" => {
                                            if initialized[1] {
                                                break;
                                            }
                                            let x = toks.next();
                                            if x.is_none() {
                                                break;
                                            } else {
                                                size = x.unwrap().to_string();
                                                initialized[1] = true;
                                            }
                                        }
                                        "past" => {
                                            if initialized[2] {
                                                break;
                                            }
                                            let x = toks.next();
                                            if x.is_none() {
                                                break;
                                            } else {
                                                past = x.unwrap().to_string();
                                                initialized[2] = true;
                                            }
                                        }
                                        _ => break,
                                    },
                                    None => break,
                                }
                            }

                            if !initialized[0] {
                                start = Input::new()
                                    .with_prompt("Start from [default: 0]")
                                    .default("0".into())
                                    .interact_text()
                                    .unwrap();
                            }
                            if !initialized[1] {
                                size = Input::new()
                                    .with_prompt("Size (10,25,50,100)")
                                    .default("100".into())
                                    .interact_text()
                                    .unwrap();
                            }
                            if !initialized[2] {
                                past = Input::new()
                                    .with_prompt("Only past questionnaires? (y/n)")
                                    .default("n".into())
                                    .interact_text()
                                    .unwrap();
                            }
                            let p: bool = (&past).to_lowercase().contains(|c| c == 'y' || c == 't');

                            if !list(&client, &start, &size, p) {
                                println!("{}", "Error retrieving list".red());
                            }
                        }

                        Some("inspect") => {
                            let id: String = Input::new()
                                .with_prompt("Questionnaire ID [default: 0]")
                                .default("0".into())
                                .interact_text()
                                .unwrap();
                            let canceled: String = Input::new()
                                .with_prompt("Canceled users?")
                                .default("n".into())
                                .interact_text()
                                .unwrap();

                            let p: bool = canceled.to_lowercase().contains("y");

                            match inspect(&client, &id, p) {
                                Ok(uId) => {
                                    if uId == -1 {
                                        let word = match p {
                                            true => "canceled",
                                            _ => "answered",
                                        };
                                        println!(
                                            "{0} {1} {2}",
                                            "No one".blue(),
                                            word.blue(),
                                            "yet!".blue()
                                        );
                                    } else {
                                        showAnswers(&client, &id, uId);
                                    }
                                }
                                Err(e) => {}
                            };
                        }

                        Some("delete") => {
                            let id: String = Input::new()
                                .with_prompt("Questionnaire ID")
                                .interact_text()
                                .unwrap();

                            if !delete(&client, &id) {
                                println!("{}", "Deletion failed.".bright_red());
                            } else {
                                println!(
                                    "{}{}{}",
                                    "OK! Questionnaire".bright_green(),
                                    &id.to_string().bright_green(),
                                    "deleted.".bright_green()
                                );
                            }
                        }

                        Some("Ctrl-C") | Some("Ctrl-D") => clean_exit(),

                        _ => help("admin"),
                    };
                    if config.history {
                        rl.save_history(".gamify_history.txt").unwrap();
                    }
                    print!("\n");
                }
                Err(ReadlineError::Interrupted) => {
                    clean_exit();
                },
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
    };

    match role {
        "admin" => admin(),
        _ => {}
    }
}

fn login(client: &Client, username: &String, password: &String) -> Result<String, ()> {
    let params = [("username", username), ("pwd", password)];
    let res = client
        .post(&format!("{}{}", ENDPOINT::BASE_LINK, ENDPOINT::LOGIN))
        .form(&params)
        .timeout(Duration::from_secs(10))
        .send();
    if res.is_err() {
        println!(
            "{}{}{}",
            "Server ".bright_red(),
            ENDPOINT::BASE_LINK.bright_red(),
            " unreachable.".bright_red()
        );
        exit(0);
    }
    let res1 = res.unwrap();
    match res1.status() {
        StatusCode::OK => return Ok(res1.text().unwrap()),
        _ => return Err(()),
    };
}

fn list(client: &Client, start: &String, size: &String, past: bool) -> bool {
    let params = [
        ("start", start),
        ("size", size),
        ("past", &past.to_string()),
    ];
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

fn showAnswers(client: &Client, questionnaireId: &String, userId: i32) {
    let params = [
        ("questionnaireId", questionnaireId),
        ("userId", &userId.to_string()),
    ];
    let res = client
        .get(&format!(
            "{}{}",
            ENDPOINT::BASE_LINK,
            ENDPOINT::ADMIN_ANSWERS_RETRIEVAL
        ))
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send();

    if res.is_err() {
        println!("{}", "Error retrieving answers.".red());
    } else {
        let r: AnswerList = res.unwrap().json().unwrap();
        println!("{}", r);
    }
}

fn inspect(client: &Client, id: &String, canceled: bool) -> Result<i32, ()> {
    let params = [("id", id.as_str()), ("start", "0"), ("size", "100")];
    let endpoint = match canceled {
        true => ENDPOINT::ADMIN_INSPECT_CANCELED,
        _ => ENDPOINT::ADMIN_INSPECT,
    };

    let res = client
        .get(&format!("{}{}", ENDPOINT::BASE_LINK, endpoint))
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send();
    if res.is_err() {
        return Err(());
    }
    let res1 = res.unwrap();
    match res1.status() {
        StatusCode::OK => {
            let result: Vec<User> = res1.json().unwrap();
            if result.is_empty() {
                return Ok(-1);
            }
            const PROMPT: &str =
                "  ┌─ ID ──┬───────────── Name ─────────────┬────── Birth ─────┬── Sex ──┐";

            let multiselected: Vec<String> = (&result).iter().map(|u| format!("{}", u)).collect();
            let mut selection = Select::new()
                .with_prompt(PROMPT)
                .items(&multiselected[..])
                .interact();

            while selection.is_err() {
                selection = Select::new()
                    .with_prompt(PROMPT)
                    .items(&multiselected[..])
                    .interact();
            }

            return Ok(result[selection.unwrap()].userId);
        }
        _ => return Err(()),
    };
}

fn delete(client: &Client, id: &String) -> bool {
    let res = client
        .delete(&format!(
            "{}{}",
            ENDPOINT::BASE_LINK,
            ENDPOINT::ADMIN_DELETE
        ))
        .query(&[("id", id.as_str())])
        .timeout(Duration::from_secs(10))
        .send();
    if res.is_err() {
        return false;
    }
    return res.unwrap().status() == StatusCode::OK;
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
