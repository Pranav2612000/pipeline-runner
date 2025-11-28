mod error;
mod job;
mod pipeline;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    file_path: String,
}

fn main() {
    let args = Args::parse();

    let executor = pipeline::Pipeline::new_with_params(args.file_path);
    match executor.run() {
        Ok(_) => println!("Execution completed successfully"),
        Err(e) => println!("Execution failed Error: {:?}", e),
    }
}
