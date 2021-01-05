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
use serde_yaml::Value;
use std::{env, process::exit, time::Duration};

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
    const LOGIN: &'static str = "CheckLogin";
    const ADMIN_LIST: &'static str = "admin/listQuestionnaires";
    const ADMIN_CREATE: &'static str = "admin/create";
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

    let mut rl = rustyline::Editor::<()>::new();
    let yaml = load_yaml!("../cli.yaml");
    let matches = App::from(yaml).get_matches();
    let f = std::fs::File::open("config.yaml").expect("Config file not found");
    let config: Value = serde_yaml::from_reader(f).unwrap();

    let client = reqwest::blocking::Client::builder()
        .user_agent(&*USER_AGENT)
        .cookie_store(true)
        .build()
        .unwrap();

    if login(
        &client,
        config["username"].as_str().unwrap().to_string(),
        config["password"].as_str().unwrap().to_string(),
    ) {
        println!("{}", "Login OK".bold().green());
    } else {
        println!("{}", "Login KO".bold().red());
        return;
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

    loop {
        let mut readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                match &*line {
                    "CTRL+C" | "CTRL+D" | "SHIFT+B" => {
                        println!("{}", "byee ^^".bright_blue().italic());
                        return;
                    }

                    "admin" => loop {
                        readline = rl.readline(&*format!("{}", "admin >> ".bright_blue()));
                        match readline {
                            Ok(line) => {
                                match &*line {
                                    "b" => {
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
                                                if !Confirm::new()
                                                    .with_prompt("Continue?")
                                                    .interact()
                                                    .unwrap()
                                                {
                                                    break;
                                                } else {
                                                    question_count += 1;
                                                }
                                            }
                                        }
                                        if create_questionnaire(
                                            &client, name, date, image, questions,
                                        ) {
                                            println!(
                                                "{}",
                                                "Questionnaire submitted successfully!"
                                                    .bright_green()
                                            );
                                            break;
                                        } else {
                                            println!(
                                                "{}",
                                                "Questionnaire submission failed!".bright_red()
                                            );
                                        }
                                    }

                                    _ => {}
                                };
                                print!("\n");
                            }
                            Err(ReadlineError::Interrupted) => {
                                clean_exit();
                            }
                            _ => {
                                println!("{}", "Error in command. Please enter correct command or press CTRL+C to exit".bold().red())
                            }
                        }
                    },

                    "user" => {}
                    _ => {}
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
    println!("\n{}", "byee ^^".bright_blue().italic());
    exit(0);
}
