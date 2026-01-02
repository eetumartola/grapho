use std::path::{Path, PathBuf};
use std::process;

use grapho_core::Project;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HeadlessPlan {
    #[serde(default)]
    nodes: Vec<PlanNode>,
    #[serde(default)]
    links: Vec<PlanLink>,
    #[serde(default)]
    output_node: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlanNode {
    name: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    inputs: Vec<PlanPin>,
    #[serde(default)]
    outputs: Vec<PlanPin>,
}

#[derive(Debug, Deserialize)]
struct PlanPin {
    name: String,
    pin_type: grapho_core::PinType,
}

#[derive(Debug, Deserialize)]
struct PlanLink {
    from: PlanEndpoint,
    to: PlanEndpoint,
}

#[derive(Debug, Deserialize)]
struct PlanEndpoint {
    node: String,
    pin: String,
}

struct HeadlessArgs {
    plan_path: Option<PathBuf>,
    save_path: Option<PathBuf>,
    print: bool,
}

pub fn maybe_run_headless(args: &[String]) -> Result<bool, String> {
    if !args
        .iter()
        .any(|arg| arg == "--headless" || arg == "-headless")
    {
        return Ok(false);
    }

    let parsed = parse_headless_args(args)?;
    let plan = if let Some(path) = parsed.plan_path {
        load_headless_plan(&path)?
    } else {
        default_headless_plan()
    };

    let project = build_project_from_plan(&plan)?;

    if let Some(path) = parsed.save_path {
        save_project_json(&project, &path)?;
        tracing::info!("headless: saved project to {:?}", path);
    }

    if parsed.print {
        let json = serde_json::to_string_pretty(&project).map_err(|err| err.to_string())?;
        println!("{json}");
    }

    if let Some(output) = plan.output_node {
        validate_topo_sort(&project, &output)?;
    }

    tracing::info!("headless: completed");
    Ok(true)
}

fn parse_headless_args(args: &[String]) -> Result<HeadlessArgs, String> {
    let mut plan_path = None;
    let mut save_path = None;
    let mut print = false;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--headless" | "-headless" => {}
            "--plan" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--plan requires a path".to_string())?;
                plan_path = Some(PathBuf::from(value));
            }
            "--save" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--save requires a path".to_string())?;
                save_path = Some(PathBuf::from(value));
            }
            "--print" => {
                print = true;
            }
            "--help" | "-h" => {
                print_headless_help();
                process::exit(0);
            }
            _ => {}
        }
    }

    Ok(HeadlessArgs {
        plan_path,
        save_path,
        print,
    })
}

fn print_headless_help() {
    println!(
        "Headless mode options:\n  --headless | -headless\n  --plan <path>\n  --save <path>\n  --print"
    );
}

fn load_headless_plan(path: &Path) -> Result<HeadlessPlan, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    serde_json::from_slice(&data).map_err(|err| err.to_string())
}

fn default_headless_plan() -> HeadlessPlan {
    HeadlessPlan {
        nodes: vec![
            PlanNode {
                name: "Box".to_string(),
                category: "Source".to_string(),
                inputs: Vec::new(),
                outputs: vec![PlanPin {
                    name: "mesh".to_string(),
                    pin_type: grapho_core::PinType::Mesh,
                }],
            },
            PlanNode {
                name: "Output".to_string(),
                category: "Output".to_string(),
                inputs: vec![PlanPin {
                    name: "in".to_string(),
                    pin_type: grapho_core::PinType::Mesh,
                }],
                outputs: Vec::new(),
            },
        ],
        links: vec![PlanLink {
            from: PlanEndpoint {
                node: "Box".to_string(),
                pin: "mesh".to_string(),
            },
            to: PlanEndpoint {
                node: "Output".to_string(),
                pin: "in".to_string(),
            },
        }],
        output_node: Some("Output".to_string()),
    }
}

fn build_project_from_plan(plan: &HeadlessPlan) -> Result<Project, String> {
    let mut project = Project::default();
    let mut name_to_id = std::collections::HashMap::new();

    for node in &plan.nodes {
        let node_id = project.graph.add_node(grapho_core::NodeDefinition {
            name: node.name.clone(),
            category: node.category.clone(),
            inputs: node
                .inputs
                .iter()
                .map(|pin| grapho_core::PinDefinition {
                    name: pin.name.clone(),
                    pin_type: pin.pin_type,
                })
                .collect(),
            outputs: node
                .outputs
                .iter()
                .map(|pin| grapho_core::PinDefinition {
                    name: pin.name.clone(),
                    pin_type: pin.pin_type,
                })
                .collect(),
        });
        name_to_id.insert(node.name.clone(), node_id);
    }

    for link in &plan.links {
        let from_node = name_to_id
            .get(&link.from.node)
            .ok_or_else(|| format!("unknown node {}", link.from.node))?;
        let to_node = name_to_id
            .get(&link.to.node)
            .ok_or_else(|| format!("unknown node {}", link.to.node))?;

        let from_pin = find_pin_id(
            &project.graph,
            *from_node,
            &link.from.pin,
            grapho_core::PinKind::Output,
        )
        .ok_or_else(|| format!("unknown output pin {}", link.from.pin))?;
        let to_pin = find_pin_id(
            &project.graph,
            *to_node,
            &link.to.pin,
            grapho_core::PinKind::Input,
        )
        .ok_or_else(|| format!("unknown input pin {}", link.to.pin))?;

        project
            .graph
            .add_link(from_pin, to_pin)
            .map_err(|err| format!("link error: {:?}", err))?;
    }

    Ok(project)
}

fn find_pin_id(
    graph: &grapho_core::Graph,
    node_id: grapho_core::NodeId,
    pin_name: &str,
    kind: grapho_core::PinKind,
) -> Option<grapho_core::PinId> {
    let node = graph.node(node_id)?;
    let pins = match kind {
        grapho_core::PinKind::Input => &node.inputs,
        grapho_core::PinKind::Output => &node.outputs,
    };

    pins.iter().copied().find(|pin_id| {
        graph
            .pin(*pin_id)
            .map(|pin| pin.name == pin_name)
            .unwrap_or(false)
    })
}

fn save_project_json(project: &Project, path: &Path) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(project).map_err(|err| err.to_string())?;
    std::fs::write(path, data).map_err(|err| err.to_string())
}

fn validate_topo_sort(project: &Project, output_node_name: &str) -> Result<(), String> {
    let node_id = project
        .graph
        .nodes()
        .find(|node| node.name == output_node_name)
        .map(|node| node.id)
        .ok_or_else(|| format!("output node {} not found", output_node_name))?;

    let order = project
        .graph
        .topo_sort_from(node_id)
        .map_err(|err| format!("topo sort failed: {:?}", err))?;
    tracing::info!("headless: topo order {:?}", order);
    Ok(())
}

fn default_category() -> String {
    "Default".to_string()
}
