use std::path::PathBuf;

use qed_ast::Position;
use qed_interpreter::Interpreter;
use qed_utils::LspArgs;
use qedlang_core::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};

pub fn run(args: LspArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let entry = PathBuf::from(args.file);
    let (_typechecker, ctx) = interpreter.typecheck(entry.clone())?;

    let file_id = ctx.program.file_resolver.resolve_id(&entry).unwrap();
    let position = Position {
        file_id: *file_id,
        line: args.pos[0],
        column: args.pos[1],
    };

    let location = ctx.position_to_location(position).unwrap();

    println!("-----------------------------------------------");

    match args.method.as_str() {
        "hover" => {
            println!("{}", ctx.hover(location).unwrap());
        }
        "definition" => {
            let (start_pos, end_pos) = ctx
                .location_to_position(ctx.goto_definition(location).unwrap())
                .unwrap();
            println!(
                "{}, {}",
                ctx.position_to_file_path(start_pos).unwrap(),
                ctx.position_to_file_path(end_pos).unwrap()
            );
        }
        "references" => {
            let references = ctx.find_all_references(location, true, false).unwrap();
            let references_pos_pathes = references
                .iter()
                .map(|location| {
                    let (start_pos, end_pos) = ctx.location_to_position(*location).unwrap();
                    format!(
                        "{}, {}",
                        ctx.position_to_file_path(start_pos).unwrap(),
                        ctx.position_to_file_path(end_pos).unwrap(),
                    )
                })
                .collect::<Vec<_>>();

            println!("{}", references_pos_pathes.join("\n"));
        }
        _ => todo!(),
    }

    Ok(())
}
