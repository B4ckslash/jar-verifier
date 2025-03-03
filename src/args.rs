use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    ///Classpath of JARs to be checked
    pub classpath: String,
    ///Whether the program runs in parallel
    #[arg(short, long, default_value_t = false)]
    pub parallel: bool,
}
