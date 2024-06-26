#![feature(box_patterns)]
#![feature(bool_to_option)]
#![feature(const_option)]
#![feature(const_fn_trait_bound)]
#![feature(box_syntax)]
#![feature(entry_insert)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(allocator_api)]
#![deny(clippy::disallowed_method)]
#![feature(slice_pattern)]
#![feature(associated_type_bounds)]
#![feature(nonzero_ops)]
#![feature(try_blocks)]
// this is to silence `.map_context()` for the time being
// #![allow(deprecated)]

use std::{io::Write, process::Command, sync::Arc, time::Instant};

use crate::{
    frontend::{
        builder::ProgramBuilder,
        js::{hosts::JSSATHostEnvironment, JavaScriptFrontend},
    },
    types::TypeCtx,
};

pub mod abst_interp;
pub mod backend;
pub mod codegen;
pub use jssat_ir::collections;
pub mod frontend;
pub use jssat_ir::id;
pub mod interner;
pub use jssat_interpreter as interpreter;
pub use jssat_ir::isa;
pub use jssat_ir::lifted;
pub mod my_tests;
pub mod opt;
pub use jssat_ir::retag;
use jssat_ir::{frontend::source_map::SourceMap, isa::AtomDealer, lifted::LiftedProgram};
use symbolic_execution::SystemRun;
pub mod symbolic_execution;
pub mod types;

/// can't have nice things :'( https://github.com/rust-lang/rust/issues/62633
pub trait UnwrapNone {
    fn expect_none(self, msg: &str);

    // lazy/hacky code but w/e
    fn expect_free(self);
}

impl<T> UnwrapNone for Option<T> {
    #[track_caller]
    fn expect_none(self, msg: &str) {
        assert!(matches!(self, None), "{}", msg);
    }

    #[track_caller]
    fn expect_free(self) {
        self.expect_none("must be free insertion slot");
    }
}

impl<L, R> UnwrapNone for bimap::Overwritten<L, R> {
    #[track_caller]
    fn expect_none(self, msg: &str) {
        assert!(
            matches!(self, bimap::Overwritten::<L, R>::Neither),
            "{}",
            msg
        );
    }

    #[track_caller]
    fn expect_free(self) {
        self.expect_none("must be free insertion slot");
    }
}

fn preview(command: &Command) -> String {
    let mut preview = String::new();

    preview.push_str(command.get_program().to_str().unwrap());

    for arg in command.get_args() {
        preview.push_str("\n\t");
        preview.push_str(arg.to_str().unwrap());
    }

    preview
}

fn link_binary(build: &[u8]) {
    let runtime_library = include_bytes!(env!("JSSATRT_PATH"));
    println!("included runtime size: {}", runtime_library.len());

    let mut runtime_object = tempfile::NamedTempFile::new().unwrap();
    let mut build_object = tempfile::NamedTempFile::new().unwrap();

    runtime_object.write_all(runtime_library).unwrap();
    build_object.write_all(build).unwrap();

    #[cfg(target_os = "windows")]
    let artifact = "jssatout.exe";
    #[cfg(target_os = "linux")]
    let artifact = "jssatout";
    #[cfg(target_os = "macos")]
    let artifact = "jssatout";

    let mut build = cc::Build::new();

    // sensible defaults for `OPT_LEVEL`, `TARGET`, and `HOST`
    if std::env::var("OPT_LEVEL").is_err() {
        build.opt_level(3);
    }

    if std::env::var("TARGET").is_err() {
        build.target(crate::backend::llvm::target_triplet().as_str());
    }

    if std::env::var("HOST").is_err() {
        build.host(crate::backend::llvm::target_triplet().as_str());
    }

    build.flag_if_supported("-flto");

    let mut command = build.get_compiler().to_command();

    command.arg("-o").arg(artifact);

    #[cfg(target_os = "linux")]
    {
        command
            .arg(format!("{}", build_object.path().display()))
            .arg(format!("{}", runtime_object.path().display()))
            .arg("-pthread")
            .arg("-ldl");
    }

    // // TODO: is this the right way to add `advapi32` to the linker flags?
    // //       we need advapi32 for mimalloc
    // #[cfg(target_os = "windows")]
    // build.object("advapi32");

    // build.file(runtime_object.path()).file(build_object.path());

    // build.compile(artifact);

    eprintln!("invoking: {}", preview(&command));
    assert!(command.spawn().unwrap().wait().unwrap().success());

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    std::compile_error!("unimplemented platform");
}

