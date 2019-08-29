use failure::ResultExt;
use exitfailure::ExitFailure;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    /// Path to file containing dataflow specification.
    #[structopt(parse(from_os_str), long = "input", short = "i")]
    input: std::path::PathBuf,
}

/// cargo run -- -i src/sample.yaml | dot -Tsvg >> sample.svg
fn main() -> Result<(), ExitFailure> {
    let args = Cli::from_args();

    let content = std::fs::read_to_string(&args.input)
        .with_context(
            |_| format!("could not read file '{:?}'", &args.input)
        )?;

    let graph: Graph = serde_yaml::from_str(&content).expect("parse failure");
    render(&mut std::io::stdout(), &graph)?;
    Ok(())
}

struct DotBuilder {
    lines: Vec<String>
}

impl DotBuilder {
    fn new() -> DotBuilder {
        DotBuilder { lines: Vec::new() }
    }

    fn add<S: Into<String>>(&mut self, line: S) {
        self.lines.push(line.into());
    }

    fn add_node(&mut self, node: &Node) {
        self.lines.push(node.to_string());
    }

    fn render<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        for line in &self.lines {
            writeln!(w, "{}", line)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Shape {
    Ellipse
}

impl Shape {
    fn to_string(self) -> String {
        match self {
            Shape::Ellipse => "ellipse".into()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Style {
    Filled
}

impl Style {
    fn to_string(self) -> String {
        match self {
            Style::Filled => "filled".into()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ColorScheme {
    Dark28
}

impl ColorScheme {
    fn to_string(self) -> String {
        match self {
            ColorScheme::Dark28 => "dark28".into()
        }
    }

    fn num_colors(self) -> usize {
        match self {
            ColorScheme::Dark28 => 8
        }
    }
}

struct Node {
    name: String,
    attributes: HashMap<String, String>,
}

impl Node {
    fn new(name: &str) -> Node {
        Node {
            name: name.to_string(),
            attributes: HashMap::new()
        }
    }

    fn style(&mut self, style: Style) -> &mut Self {
        self.attribute("style", style.to_string())
    }

    fn shape(&mut self, shape: Shape) -> &mut Self {
        self.attribute("shape", shape.to_string())
    }

    fn fillcolor(&mut self, color_scheme: ColorScheme, color: &str) -> &mut Self {
        self.attribute(
            "fillcolor",
            format!("\"/{}/{}\"", color_scheme.to_string(), color)
        )
    }

    fn attribute<S: Into<String>, T: Into<String>>(
        &mut self, name: S, value: T) -> &mut Self {
        self.attributes.insert(name.into(), value.into());
        self
    }

    fn to_string(&self) -> String {
        let mut attrs: Vec<String> = self
            .attributes
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        attrs.sort();
        format!("{}[{}];", self.name, attrs.join("="))
    }
}

#[test]
fn node_to_string() {
    let mut n = Node::new("foo");
    let n = n
        .shape(Shape::Ellipse)
        .style(Style::Filled)
        .fillcolor(ColorScheme::Dark28, "1");
    assert_eq!(n.to_string(), "fff");
}

fn render<W: std::io::Write>(w: &mut W, graph: &Graph) -> std::io::Result<()> {
    // See https://www.graphviz.org/doc/info/colors.html for the definitions
    // of the colour schemes. Functions are colored according to their owner,
    // wrapping if we run out of colours.
    const NUM_COLOURS: usize = 8;
    const COLOUR_SCHEME: &str = "dark28";

    let mut owners: Vec<String> = graph
        .functions
        .iter()
        .map(|f| f.owner.clone())
        .collect();

    owners.sort();
    owners.dedup();

    let colours: HashMap<String, String> = owners
        .iter()
        .enumerate()
        .map(|(count, owner)| {
            let c = count % NUM_COLOURS + 1;
            //(owner.clone(), format!("\"/{}/{}\"", COLOUR_SCHEME, c))
            (owner.clone(), c.to_string())
        })
        .collect();

    let mut g = DotBuilder::new();
    g.add("digraph G {{");
    for d in &graph.data {
        g.add(format!("{} [shape=box]", d.name));
    }

    for f in &graph.functions {
        g.add_node(Node::new(&f.name)
            .shape(Shape::Ellipse)
            .style(Style::Filled)
            .fillcolor(ColorScheme::Dark28, &colours[&f.owner])
        );
        // g.add(format!(
        //     "{} [shape=ellipse,style=filled,fillcolor={}]",
        //     f.name,
        //     &colours[&f.owner]
        // ));
        for i in &f.inputs {
            g.add(format!(
                "{} -> {}", i, f.name
            ));
        }
        for o in &f.outputs {
            g.add(format!(
                "{} -> {}", f.name, o
            ));
        }
    }
    g.add("subgraph cluster_legend {{");
    g.add("label=\"Legend\"");
    g.add("rankdir=TB");
    let mut ordering = String::new();
    for (name, color) in &colours {
        g.add(format!(
            "legend_{} [label={},style=filled,fillcolor={}]",
            name,
            name,
            color
        ));
        if !ordering.is_empty() {
            ordering = ordering + "->";
        }
        ordering = format!("{}legend_{}", ordering, name);
    }
    ordering = ordering + "[style=invis]";
    g.add(format!("{}", ordering));
    g.add("}}");
    g.add("}}");

    g.render(w)?;

    Ok(())
}

macro_rules! attrs {
    ( $( $attr:expr => $value:expr),* ) => {
        {
            let mut vals = Vec::new();
            $(
                vals.push(format!("{}={}", $attr, $value));
            )*
            let res = vals.join(",");
            format!("[{}]", res)
        }
    }
}

#[test]
fn foo() {
    let a = attrs!(
        "label" => "foo",
        "style" => "filled",
        "fillcolor" => "red"
    );
    assert_eq!(a, String::from("[label=foo,style=filled,fillcolor=red]"));
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
    /// Human-readable description of this data.
    description: Option<String>,
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
