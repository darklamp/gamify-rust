use colored::Colorize;
fn main() {
    println!("{} {}", "Gamify Admin CLI v.".bold().bright_blue(), env!("CARGO_PKG_VERSION").bright_blue());
    println!("{} v. {}", "Gamify Admin CLI".bold().green(), env!("CARGO_PKG_VERSION"));

}
