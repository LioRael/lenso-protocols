use std::{env, path::Path, process::ExitCode};

use lenso_contract_codegen::{
    ProjectionLanguage, check_generated, check_projection, lint_compatibility, write_generated,
    write_projection,
};

fn usage() -> &'static str {
    "usage:\n  lenso-contract-codegen generate <descriptor> <rust-output> <typescript-output>\n  lenso-contract-codegen generate <descriptor> --rust <output>\n  lenso-contract-codegen generate <descriptor> --rust-runtime <output>\n  lenso-contract-codegen generate <descriptor> --typescript <output>\n  lenso-contract-codegen generate <descriptor> --wit <output>\n  lenso-contract-codegen check <descriptor> <rust-output> <typescript-output>\n  lenso-contract-codegen check <descriptor> --rust <output>\n  lenso-contract-codegen check <descriptor> --rust-runtime <output>\n  lenso-contract-codegen check <descriptor> --typescript <output>\n  lenso-contract-codegen check <descriptor> --wit <output>\n  lenso-contract-codegen lint <old-descriptor> <new-descriptor>"
}

fn selected_projection(arguments: &[String]) -> Option<(ProjectionLanguage, &Path)> {
    if arguments.len() != 4 {
        return None;
    }
    let language = match arguments[2].as_str() {
        "--rust" => ProjectionLanguage::Rust,
        "--rust-runtime" => ProjectionLanguage::RustRuntime,
        "--typescript" => ProjectionLanguage::TypeScript,
        "--wit" => ProjectionLanguage::Wit,
        _ => return None,
    };
    Some((language, Path::new(&arguments[3])))
}

fn run(arguments: &[String]) -> Result<(), String> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(usage().to_owned());
    };
    match command {
        "generate" => match selected_projection(arguments) {
            Some((language, output)) => {
                write_projection(Path::new(&arguments[1]), language, output)
                    .map_err(|error| error.to_string())
            }
            None if arguments.len() == 4 => write_generated(
                Path::new(&arguments[1]),
                Path::new(&arguments[2]),
                Path::new(&arguments[3]),
            )
            .map_err(|error| error.to_string()),
            None => Err(usage().to_owned()),
        },
        "check" => match selected_projection(arguments) {
            Some((language, output)) => {
                check_projection(Path::new(&arguments[1]), language, output)
                    .map_err(|error| error.to_string())
            }
            None if arguments.len() == 4 => check_generated(
                Path::new(&arguments[1]),
                Path::new(&arguments[2]),
                Path::new(&arguments[3]),
            )
            .map_err(|error| error.to_string()),
            None => Err(usage().to_owned()),
        },
        "lint" if arguments.len() == 3 => {
            lint_compatibility(Path::new(&arguments[1]), Path::new(&arguments[2]))
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        _ => Err(usage().to_owned()),
    }
}

fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "tests/fixtures/profile/capability.json";

    #[test]
    fn one_language_projection_does_not_require_a_peer_output() {
        let root = std::env::temp_dir().join(format!(
            "lenso-contract-codegen-cli-projection-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temporary directory should be created");
        let output = root.join("bindings.ts");
        let output = output.to_string_lossy().into_owned();

        run(&[
            "generate".to_owned(),
            FIXTURE.to_owned(),
            "--typescript".to_owned(),
            output.clone(),
        ])
        .expect("TypeScript projection should generate independently");
        run(&[
            "check".to_owned(),
            FIXTURE.to_owned(),
            "--typescript".to_owned(),
            output,
        ])
        .expect("TypeScript projection should check independently");

        assert!(!root.join("bindings.rs").exists());
        std::fs::remove_dir_all(root).expect("temporary directory should be removable");
    }

    #[test]
    fn runtime_codec_projection_is_selectable() {
        let root = std::env::temp_dir().join(format!(
            "lenso-contract-codegen-cli-runtime-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temporary directory should be created");
        let output = root.join("runtime.rs").to_string_lossy().into_owned();

        run(&[
            "generate".to_owned(),
            FIXTURE.to_owned(),
            "--rust-runtime".to_owned(),
            output.clone(),
        ])
        .expect("runtime codec projection should generate independently");
        run(&[
            "check".to_owned(),
            FIXTURE.to_owned(),
            "--rust-runtime".to_owned(),
            output,
        ])
        .expect("runtime codec projection should check independently");

        std::fs::remove_dir_all(root).expect("temporary directory should be removable");
    }
}
