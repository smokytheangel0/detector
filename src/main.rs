use jlrs::prelude::*;

fn main() {
    let handle = Builder::new().start_local().expect("cannot init Julia");

    handle.local_scope::<_, 1>(|mut frame| {
        // Safety: we only evaluate a print statement, which is perfectly safe.
        unsafe {
            Value::eval_string(&mut frame, "
                using Pkg
                Pkg.activate()
                Pkg.add(\"PlutoUI\")
                Pkg.add(\"Kroki\")
                Pkg.add(\"ShortCodes\")
                Pkg.add(\"PlutoTeachingTools\")
                Pkg.add(\"MarkdownLiteral\")
                Pkg.add(\"InteractiveUtils\")
                Pkg.add(\"Markdown\")
                Pkg.instantiate()
                import Pluto
                Pluto.run(notebook = joinpath(pwd(), \"whitepaper.jl\"))")
        }.expect("an exception occurred");
    });
}