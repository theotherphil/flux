use failure::ResultExt;
use exitfailure::ExitFailure;
use serde::{Serialize, Deserialize};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    /// Path to file containing dataflow specification.
    #[structopt(parse(from_os_str), long = "input", short = "i")]
    input: std::path::PathBuf,
}

/// cargo run -- -i src/sample.json | dot -Tsvg >> sample.svg
fn main() -> Result<(), ExitFailure> {
    let args = Cli::from_args();

    let content = std::fs::read_to_string(&args.input)
        .with_context(
            |_| format!("could not read file '{:?}'", &args.input)
        )?;

    let graph: Graph = serde_json::from_str(&content).expect("parse failure");
    render(&mut std::io::stdout(), &graph)?;
    Ok(())
}

fn render<W: std::io::Write>(w: &mut W, graph: &Graph) -> std::io::Result<()> {
    writeln!(w, "digraph G {{")?;
    for d in &graph.data {
        writeln!(w, "{} [shape=box]", d.name)?;
    }
    for f in &graph.functions {
        writeln!(w, "{} [shape=ellipse]", f.name)?;
        for i in &f.inputs {
            writeln!(w, "{} -> {}", i, f.name)?;
        }
        for o in &f.outputs {
            writeln!(w, "{} -> {}", f.name, o)?;
        }
    }
    writeln!(w, "}}")?;
    Ok(())
}

/// A dataflow graph.
#[derive(Debug, Serialize, Deserialize)]
struct Graph {
    data: Vec<Data>,
    functions: Vec<Function>,
}

/// A piece of data in a dataflow graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Data {
    /// The name of this data, as shown on the
    /// rendered diagram.
    name: String,
    /// The name of the application or service that maintains
    /// or provides this data.
    source: String,
}

/// A process in a dataflow graph, i.e. a function.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Function {
    /// The name of this function, as shown on the
    /// rendered diagram.
    name: String,
    /// The process or service which performs this process.
    owner: String,
    /// Inputs to this function. To render a graph, each input needs
    /// to have a corresponding Data instance.
    inputs: Vec<String>,
    /// Outputs from this function. To render a graph, each output needs
    /// to have a corresponding Data instance.
    outputs: Vec<String>,
}
