#![allow(non_snake_case)]

use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use figlet_rs::FIGfont;
use lazy_static::lazy_static;
use native_dialog::FileDialog;
use reqwest::{
    blocking::{multipart, Client},
    StatusCode,
};
use rustyline::error::ReadlineError;
use serde::Deserialize;
use std::{env, fmt::Display, process::exit, time::Duration};
extern crate term_size;

#[non_exhaustive]
struct ENDPOINT;
impl ENDPOINT {
    const DEFAULT_BASE_LINK: &'static str = "http://localhost:8080/GamifyUser/";
    const CAMPAIGN_IMAGES: &'static str = "uploads/campaignImages/";
    const LOGIN: &'static str = "CheckLogin";
    const ADMIN_LIST: &'static str = "admin/listQuestionnaires";
    const ADMIN_CREATE: &'static str = "admin/create";
    const ADMIN_DELETE: &'static str = "admin/delete";
    const ADMIN_INSPECT: &'static str = "admin/listQuestionnaireCompletedUsers";
    const ADMIN_INSPECT_CANCELED: &'static str = "admin/listQuestionnaireCanceledUsers";
    const ADMIN_ANSWERS_RETRIEVAL: &'static str = "admin/getAnswers";
}

lazy_static! {
    static ref USER_AGENT: String = format!(
        "gamify-rust / {} / {}",
        env!("CARGO_PKG_VERSION"),
        env::consts::OS
    );
    //static ref BASE_LINK: &'static str = "asd";
    // read config
    static ref CONFIG: Config = {
        let f = std::fs::File::open("config.yaml").expect("Config file not found");
        serde_yaml::from_reader(f).expect("Config not readable")
    };
    static ref TERMINAL_DIMENSIONS: (usize, usize) = {
        if let Some((w, h)) = term_size::dimensions() {
            (w,h)
        } else {
            if CONFIG.debug {
                println!("Unable to get term size :(")
            }
            (137, 35)
        }
    };


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

#[derive(Deserialize)]
struct User {
    userId: i32,
    birth: String,
    sex: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    username: String,
    password: String,
    debug: bool,
    history: bool,
    #[serde(default = "default_base_link")]
    #[serde(alias = "baselink")]
    base_link: String,
}

fn default_base_link() -> String {
    ENDPOINT::DEFAULT_BASE_LINK.to_string()
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
    if CONFIG.debug {
        println!("Terminal dimensions: {:#?}", &*TERMINAL_DIMENSIONS);
    }

    println!(
        "{:~^width$}\n{:^width$}\n{:^width$}\n{:^width$}",
        " Gamify CLI ".bold().on_black().bold().bright_blue(),
        env!("CARGO_PKG_VERSION").bright_blue(),
        "https://github.com/darklamp/gamify-rust".magenta(),
        "Press CTRL+C to exit or anything to get help."
            .italic()
            .white(),
        width = TERMINAL_DIMENSIONS.0,
    );

    let ascii_font = FIGfont::from_file("resources/isometric3.flf").unwrap();

    let mut rl = rustyline::Editor::<()>::new();

    // try and load history file if history option is on
    if CONFIG.history {
        if rl.load_history(".gamify_history.txt").is_err() && CONFIG.debug {
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

    match login(&client, &CONFIG.username, &CONFIG.password) {
        Ok(res) => {
            foo = res.clone();
            role = foo.strip_prefix("/GamifyUser/").unwrap();
            if CONFIG.debug {
                println!("{}", "Login OK".bold().green());
            }
            if (CONFIG.username.len() + 3) <= (TERMINAL_DIMENSIONS.0 as f64 / 13.6) as usize {
                println!(
                    "{}",
                    ascii_font
                        .convert(format!("Hi {}", CONFIG.username).as_str())
                        .unwrap()
                        .to_string()
                        .blue()
                );
            } else {
                println!(
                    "{}\n{}",
                    ascii_font.convert("Hi").unwrap().to_string().blue(),
                    ascii_font
                        .convert(format!("{}", CONFIG.username).as_str())
                        .unwrap()
                        .to_string()
                        .blue()
                );
            }
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
        let prompt = format!("{}{}", &(CONFIG.username).blue(), " >> ".blue());
        loop {
            let readline = rl.readline(&*prompt);
            match readline {
                Ok(line) => {
                    if CONFIG.history {
                        rl.add_history_entry(line.as_str());
                    }
                    let mut toks = line.split(' ').fuse();
                    match toks.next() {
                        Some("b") | Some("back") | Some("exit") => {
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
                            let mut start: String = String::from("0");
                            let mut size: String = String::from("100");
                            let mut past: String = String::from("n");

                            let mut initialized: [bool; 3] = [false, false, false];
                            loop {
                                match toks.next() {
                                    Some(a) => match a {
                                        "d" | "default" => {
                                            initialized[0] = true;
                                            initialized[1] = true;
                                            initialized[2] = true;

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
                                            past = "y".to_string();
                                            initialized[2] = true;
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
                            let mut id: String = String::from("");
                            let mut canceled: String = String::from("");

                            let mut initialized: [bool; 2] = [false, false];
                            loop {
                                match toks.next() {
                                    Some(a) => {
                                        id = a.to_string();
                                        initialized[0] = true;
                                        initialized[1] = true;
                                        break;
                                    }
                                    None => break,
                                }
                            }

                            if !initialized[0] {
                                id = Input::new()
                                    .with_prompt("Questionnaire ID [default: 0]")
                                    .default("0".into())
                                    .interact_text()
                                    .unwrap();
                            }
                            if !initialized[1] {
                                canceled = Input::new()
                                    .with_prompt("Canceled users?")
                                    .default("n".into())
                                    .interact_text()
                                    .unwrap();
                            }

                            let p: bool = canceled.to_lowercase().contains("y");

                            match inspect(&client, &id, p) {
                                Ok(uId) => {
                                    if uId == -1 {
                                        let word = match p {
                                            true => "canceled",
                                            _ => "answered",
                                        };
                                        print!(
                                            "{0} {1} {2}",
                                            "No one".blue(),
                                            word.blue(),
                                            "yet!".blue()
                                        );
                                    } else {
                                        showAnswers(&client, &id, uId);
                                    }
                                }
                                Err(_) => {
                                    print!("{}", "Error in retrieving data. You probably provided a non-existent id. ಠ_ಠ".bright_red());
                                }
                            };
                        }

                        Some("delete") => {
                            let id: String = Input::new()
                                .with_prompt("Questionnaire ID")
                                .interact_text()
                                .unwrap();

                            if !delete(&client, &id) {
                                print!("{}", "Deletion failed.".bright_red());
                            } else {
                                print!(
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
                    if CONFIG.history {
                        rl.save_history(".gamify_history.txt").unwrap();
                    }
                    print!("\n");
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    clean_exit();
                }
                _ => {
                    print!(
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
        .post(&format!("{}{}", CONFIG.base_link, ENDPOINT::LOGIN))
        .form(&params)
        .timeout(Duration::from_secs(10))
        .send();
    if res.is_err() {
        print!(
            "{}{}{}",
            "Server ".bright_red(),
            CONFIG.base_link.bright_red(),
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
        .get(&format!("{}{}", CONFIG.base_link, ENDPOINT::ADMIN_LIST))
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send();
    let res1 = res.unwrap();
    match res1.status() {
        StatusCode::OK => {
            let result: Vec<Questionnaire> = res1.json().unwrap();
            println!(
                "{:^width$}",
                "┌─ ID ──┬───────────── Name ─────────────┬────── Date ──────┐",
                width = TERMINAL_DIMENSIONS.0
            );
            for r in result {
                let date0: Vec<&str> = r.datetime.split(",").collect();
                let datel = format!("{} {}", date0[0], date0[1]);

                println!(
                    "{:^width$}",
                    format!(
                        "{}{}{}",
                        format!("│ {:^5} ", r.questionnaireId.to_string().blue()),
                        format!("│ {:^30} ", r.name.bright_blue().bold()),
                        format!("│ {:>16} │", format! {"{}", datel})
                    ),
                    width = TERMINAL_DIMENSIONS.0 + 20
                );
            }
            println!(
                "{:^width$}",
                "└─ ID ──┴───────────── Name ─────────────┴────── Date ──────┘",
                width = TERMINAL_DIMENSIONS.0
            );
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
            CONFIG.base_link,
            ENDPOINT::ADMIN_ANSWERS_RETRIEVAL
        ))
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send();

    if res.is_err() {
        print!("{}", "Error retrieving answers.".red());
    } else {
        let r: AnswerList = res.unwrap().json().unwrap();

        let line = format!(
            "{}{}{}",
            format!(
                "Age: {}, ",
                r.stats
                    .get(0)
                    .unwrap_or(&None)
                    .clone()
                    .unwrap_or("N/A".to_string())
                    .bright_purple()
            ),
            format!(
                "Sex: {}, ",
                r.stats
                    .get(1)
                    .unwrap_or(&None)
                    .clone()
                    .unwrap_or("N/A".to_string())
                    .bright_purple()
            ),
            format!(
                "Exp: {}",
                r.stats
                    .get(2)
                    .unwrap_or(&None)
                    .clone()
                    .unwrap_or("N/A".to_string())
                    .bright_purple()
            )
        );

        println!(
            "\n{:~^width$}",
            " Statistical answers ",
            width = TERMINAL_DIMENSIONS.0
        );

        println!("\n{:^width$}\n", line, width = TERMINAL_DIMENSIONS.0 + 27);

        println!(
            "{:~^width$}\n",
            " Optional answers ",
            width = TERMINAL_DIMENSIONS.0
        );

        println!("{:width$}", "".on_purple(), width = TERMINAL_DIMENSIONS.0);

        for x in r.opt {
            println!(
                "{:^width$}",
                x.question.bright_yellow(),
                width = TERMINAL_DIMENSIONS.0
            );
            println!(
                "{:^width$}",
                x.content.bright_white(),
                width = TERMINAL_DIMENSIONS.0
            );
            println!("{:width$}", "".on_purple(), width = TERMINAL_DIMENSIONS.0);
        }

        println!("\n{:~^width$}", "", width = TERMINAL_DIMENSIONS.0);
    }
}

fn inspect(client: &Client, id: &String, canceled: bool) -> Result<i32, ()> {
    let params = [("id", id.as_str()), ("start", "0"), ("size", "100")];
    let endpoint = match canceled {
        true => ENDPOINT::ADMIN_INSPECT_CANCELED,
        _ => ENDPOINT::ADMIN_INSPECT,
    };

    let res = client
        .get(&format!("{}{}", CONFIG.base_link, endpoint))
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
                "   ─ ID ──┬───────────── Name ─────────────┬────── Birth ─────┬── Sex ── ";

            let multiselected: Vec<String> = (&result).iter().map(|u| format!("{}", u)).collect();
            let mut selection = Select::new()
                .with_prompt(PROMPT)
                .items(&multiselected[..])
                .clear(true)
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
        .delete(&format!("{}{}", CONFIG.base_link, ENDPOINT::ADMIN_DELETE))
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
    const QNAMES: [&str; 6] = [
        "Question0",
        "Question1",
        "Question2",
        "Question3",
        "Question4",
        "Question5",
    ];

    for q in questions {
        form = form.text(QNAMES[counter], q);
        counter += 1;
    }
    let res = client
        .post(&format!("{}{}", CONFIG.base_link, ENDPOINT::ADMIN_CREATE))
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
    println!("\n{}\n", " (ᵟຶ︵ ᵟຶ) bye (ᵟຶ︵ ᵟຶ) ".bright_blue());
    exit(0);
}
