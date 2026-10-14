use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: morph <file.mai>");
        return ExitCode::from(2);
    };

    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("error: could not read `{path}`: {err}");
            return ExitCode::from(2);
        }
    };

    match morph::run(&source, std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprint!("{}", err.render(path, &source));
            ExitCode::FAILURE
        }
    }
}
