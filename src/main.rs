use std::process;

use clap::Parser;

use nitrocop::cli::Args;

fn main() {
    // Use 4 MB stacks for rayon worker threads.
    // This further trims per-thread stack reservation overhead for typical
    // project lint runs while still leaving reasonable recursion headroom.
    rayon::ThreadPoolBuilder::new()
        .stack_size(4 * 1024 * 1024)
        .build_global()
        .ok();

    let args = Args::parse();
    match nitrocop::run(args) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(3);
        }
    }
}
