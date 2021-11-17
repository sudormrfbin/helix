use helix_term::commands::cmd::TYPABLE_COMMAND_LIST;

use std::{fs, path::PathBuf};

fn gen_typable_cmds() -> String {
    let mut md = String::new();
    md.push_str("| Name | Description |\n");
    md.push_str("| ---  | ---         |\n");

    let cmdify = |s: &str| format!("`:{}`", s);

    for cmd in TYPABLE_COMMAND_LIST {
        let names = std::iter::once(&cmd.name)
            .chain(cmd.aliases.iter())
            .map(|a| cmdify(a))
            .collect::<Vec<_>>()
            .join(", ");

        let entry = format!("| {} | {} |\n", names, cmd.doc);
        md.push_str(&entry);
    }

    md
}

const TYPABLE_COMMAND_MD_OUTPUT: &str = "typable-cmd.md";

fn book_gen_path() -> PathBuf {
    let helix_doc_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = helix_doc_crate.parent().unwrap();
    workspace_dir.join("book/src/generated/")
}

fn write(filename: &str, data: &str) {
    let error = format!("Could not write to {}", filename);
    let path = book_gen_path().join(filename);
    fs::write(path, data).expect(&error);
}

fn main() {
    write(TYPABLE_COMMAND_MD_OUTPUT, &gen_typable_cmds());
}