fn main() {
    // let file_name = std::env::args()
    //     .into_iter()
    //     .skip(1)
    //     .next()
    //     .expect("expected file name");

    // let content = std::fs::read_to_string(file_name).expect("expected to read
    // file");
    let content = r#"
function f(x) {
    x('Hello, World!');
}

f(print);
"#
    .to_owned();

    let mut builder = ProgramBuilder::new();
    let mut f = builder.start_function_main();
    let mut b = f.start_block_main();

    let mut frontend = JavaScriptFrontend::new(&mut builder);
    let result = frontend
        .parse(&content, &mut b, &mut JSSATHostEnvironment::new())
        .expect("should parse js");
    let source_map = frontend.ecma_methods.source_map;

    f.end_block(b.ret(Some(result)));
    builder.end_function(f);
    let ir = builder.finish();

    let dealer = ir.dealer.clone();

    // println!("{}", crate::frontend::display_jssatir::display(&ir));

    println!("lifting program");
    let program = time(move || lifted::lift(ir));

    println!("executing program");
    interpret(&program, dealer, source_map);
    let (result, collector) = time(|| {
        let mut engine = abst_interp::AbsIntEngine::new_with_collector(
            &program,
            abst_interp::MomentCollector::new(&program),
        );

        let result = engine.call(program.entrypoint, TypeCtx::new());
        (result, engine.collector)
    });

    match result {
        Ok(_result) => {
            println!("ran successfully!");
        }
        Err(err) => {
            println!("error: {:?}", err);

            let listen_url = "127.0.0.1:8000";
            println!("preparing data for domino");
            let data = collector.moment.into_data_with(dealer, source_map);
            println!("starting domino on http://{listen_url}");
            domino::launch(listen_url, &data).unwrap();
        }
    }
}

fn rest(program: SystemRun) {
    println!("typing program");
    let program = time(move || codegen::type_program(program));

    println!("optimizing system run");
    let program = time(move || opt::opt(program));

    println!("{}", codegen::display_typed(&program));

    println!("lowering run");
    let program = time(move || codegen::lower(program));

    println!("{}", codegen::display_program(&program));

    println!("compiling");
    let build = time(move || backend::compile(program));

    eprintln!("OUTPUT LLVM IR (use unix pipes to redirect this into a file):");
    println!("{}", build.llvm_ir);

    link_binary(build.obj.as_slice());
}

fn time<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let end = Instant::now();
    println!("took {:?}", end - start);
    println!();
    result
}

fn interpret(program: &LiftedProgram, dealer: Arc<AtomDealer>, source_map: SourceMap) -> ! {
    // let program = time(|| symbolic_execution::execute(&program));
    let builder = crate::interpreter::InterpreterBuilder::new(program);
    let mut interpreter = builder.build();

    let (interpreter, interpreter_result) = time(|| {
        let result = interpreter.execute_fn_id(program.entrypoint, vec![]);
        (interpreter, result)
    });

    println!("executed");

    println!(
        "executed: {:?}",
        match interpreter_result {
            Ok(value) => format!("success: {:?}", value),
            // Ok(_) => "success".to_string(),
            Err(err) => format!("error: {}", err),
            // Err(_) => "error".to_string(),
        }
    );

    let listen_url = "127.0.0.1:8000";
    println!("preparing data for domino");
    let data = interpreter.moment.into_data_with(dealer, source_map);
    println!("starting domino on http://{listen_url}");
    domino::launch(listen_url, &data).unwrap();

    panic!("done");
}
