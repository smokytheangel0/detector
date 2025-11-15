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
    //parse verus using vest with dsl translated from the verus.pest file.
        //functions linked with struct and sother impls if method or associated function
            //exec  sig + body
            //spec  sig + body
            //proof sig + body
        //structs
        //enums
        //primatives
    //generate pluto controls using jlrs
        //snippet box controls
            //|prettified exec only
                //this requires generating translations for code using vstd idioms
            //|a function its the proofs, specs, linked with the other functions, proofs and specs that it depends on
            //|auto matching highlighted assembly ()
                //0 => new browser window (compiler-explorer)
                //1..n => new tab in (compiler-explorer)


        
        

}